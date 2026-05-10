use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::orchestrator::OrchestratorProvider;
use super::types::{ClusterInfo, ClusterStatus, CreateClusterOptions, NodeInfo, NodeRole};
use crate::config::Config;

pub struct RusternetesOrchestrator {
    config: Config,
    data_dir: PathBuf,
    socket_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessEntry {
    component: String,
    pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContainerEntry {
    component: String,
    container_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ClusterMode {
    Single,
    Multi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClusterManifest {
    name: String,
    mode: ClusterMode,
    workers: u32,
    api_port: u16,
    etcd_client_port: Option<u16>,
    etcd_peer_port: Option<u16>,
    /// Single-node: PID-based process tracking
    #[serde(default)]
    processes: Vec<ProcessEntry>,
    /// Multi-node: container-based tracking
    #[serde(default)]
    containers: Vec<ContainerEntry>,
    /// Multi-node: per-cluster container network name
    #[serde(skip_serializing_if = "Option::is_none")]
    network_name: Option<String>,
    kubeconfig_path: PathBuf,
    created: String,
}

struct RunContainerArgs<'a> {
    name: &'a str,
    image: &'a str,
    network: &'a str,
    publish_ports: &'a [String],
    env: &'a [(&'a str, &'a str)],
    volumes: &'a [(&'a str, &'a str)],
    cap_add: &'a [&'a str],
    cmd_args: &'a [&'a str],
}

impl RusternetesOrchestrator {
    pub fn new(config: &Config) -> Self {
        let data_dir = config
            .rusternetes
            .data_dir
            .clone()
            .unwrap_or_else(|| config.cluster.data_dir.join("rusternetes"));

        let socket_path = config.socktainer.socket_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".socktainer")
                .join("container.sock")
        });

        Self {
            config: config.clone(),
            data_dir,
            socket_path,
        }
    }

    pub fn cluster_dir(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }

    fn manifest_path(&self, name: &str) -> PathBuf {
        self.cluster_dir(name).join("kina-manifest.json")
    }

    fn logs_dir(&self, name: &str) -> PathBuf {
        self.cluster_dir(name).join("logs")
    }

    pub fn cluster_manifest_exists(&self, name: &str) -> bool {
        self.manifest_path(name).exists()
    }

    fn rusternetes_binary(&self) -> String {
        self.config
            .rusternetes
            .binary_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "rusternetes".to_string())
    }

    fn container_name(cluster: &str, component: &str) -> String {
        format!("kina-{}-{}", cluster, component)
    }

    fn network_name(cluster: &str) -> String {
        format!("kina-{}-net", cluster)
    }

    fn rusternetes_image_tag(component: &str) -> String {
        format!("kina-rusternetes-{}", component)
    }

    fn read_manifest(&self, name: &str) -> Result<ClusterManifest> {
        let path = self.manifest_path(name);
        let content = fs::read_to_string(&path).with_context(|| {
            format!("Failed to read rusternetes manifest for cluster '{}'", name)
        })?;
        serde_json::from_str(&content).with_context(|| {
            format!(
                "Failed to parse rusternetes manifest for cluster '{}'",
                name
            )
        })
    }

    fn write_manifest(&self, manifest: &ClusterManifest) -> Result<()> {
        let path = self.manifest_path(&manifest.name);
        let content = serde_json::to_string_pretty(manifest)?;
        fs::write(&path, content).with_context(|| {
            format!(
                "Failed to write rusternetes manifest for cluster '{}'",
                manifest.name
            )
        })
    }

    fn is_pid_alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    fn send_signal(pid: u32, signal: libc::c_int) -> Result<()> {
        let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "Failed to send signal {} to PID {}: {}",
                signal,
                pid,
                err
            ));
        }
        Ok(())
    }

    fn open_log_file(&self, cluster_name: &str, stem: &str) -> Result<File> {
        let dir = self.logs_dir(cluster_name);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.log", stem));
        File::create(&path)
            .with_context(|| format!("Failed to create log file: {}", path.display()))
    }

    fn spawn_detached(
        &self,
        cluster_name: &str,
        component: &str,
        binary: &str,
        args: &[&str],
        extra_env: &[(&str, &str)],
    ) -> Result<u32> {
        let stdout_file = self.open_log_file(cluster_name, component)?;
        let stderr_file = self.open_log_file(cluster_name, &format!("{}.err", component))?;

        let mut cmd = Command::new(binary);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .env(
                "DOCKER_HOST",
                format!("unix://{}", self.socket_path.display()),
            );

        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn {} (binary: {})", component, binary))?;

        let pid = child.id();
        drop(child);

        info!("Spawned {} with PID {}", component, pid);
        Ok(pid)
    }

    async fn wait_for_port(host: &str, port: u16, timeout_secs: u64) -> Result<()> {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(anyhow::anyhow!(
                    "Timeout ({}s) waiting for port {} to open",
                    timeout_secs,
                    port
                ));
            }
            if tokio::net::TcpStream::connect((host, port)).await.is_ok() {
                debug!("Port {} is open", port);
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn wait_for_file(path: &Path, timeout_secs: u64) -> Result<()> {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(anyhow::anyhow!(
                    "Timeout ({}s) waiting for file: {}",
                    timeout_secs,
                    path.display()
                ));
            }
            if path.exists() {
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    fn check_binary_available(binary: &str) -> bool {
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                if Path::new(dir).join(binary).is_file() {
                    return true;
                }
            }
        }
        Path::new(binary).is_absolute() && Path::new(binary).is_file()
    }

    fn ensure_socktainer_launched(&self) -> Result<()> {
        if self.socket_path.exists() {
            debug!(
                "Socktainer socket already present at {}",
                self.socket_path.display()
            );
            return Ok(());
        }

        let socktainer_bin = self
            .config
            .socktainer
            .binary_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "socktainer".to_string());

        if !Self::check_binary_available(&socktainer_bin) {
            return Err(anyhow::anyhow!(
                "socktainer not found. Install it with:\n  brew tap socktainer/tap\n  brew install socktainer"
            ));
        }

        info!("Starting socktainer daemon...");
        fs::create_dir_all(&self.data_dir)?;

        let stdout = File::create(self.data_dir.join("socktainer.log"))?;
        let stderr = File::create(self.data_dir.join("socktainer.err.log"))?;

        let child = Command::new(&socktainer_bin)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .context("Failed to start socktainer")?;
        drop(child);

        Ok(())
    }

    async fn ensure_socktainer_ready(&self) -> Result<()> {
        self.ensure_socktainer_launched()?;

        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for socktainer socket at {}",
                    self.socket_path.display()
                ));
            }
            if self.socket_path.exists() {
                info!("Socktainer ready at {}", self.socket_path.display());
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    // --- Apple Container helpers (multi-node) ---

    async fn run_apple_container_cmd(args: &[&str]) -> Result<String> {
        let output = tokio::process::Command::new("container")
            .args(args)
            .output()
            .await
            .with_context(|| format!("Failed to run: container {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "container {} failed: {}",
                args.join(" "),
                stderr.trim()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn create_network(name: &str) -> Result<()> {
        match Self::run_apple_container_cmd(&["network", "create", name]).await {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("already exists") || msg.contains("exists") {
                    debug!("Network {} already exists, continuing", name);
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn delete_network(name: &str) -> Result<()> {
        if let Err(e) = Self::run_apple_container_cmd(&["network", "delete", name]).await {
            debug!("Network delete {} (may not exist): {}", name, e);
        }
        Ok(())
    }

    async fn start_container(args: &RunContainerArgs<'_>) -> Result<()> {
        let mut cmd_args: Vec<String> = vec![
            "run".to_string(),
            "--detach".to_string(),
            "--name".to_string(),
            args.name.to_string(),
            "--network".to_string(),
            args.network.to_string(),
        ];

        for port in args.publish_ports {
            cmd_args.push("--publish".to_string());
            cmd_args.push(port.clone());
        }

        for (k, v) in args.env {
            cmd_args.push("--env".to_string());
            cmd_args.push(format!("{}={}", k, v));
        }

        for (host, container) in args.volumes {
            cmd_args.push("--volume".to_string());
            cmd_args.push(format!("{}:{}", host, container));
        }

        for cap in args.cap_add {
            cmd_args.push("--cap-add".to_string());
            cmd_args.push(cap.to_string());
        }

        cmd_args.push(args.image.to_string());

        for arg in args.cmd_args {
            cmd_args.push(arg.to_string());
        }

        let refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
        Self::run_apple_container_cmd(&refs)
            .await
            .with_context(|| format!("Failed to start container '{}'", args.name))?;
        Ok(())
    }

    async fn stop_container(name: &str) -> Result<()> {
        if let Err(e) = Self::run_apple_container_cmd(&["stop", name]).await {
            debug!("Container stop {} (may already be stopped): {}", name, e);
        }
        Ok(())
    }

    async fn delete_container(name: &str) -> Result<()> {
        if let Err(e) = Self::run_apple_container_cmd(&["delete", name]).await {
            debug!("Container delete {} (may not exist): {}", name, e);
        }
        Ok(())
    }

    async fn get_container_ip(container_name: &str) -> Result<String> {
        let output = Self::run_apple_container_cmd(&["inspect", container_name])
            .await
            .with_context(|| format!("Failed to inspect container '{}'", container_name))?;

        let data: serde_json::Value = serde_json::from_str(&output)
            .with_context(|| format!("Failed to parse inspect JSON for '{}'", container_name))?;

        // Inspect returns an array; networks[0].ipv4Address is "IP/cidr"
        let ip_cidr = data[0]["networks"][0]["ipv4Address"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("No IPv4 address found for container '{}'", container_name)
            })?;

        let ip = ip_cidr.split('/').next().ok_or_else(|| {
            anyhow::anyhow!(
                "Unexpected IP format '{}' for '{}'",
                ip_cidr,
                container_name
            )
        })?;

        Ok(ip.to_string())
    }

    async fn is_container_running(name: &str) -> bool {
        // container list (without --all) only shows running containers
        match tokio::process::Command::new("container")
            .args(["list", "--quiet"])
            .output()
            .await
        {
            Ok(out) if out.status.success() => {
                let output = String::from_utf8_lossy(&out.stdout);
                output.lines().any(|line| line.trim() == name)
            }
            _ => false,
        }
    }

    fn generate_kubeconfig_content(cluster_name: &str, api_port: u16) -> String {
        format!(
            r#"apiVersion: v1
kind: Config
clusters:
- cluster:
    server: http://127.0.0.1:{port}
  name: {name}
contexts:
- context:
    cluster: {name}
    user: admin
  name: {name}
current-context: {name}
users:
- name: admin
  user: {{}}
"#,
            port = api_port,
            name = cluster_name,
        )
    }

    // --- Single-node path ---

    async fn create_single_node(
        &self,
        options: &CreateClusterOptions,
        cluster_dir: &Path,
    ) -> Result<ClusterManifest> {
        let api_port = self.config.rusternetes.port;
        let binary = self.rusternetes_binary();

        if !Self::check_binary_available(&binary) {
            return Err(anyhow::anyhow!(
                "rusternetes binary '{}' not found. Build it from source:\n\
                 git clone https://github.com/calfonso/rusternetes\n\
                 cd rusternetes && cargo build --release",
                binary
            ));
        }

        let data_dir = cluster_dir.join("data");
        let data_dir_str = data_dir.to_string_lossy().to_string();
        let port_str = api_port.to_string();

        fs::create_dir_all(&data_dir)?;

        info!("Spawning rusternetes all-in-one on port {}...", api_port);
        let pid = self.spawn_detached(
            &options.name,
            "rusternetes",
            &binary,
            &["server", "--data-dir", &data_dir_str, "--port", &port_str],
            &[],
        )?;

        info!("Waiting for rusternetes API server on port {}...", api_port);
        Self::wait_for_port("127.0.0.1", api_port, 60)
            .await
            .context("rusternetes API server did not start in time")?;

        let kubeconfig_src = data_dir.join("admin.kubeconfig");
        info!("Waiting for kubeconfig at {}...", kubeconfig_src.display());
        Self::wait_for_file(&kubeconfig_src, 30)
            .await
            .context("rusternetes did not generate a kubeconfig in time")?;

        let kubeconfig_dest = self
            .config
            .kubernetes
            .kubeconfig_dir
            .join(format!("{}.yaml", options.name));
        fs::create_dir_all(kubeconfig_dest.parent().unwrap())?;
        fs::copy(&kubeconfig_src, &kubeconfig_dest).context("Failed to copy kubeconfig")?;
        info!("Kubeconfig written to {}", kubeconfig_dest.display());

        Ok(ClusterManifest {
            name: options.name.clone(),
            mode: ClusterMode::Single,
            workers: 0,
            api_port,
            etcd_client_port: None,
            etcd_peer_port: None,
            processes: vec![ProcessEntry {
                component: "rusternetes".to_string(),
                pid,
            }],
            containers: vec![],
            network_name: None,
            kubeconfig_path: kubeconfig_dest,
            created: Utc::now().to_rfc3339(),
        })
    }

    // --- Multi-node path (container-based) ---

    async fn create_multi_node(
        &self,
        options: &CreateClusterOptions,
        workers: u32,
        cluster_dir: &Path,
    ) -> Result<ClusterManifest> {
        let api_port = self.config.rusternetes.port;
        let etcd_client_port = self.config.rusternetes.etcd_client_port;

        // Verify component images exist — built separately via `mise run rusternetes:build`
        let missing: Vec<&str> = futures::future::join_all(
            [
                "api-server",
                "scheduler",
                "controller-manager",
                "kubelet",
                "kube-proxy",
            ]
            .iter()
            .map(|c| async move {
                let tag = Self::rusternetes_image_tag(c);
                let exists = tokio::process::Command::new("container")
                    .args(["image", "inspect", &tag])
                    .output()
                    .await
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                (*c, exists)
            }),
        )
        .await
        .into_iter()
        .filter_map(|(c, exists)| if exists { None } else { Some(c) })
        .collect();

        if !missing.is_empty() {
            return Err(anyhow::anyhow!(
                "Missing rusternetes images: {}. Run `mise run rusternetes:build` first.",
                missing.join(", ")
            ));
        }

        let network = Self::network_name(&options.name);
        info!("Creating container network {}...", network);
        Self::create_network(&network).await?;

        let mut containers: Vec<ContainerEntry> = Vec::new();

        // Start etcd (accessible within the container network only)
        let etcd_cname = Self::container_name(&options.name, "etcd");
        let etcd_advertise_url = format!("http://{}:2379", etcd_cname);
        info!("Starting etcd container {}...", etcd_cname);
        let etcd_peer_url = format!("http://{}:2380", etcd_cname);
        let etcd_initial_cluster = format!("default={}", etcd_peer_url);
        Self::start_container(&RunContainerArgs {
            name: &etcd_cname,
            image: "quay.io/coreos/etcd:v3.5.17",
            network: &network,
            publish_ports: &[format!("{}:2379", etcd_client_port)],
            env: &[("ETCDCTL_API", "3")],
            volumes: &[],
            cap_add: &[],
            cmd_args: &[
                "/usr/local/bin/etcd",
                "--data-dir",
                "/etcd-data",
                "--listen-client-urls",
                "http://0.0.0.0:2379",
                "--advertise-client-urls",
                &etcd_advertise_url,
                "--listen-peer-urls",
                "http://0.0.0.0:2380",
                "--initial-advertise-peer-urls",
                &etcd_peer_url,
                "--initial-cluster",
                &etcd_initial_cluster,
            ],
        })
        .await?;
        containers.push(ContainerEntry {
            component: "etcd".to_string(),
            container_name: etcd_cname.clone(),
        });

        // Wait for etcd to be reachable on the published host port
        info!("Waiting for etcd on port {}...", etcd_client_port);
        Self::wait_for_port("127.0.0.1", etcd_client_port, 30)
            .await
            .context("etcd container did not become ready in time")?;

        // Apple Container provides no DNS server for container networks (nameservers: []).
        // The initial etcd connection check uses getaddrinfo which falls back to /etc/hosts,
        // but tonic's async gRPC channel uses a DNS-server-only resolver that bypasses
        // /etc/hosts and therefore fails. Use the container's IP address directly.
        let etcd_ip = Self::get_container_ip(&etcd_cname)
            .await
            .with_context(|| format!("Failed to get IP for etcd container '{}'", etcd_cname))?;
        let etcd_url = format!("http://{}:2379", etcd_ip);
        info!("etcd container IP: {} -> using {}", etcd_cname, etcd_url);

        // Start api-server (HTTP, no TLS — simpler kubeconfig generation for local dev)
        let apiserver_cname = Self::container_name(&options.name, "api-server");
        let api_port_str = format!("{}:6443", api_port);
        info!(
            "Starting api-server container {} on port {}...",
            apiserver_cname, api_port
        );
        Self::start_container(&RunContainerArgs {
            name: &apiserver_cname,
            image: &Self::rusternetes_image_tag("api-server"),
            network: &network,
            publish_ports: &[api_port_str],
            env: &[("RUST_LOG", "info")],
            volumes: &[],
            cap_add: &[],
            cmd_args: &[
                "--bind-address",
                "0.0.0.0:6443",
                "--etcd-servers",
                &etcd_url,
                "--skip-auth",
                "--console-dir",
                "/app/console",
            ],
        })
        .await?;
        containers.push(ContainerEntry {
            component: "api-server".to_string(),
            container_name: apiserver_cname.clone(),
        });

        // Wait for the api-server port
        info!("Waiting for api-server on port {}...", api_port);
        Self::wait_for_port("127.0.0.1", api_port, 120)
            .await
            .context("rusternetes api-server container did not become ready in time")?;

        // Get api-server IP for the same reason as etcd — tonic bypasses /etc/hosts
        let apiserver_ip = Self::get_container_ip(&apiserver_cname)
            .await
            .with_context(|| {
                format!(
                    "Failed to get IP for api-server container '{}'",
                    apiserver_cname
                )
            })?;
        info!(
            "api-server container IP: {} -> {}",
            apiserver_cname, apiserver_ip
        );

        // Write kubeconfig pointing to localhost (HTTP, no TLS)
        let kubeconfig_dest = self
            .config
            .kubernetes
            .kubeconfig_dir
            .join(format!("{}.yaml", options.name));
        fs::create_dir_all(kubeconfig_dest.parent().unwrap())?;
        let kubeconfig_content = Self::generate_kubeconfig_content(&options.name, api_port);
        fs::write(&kubeconfig_dest, &kubeconfig_content).context("Failed to write kubeconfig")?;
        info!("Kubeconfig written to {}", kubeconfig_dest.display());

        // Start scheduler
        let scheduler_cname = Self::container_name(&options.name, "scheduler");
        info!("Starting scheduler container {}...", scheduler_cname);
        Self::start_container(&RunContainerArgs {
            name: &scheduler_cname,
            image: &Self::rusternetes_image_tag("scheduler"),
            network: &network,
            publish_ports: &[],
            env: &[("RUST_LOG", "info")],
            volumes: &[],
            cap_add: &[],
            cmd_args: &["--etcd-servers", &etcd_url, "--interval", "1"],
        })
        .await?;
        containers.push(ContainerEntry {
            component: "scheduler".to_string(),
            container_name: scheduler_cname,
        });

        // Start controller-manager
        let cm_cname = Self::container_name(&options.name, "controller-manager");
        info!("Starting controller-manager container {}...", cm_cname);
        Self::start_container(&RunContainerArgs {
            name: &cm_cname,
            image: &Self::rusternetes_image_tag("controller-manager"),
            network: &network,
            publish_ports: &[],
            env: &[("RUST_LOG", "info")],
            volumes: &[],
            cap_add: &[],
            cmd_args: &["--etcd-servers", &etcd_url, "--sync-interval", "3"],
        })
        .await?;
        containers.push(ContainerEntry {
            component: "controller-manager".to_string(),
            container_name: cm_cname,
        });

        // Start N kubelet containers
        let socket_host = self.socket_path.to_string_lossy().to_string();
        for i in 0..workers {
            let node_num = i + 1;
            let node_name = format!("node-{}", node_num);
            let kubelet_cname =
                Self::container_name(&options.name, &format!("kubelet-{}", node_num));
            let metrics_port = (10250 + i) as u16;

            // The kubelet creates pod volume dirs here. The same path must be used on both sides
            // of the bind mount so that socktainer (running on the host) can find those paths
            // when it bind-mounts them into pod containers.
            let volumes_dir = cluster_dir
                .join("volumes")
                .join(format!("node-{}", node_num));
            fs::create_dir_all(&volumes_dir)?;
            let volumes_path = volumes_dir.to_string_lossy().to_string();

            info!(
                "Starting kubelet container {} for {}...",
                kubelet_cname, node_name
            );
            Self::start_container(&RunContainerArgs {
                name: &kubelet_cname,
                image: &Self::rusternetes_image_tag("kubelet"),
                network: &network,
                publish_ports: &[],
                env: &[
                    ("RUST_LOG", "debug"),
                    ("DOCKER_HOST", "unix:///var/run/container.sock"),
                    ("KUBERNETES_SERVICE_HOST_OVERRIDE", &apiserver_ip),
                    ("KUBELET_VOLUMES_PATH", &volumes_path),
                ],
                volumes: &[
                    (&socket_host, "/var/run/container.sock"),
                    (&volumes_path, &volumes_path),
                ],
                cap_add: &[],
                cmd_args: &[
                    "--node-name",
                    &node_name,
                    "--etcd-servers",
                    &etcd_url,
                    "--cluster-dns",
                    "10.96.0.10",
                    "--metrics-port",
                    &metrics_port.to_string(),
                    "--sync-interval",
                    "3",
                ],
            })
            .await?;
            containers.push(ContainerEntry {
                component: format!("kubelet-{}", node_num),
                container_name: kubelet_cname,
            });
        }

        // Start one kube-proxy per worker node
        for i in 0..workers {
            let node_num = i + 1;
            let node_name = format!("node-{}", node_num);
            let proxy_cname =
                Self::container_name(&options.name, &format!("kube-proxy-{}", node_num));
            info!(
                "Starting kube-proxy container {} for {}...",
                proxy_cname, node_name
            );
            Self::start_container(&RunContainerArgs {
                name: &proxy_cname,
                image: &Self::rusternetes_image_tag("kube-proxy"),
                network: &network,
                publish_ports: &[],
                env: &[("RUST_LOG", "info")],
                volumes: &[],
                cap_add: &["CAP_NET_ADMIN", "CAP_NET_RAW"],
                cmd_args: &["--node-name", &node_name, "--etcd-servers", &etcd_url],
            })
            .await?;
            containers.push(ContainerEntry {
                component: format!("kube-proxy-{}", node_num),
                container_name: proxy_cname,
            });
        }

        Ok(ClusterManifest {
            name: options.name.clone(),
            mode: ClusterMode::Multi,
            workers,
            api_port,
            etcd_client_port: Some(etcd_client_port),
            etcd_peer_port: Some(self.config.rusternetes.etcd_peer_port),
            processes: vec![],
            containers,
            network_name: Some(network),
            kubeconfig_path: kubeconfig_dest,
            created: Utc::now().to_rfc3339(),
        })
    }

    // --- Teardown ---

    async fn stop_single_node_processes(&self, manifest: &ClusterManifest) {
        for entry in &manifest.processes {
            if !Self::is_pid_alive(entry.pid) {
                debug!(
                    "Process {} (PID {}) already gone",
                    entry.component, entry.pid
                );
                continue;
            }
            info!("Stopping {} (PID {})...", entry.component, entry.pid);
            let _ = Self::send_signal(entry.pid, libc::SIGTERM);
            sleep(Duration::from_millis(800)).await;
            if Self::is_pid_alive(entry.pid) {
                warn!("Force-killing {} (PID {})", entry.component, entry.pid);
                let _ = Self::send_signal(entry.pid, libc::SIGKILL);
            }
        }
    }

    async fn stop_multi_node_containers(&self, manifest: &ClusterManifest) {
        // Ordered teardown: kube-proxies → kubelets → scheduler/cm → api-server → etcd
        let ordered_prefixes = [
            "kube-proxy-",
            "kubelet-",
            "scheduler",
            "controller-manager",
            "api-server",
            "etcd",
        ];

        for prefix in &ordered_prefixes {
            for entry in manifest
                .containers
                .iter()
                .filter(|c| c.component.starts_with(prefix))
            {
                info!("Stopping container {}...", entry.container_name);
                let _ = Self::stop_container(&entry.container_name).await;
                let _ = Self::delete_container(&entry.container_name).await;
            }
        }

        if let Some(ref net) = manifest.network_name {
            info!("Deleting container network {}...", net);
            let _ = Self::delete_network(net).await;
        }
    }

    async fn manifest_to_cluster_info(&self, manifest: &ClusterManifest) -> ClusterInfo {
        let (status, nodes) = match manifest.mode {
            ClusterMode::Single => {
                let all_alive = manifest.processes.iter().all(|p| Self::is_pid_alive(p.pid));
                let any_alive = manifest.processes.iter().any(|p| Self::is_pid_alive(p.pid));

                let status = match (all_alive, any_alive) {
                    (true, _) => ClusterStatus::Running,
                    (false, true) => ClusterStatus::Error,
                    (false, false) => ClusterStatus::Stopped,
                };

                let cp_alive = manifest
                    .processes
                    .iter()
                    .find(|p| p.component == "rusternetes")
                    .map(|p| Self::is_pid_alive(p.pid))
                    .unwrap_or(false);

                let nodes = vec![NodeInfo {
                    name: format!("{}-control-plane", manifest.name),
                    role: NodeRole::ControlPlane,
                    status: if cp_alive {
                        "Ready".to_string()
                    } else {
                        "NotReady".to_string()
                    },
                    version: String::new(),
                    container_id: None,
                    ip_address: Some("127.0.0.1".to_string()),
                }];

                (status, nodes)
            }

            ClusterMode::Multi => {
                // Check running state for each container in parallel
                let running_checks: Vec<bool> = futures::future::join_all(
                    manifest
                        .containers
                        .iter()
                        .map(|c| Self::is_container_running(&c.container_name)),
                )
                .await;

                let all_running = !running_checks.is_empty() && running_checks.iter().all(|&r| r);
                let any_running = running_checks.iter().any(|&r| r);

                let status = match (all_running, any_running) {
                    (true, _) => ClusterStatus::Running,
                    (false, true) => ClusterStatus::Error,
                    (false, false) => ClusterStatus::Stopped,
                };

                let apiserver_idx = manifest
                    .containers
                    .iter()
                    .position(|c| c.component == "api-server");
                let cp_running = apiserver_idx
                    .map(|i| running_checks.get(i).copied().unwrap_or(false))
                    .unwrap_or(false);

                let mut nodes = vec![NodeInfo {
                    name: format!("{}-control-plane", manifest.name),
                    role: NodeRole::ControlPlane,
                    status: if cp_running {
                        "Ready".to_string()
                    } else {
                        "NotReady".to_string()
                    },
                    version: String::new(),
                    container_id: apiserver_idx.map(|_| {
                        manifest.containers[apiserver_idx.unwrap()]
                            .container_name
                            .clone()
                    }),
                    ip_address: Some("127.0.0.1".to_string()),
                }];

                for i in 0..manifest.workers {
                    let node_num = i + 1;
                    let component = format!("kubelet-{}", node_num);
                    let kubelet_idx = manifest
                        .containers
                        .iter()
                        .position(|c| c.component == component);
                    let kubelet_running = kubelet_idx
                        .map(|idx| running_checks.get(idx).copied().unwrap_or(false))
                        .unwrap_or(false);
                    let container_name =
                        kubelet_idx.map(|idx| manifest.containers[idx].container_name.clone());
                    nodes.push(NodeInfo {
                        name: format!("node-{}", node_num),
                        role: NodeRole::Worker,
                        status: if kubelet_running {
                            "Ready".to_string()
                        } else {
                            "NotReady".to_string()
                        },
                        version: String::new(),
                        container_id: container_name,
                        ip_address: None,
                    });
                }

                (status, nodes)
            }
        };

        ClusterInfo {
            name: manifest.name.clone(),
            image: "rusternetes".to_string(),
            status,
            created: manifest.created.clone(),
            nodes,
            kubeconfig_path: Some(manifest.kubeconfig_path.to_string_lossy().to_string()),
        }
    }
}

#[async_trait]
impl OrchestratorProvider for RusternetesOrchestrator {
    async fn create_cluster(&self, options: &CreateClusterOptions) -> Result<PathBuf> {
        self.ensure_socktainer_ready().await?;

        let cluster_dir = self.cluster_dir(&options.name);
        fs::create_dir_all(&cluster_dir)?;

        let manifest = match options.workers {
            Some(w) if w > 0 => {
                info!(
                    "Creating multi-node rusternetes cluster '{}' with {} worker(s)",
                    options.name, w
                );
                self.create_multi_node(options, w, &cluster_dir).await?
            }
            _ => {
                info!(
                    "Creating single-node rusternetes cluster '{}'",
                    options.name
                );
                self.create_single_node(options, &cluster_dir).await?
            }
        };

        let kubeconfig_path = manifest.kubeconfig_path.clone();
        self.write_manifest(&manifest)?;
        info!("Cluster '{}' ready", options.name);

        Ok(kubeconfig_path)
    }

    async fn delete_cluster(&self, name: &str) -> Result<()> {
        let manifest = self.read_manifest(name)?;

        match manifest.mode {
            ClusterMode::Single => {
                info!("Stopping processes for cluster '{}'...", name);
                self.stop_single_node_processes(&manifest).await;
            }
            ClusterMode::Multi => {
                info!("Stopping containers for cluster '{}'...", name);
                self.stop_multi_node_containers(&manifest).await;
            }
        }

        let cluster_dir = self.cluster_dir(name);
        if cluster_dir.exists() {
            fs::remove_dir_all(&cluster_dir).with_context(|| {
                format!(
                    "Failed to remove cluster directory: {}",
                    cluster_dir.display()
                )
            })?;
        }

        if manifest.kubeconfig_path.exists() {
            let _ = fs::remove_file(&manifest.kubeconfig_path);
        }

        Ok(())
    }

    async fn get_kubeconfig_path(&self, name: &str) -> Result<PathBuf> {
        let manifest = self.read_manifest(name)?;
        Ok(manifest.kubeconfig_path)
    }

    async fn is_running(&self, name: &str) -> Result<bool> {
        let manifest = match self.read_manifest(name) {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };
        match manifest.mode {
            ClusterMode::Single => Ok(manifest.processes.iter().all(|p| Self::is_pid_alive(p.pid))),
            ClusterMode::Multi => {
                let checks: Vec<bool> = futures::future::join_all(
                    manifest
                        .containers
                        .iter()
                        .map(|c| Self::is_container_running(&c.container_name)),
                )
                .await;
                Ok(!checks.is_empty() && checks.iter().all(|&r| r))
            }
        }
    }

    async fn list_clusters(&self) -> Result<Vec<ClusterInfo>> {
        let mut clusters = Vec::new();

        if !self.data_dir.exists() {
            return Ok(clusters);
        }

        for entry in fs::read_dir(&self.data_dir)?.flatten() {
            let manifest_path = entry.path().join("kina-manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            let content = match fs::read_to_string(&manifest_path) {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        "Failed to read manifest at {}: {}",
                        manifest_path.display(),
                        e
                    );
                    continue;
                }
            };

            let manifest: ClusterManifest = match serde_json::from_str(&content) {
                Ok(m) => m,
                Err(e) => {
                    warn!(
                        "Failed to parse manifest at {}: {}",
                        manifest_path.display(),
                        e
                    );
                    continue;
                }
            };

            clusters.push(self.manifest_to_cluster_info(&manifest).await);
        }

        Ok(clusters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn make_orchestrator(temp_dir: &TempDir) -> RusternetesOrchestrator {
        let mut config = Config::default();
        config.rusternetes.data_dir = Some(temp_dir.path().to_path_buf());
        config.kubernetes.kubeconfig_dir = temp_dir.path().join("kubeconfigs");
        RusternetesOrchestrator::new(&config)
    }

    fn make_single_node_manifest(name: &str, pid: u32, kubeconfig: PathBuf) -> ClusterManifest {
        ClusterManifest {
            name: name.to_string(),
            mode: ClusterMode::Single,
            workers: 0,
            api_port: 6443,
            etcd_client_port: None,
            etcd_peer_port: None,
            processes: vec![ProcessEntry {
                component: "rusternetes".to_string(),
                pid,
            }],
            containers: vec![],
            network_name: None,
            kubeconfig_path: kubeconfig,
            created: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_multi_node_manifest(name: &str, workers: u32, kubeconfig: PathBuf) -> ClusterManifest {
        let mut containers = vec![
            ContainerEntry {
                component: "etcd".to_string(),
                container_name: format!("kina-{}-etcd", name),
            },
            ContainerEntry {
                component: "api-server".to_string(),
                container_name: format!("kina-{}-api-server", name),
            },
            ContainerEntry {
                component: "scheduler".to_string(),
                container_name: format!("kina-{}-scheduler", name),
            },
            ContainerEntry {
                component: "controller-manager".to_string(),
                container_name: format!("kina-{}-controller-manager", name),
            },
        ];
        for i in 0..workers {
            containers.push(ContainerEntry {
                component: format!("kubelet-{}", i + 1),
                container_name: format!("kina-{}-kubelet-{}", name, i + 1),
            });
        }
        for i in 0..workers {
            containers.push(ContainerEntry {
                component: format!("kube-proxy-{}", i + 1),
                container_name: format!("kina-{}-kube-proxy-{}", name, i + 1),
            });
        }
        ClusterManifest {
            name: name.to_string(),
            mode: ClusterMode::Multi,
            workers,
            api_port: 6443,
            etcd_client_port: Some(2379),
            etcd_peer_port: Some(2380),
            processes: vec![],
            containers,
            network_name: Some(format!("kina-{}-net", name)),
            kubeconfig_path: kubeconfig,
            created: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn get_dead_pid() -> u32 {
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn true");
        let pid = child.id();
        child.wait().expect("wait true");
        pid
    }

    // --- is_pid_alive ---

    #[test]
    fn test_is_pid_alive_current_process() {
        assert!(RusternetesOrchestrator::is_pid_alive(std::process::id()));
    }

    #[test]
    fn test_is_pid_alive_dead_process() {
        let dead = get_dead_pid();
        assert!(!RusternetesOrchestrator::is_pid_alive(dead));
    }

    // --- check_binary_available ---

    #[test]
    fn test_check_binary_available_sh() {
        assert!(RusternetesOrchestrator::check_binary_available("sh"));
    }

    #[test]
    fn test_check_binary_available_missing() {
        assert!(!RusternetesOrchestrator::check_binary_available(
            "definitely-not-a-real-binary-kina-xyz123"
        ));
    }

    // --- container / network naming ---

    #[test]
    fn test_container_name() {
        assert_eq!(
            RusternetesOrchestrator::container_name("mycluster", "etcd"),
            "kina-mycluster-etcd"
        );
        assert_eq!(
            RusternetesOrchestrator::container_name("test", "kubelet-1"),
            "kina-test-kubelet-1"
        );
    }

    #[test]
    fn test_network_name() {
        assert_eq!(
            RusternetesOrchestrator::network_name("mycluster"),
            "kina-mycluster-net"
        );
    }

    #[test]
    fn test_rusternetes_image_tag() {
        assert_eq!(
            RusternetesOrchestrator::rusternetes_image_tag("api-server"),
            "kina-rusternetes-api-server"
        );
    }

    // --- generate_kubeconfig_content ---

    #[test]
    fn test_generate_kubeconfig_content() {
        let content = RusternetesOrchestrator::generate_kubeconfig_content("test-cluster", 6443);
        assert!(content.contains("server: http://127.0.0.1:6443"));
        assert!(content.contains("name: test-cluster"));
        assert!(content.contains("current-context: test-cluster"));
        assert!(content.contains("user: admin"));
    }

    #[test]
    fn test_generate_kubeconfig_content_custom_port() {
        let content = RusternetesOrchestrator::generate_kubeconfig_content("my-cluster", 7443);
        assert!(content.contains("server: http://127.0.0.1:7443"));
        assert!(content.contains("name: my-cluster"));
    }

    // --- cluster_manifest_exists ---

    #[test]
    fn test_cluster_manifest_not_exists_no_dir() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        assert!(!orch.cluster_manifest_exists("nonexistent"));
    }

    #[test]
    fn test_cluster_manifest_exists_after_write() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest = make_single_node_manifest(
            "test-cluster",
            std::process::id(),
            temp.path().join("kubeconfigs").join("test-cluster.yaml"),
        );
        fs::create_dir_all(orch.cluster_dir("test-cluster")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        assert!(orch.cluster_manifest_exists("test-cluster"));
        assert!(!orch.cluster_manifest_exists("other-cluster"));
    }

    // --- manifest roundtrip ---

    #[test]
    fn test_single_node_manifest_roundtrip() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let original = make_single_node_manifest(
            "roundtrip",
            std::process::id(),
            temp.path().join("roundtrip.yaml"),
        );
        fs::create_dir_all(orch.cluster_dir("roundtrip")).unwrap();
        orch.write_manifest(&original).unwrap();

        let loaded = orch.read_manifest("roundtrip").unwrap();
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.mode, original.mode);
        assert_eq!(loaded.workers, original.workers);
        assert_eq!(loaded.processes.len(), 1);
        assert!(loaded.containers.is_empty());
        assert!(loaded.network_name.is_none());
    }

    #[test]
    fn test_multi_node_manifest_roundtrip() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let original = make_multi_node_manifest("multi-rt", 2, temp.path().join("multi-rt.yaml"));
        fs::create_dir_all(orch.cluster_dir("multi-rt")).unwrap();
        orch.write_manifest(&original).unwrap();

        let loaded = orch.read_manifest("multi-rt").unwrap();
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.mode, ClusterMode::Multi);
        assert_eq!(loaded.workers, 2);
        assert!(loaded.processes.is_empty());
        // 4 control-plane + 2 kubelet + 2 kube-proxy containers
        assert_eq!(loaded.containers.len(), 8);
        assert_eq!(loaded.network_name, Some("kina-multi-rt-net".to_string()));
        assert_eq!(loaded.etcd_client_port, Some(2379));
        assert!(loaded
            .containers
            .iter()
            .any(|c| c.container_name == "kina-multi-rt-api-server"));
        assert!(loaded
            .containers
            .iter()
            .any(|c| c.container_name == "kina-multi-rt-kubelet-1"));
        assert!(loaded
            .containers
            .iter()
            .any(|c| c.container_name == "kina-multi-rt-kubelet-2"));
        assert!(loaded
            .containers
            .iter()
            .any(|c| c.container_name == "kina-multi-rt-kube-proxy-1"));
        assert!(loaded
            .containers
            .iter()
            .any(|c| c.container_name == "kina-multi-rt-kube-proxy-2"));
    }

    // --- list_clusters ---

    #[tokio::test]
    async fn test_list_clusters_no_data_dir() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        let clusters = orch.list_clusters().await.unwrap();
        assert!(clusters.is_empty());
    }

    #[tokio::test]
    async fn test_list_clusters_single_node_running() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest = make_single_node_manifest(
            "my-cluster",
            std::process::id(),
            temp.path().join("kubeconfigs").join("my-cluster.yaml"),
        );
        fs::create_dir_all(orch.cluster_dir("my-cluster")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        assert_eq!(clusters.len(), 1);

        let c = &clusters[0];
        assert_eq!(c.name, "my-cluster");
        assert_eq!(c.image, "rusternetes");
        assert_eq!(c.status, ClusterStatus::Running);
        assert_eq!(c.nodes.len(), 1);
        assert_eq!(c.nodes[0].role, NodeRole::ControlPlane);
        assert_eq!(c.nodes[0].status, "Ready");
    }

    #[tokio::test]
    async fn test_list_clusters_multi_node_manifest_loads() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest = make_multi_node_manifest("multi", 2, temp.path().join("multi.yaml"));
        fs::create_dir_all(orch.cluster_dir("multi")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        assert_eq!(clusters.len(), 1);

        let c = &clusters[0];
        assert_eq!(c.name, "multi");
        assert_eq!(c.image, "rusternetes");
        // Containers are not actually running in tests, so status is Stopped
        assert_eq!(c.status, ClusterStatus::Stopped);
        // 1 control-plane + 2 workers
        assert_eq!(c.nodes.len(), 3);
        assert_eq!(c.nodes[0].role, NodeRole::ControlPlane);
        assert_eq!(c.nodes[1].role, NodeRole::Worker);
        assert_eq!(c.nodes[2].role, NodeRole::Worker);
    }

    #[tokio::test]
    async fn test_list_clusters_stopped_single_node() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        let dead_pid = get_dead_pid();

        let manifest =
            make_single_node_manifest("stopped", dead_pid, temp.path().join("stopped.yaml"));
        fs::create_dir_all(orch.cluster_dir("stopped")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].status, ClusterStatus::Stopped);
        assert_eq!(clusters[0].nodes[0].status, "NotReady");
    }

    #[tokio::test]
    async fn test_list_clusters_skips_dir_without_manifest() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        fs::create_dir_all(orch.cluster_dir("no-manifest")).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        assert!(clusters.is_empty());
    }

    // --- is_running ---

    #[tokio::test]
    async fn test_is_running_nonexistent_cluster() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        assert!(!orch.is_running("does-not-exist").await.unwrap());
    }

    #[tokio::test]
    async fn test_is_running_live_single_node_process() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest =
            make_single_node_manifest("live", std::process::id(), temp.path().join("live.yaml"));
        fs::create_dir_all(orch.cluster_dir("live")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        assert!(orch.is_running("live").await.unwrap());
    }

    #[tokio::test]
    async fn test_is_running_dead_single_node_process() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        let dead_pid = get_dead_pid();

        let manifest = make_single_node_manifest("dead", dead_pid, temp.path().join("dead.yaml"));
        fs::create_dir_all(orch.cluster_dir("dead")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        assert!(!orch.is_running("dead").await.unwrap());
    }

    #[tokio::test]
    async fn test_is_running_multi_node_no_containers_running() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest =
            make_multi_node_manifest("multi-stopped", 1, temp.path().join("multi-stopped.yaml"));
        fs::create_dir_all(orch.cluster_dir("multi-stopped")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        assert!(!orch.is_running("multi-stopped").await.unwrap());
    }

    // --- generate_kubeconfig_content is valid YAML ---

    #[test]
    fn test_generate_kubeconfig_is_valid_yaml() {
        let content = RusternetesOrchestrator::generate_kubeconfig_content("my-cluster", 6443);
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(&content).expect("generated kubeconfig should be valid YAML");
        assert_eq!(parsed["apiVersion"], "v1");
        assert_eq!(parsed["kind"], "Config");
        assert_eq!(parsed["current-context"], "my-cluster");
        assert_eq!(
            parsed["clusters"][0]["cluster"]["server"],
            "http://127.0.0.1:6443"
        );
    }

    // --- multi-node node structure with kubelet containers ---

    #[tokio::test]
    async fn test_list_clusters_multi_node_worker_container_ids() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let manifest =
            make_multi_node_manifest("multi-full", 2, temp.path().join("multi-full.yaml"));
        fs::create_dir_all(orch.cluster_dir("multi-full")).unwrap();
        orch.write_manifest(&manifest).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        let c = &clusters[0];

        // Worker nodes should have their container names as container_id
        let worker1 = c.nodes.iter().find(|n| n.name == "node-1").unwrap();
        assert_eq!(worker1.role, NodeRole::Worker);
        assert_eq!(
            worker1.container_id,
            Some("kina-multi-full-kubelet-1".to_string())
        );

        let worker2 = c.nodes.iter().find(|n| n.name == "node-2").unwrap();
        assert_eq!(
            worker2.container_id,
            Some("kina-multi-full-kubelet-2".to_string())
        );
    }

    // --- listing multiple clusters ---

    #[tokio::test]
    async fn test_list_clusters_mixed_single_and_multi() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);

        let single = make_single_node_manifest(
            "single-one",
            std::process::id(),
            temp.path().join("single-one.yaml"),
        );
        fs::create_dir_all(orch.cluster_dir("single-one")).unwrap();
        orch.write_manifest(&single).unwrap();

        let multi = make_multi_node_manifest("multi-two", 1, temp.path().join("multi-two.yaml"));
        fs::create_dir_all(orch.cluster_dir("multi-two")).unwrap();
        orch.write_manifest(&multi).unwrap();

        let clusters = orch.list_clusters().await.unwrap();
        assert_eq!(clusters.len(), 2);

        let names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"single-one"));
        assert!(names.contains(&"multi-two"));

        let single_info = clusters.iter().find(|c| c.name == "single-one").unwrap();
        assert_eq!(single_info.status, ClusterStatus::Running);
        assert_eq!(single_info.nodes.len(), 1);

        let multi_info = clusters.iter().find(|c| c.name == "multi-two").unwrap();
        assert_eq!(multi_info.status, ClusterStatus::Stopped);
        assert_eq!(multi_info.nodes.len(), 2); // 1 control-plane + 1 worker
    }

    // --- read_manifest error on missing file ---

    #[test]
    fn test_read_manifest_missing_file_returns_error() {
        let temp = TempDir::new().unwrap();
        let orch = make_orchestrator(&temp);
        let result = orch.read_manifest("no-such-cluster");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no-such-cluster"));
    }
}
