# Configuration

## Configuration File Location

kina uses a TOML configuration file located at:
```
~/.config/kina/config.toml
```

## Default Configuration

```toml
[cluster]
default_name = "kina"
default_image = "kina/node:v1.36.1"
default_wait_timeout = 300
data_dir = "~/.local/share/kina"
retain_on_failure = false
default_cni = "ptp"

[apple_container]
cli_path = null  # Auto-detected

[apple_container.runtime_config]
cpu_limit = null
memory_limit = "2Gi"
storage_limit = "20Gi"

[apple_container.network]
network_name = "kina"
enable_ipv6 = false
dns_servers = []

[kubernetes]
default_version = "v1.36.1"
kubectl_path = null  # Auto-detected
default_namespace = "default"
kubeconfig_dir = "~/.config/kina/kubeconfig"

[logging]
level = "info"
format = "text"
file_logging = false
log_dir = null
```

## Environment Variables

```bash
export RUST_LOG="info"
export RUST_BACKTRACE="1"
```

Cluster creation also writes kubeconfig files to `~/.kube/<cluster-name>` and merges them into `~/.kube/config` for normal `kubectl` use. The `kubernetes.kubeconfig_dir` setting is part of the stored configuration schema and is not the primary location used by created clusters.
