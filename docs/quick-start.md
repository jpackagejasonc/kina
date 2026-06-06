# Quick Start

## Create Your First Cluster

```bash
# Create a cluster with default settings
kina create my-cluster

# kina writes an individual kubeconfig at ~/.kube/my-cluster and merges it into ~/.kube/config.
# If you prefer to point kubectl at the individual file:
export KUBECONFIG=~/.kube/my-cluster

# Verify cluster is working
kubectl get nodes
```

**Advanced Options:**
```bash
# Create cluster with explicit CNI selection and wait for readiness
kina create demo --cni ptp --wait 300

# Create one control-plane node plus two workers
kina create demo-multi --workers 2 --wait 300
```

## Install Traefik (Gateway API)

```bash
# Install Traefik gateway controller to your cluster
kina install traefik --cluster my-cluster
```

This installs the Gateway API CRDs (v1.5.1, standard channel), the Traefik
DaemonSet, a `traefik` `GatewayClass`, and a shared `Gateway` named `traefik`
(listening on :80 and :443) in the `traefik` namespace. Apps in any namespace
can attach `HTTPRoute`s to it.

## Check Cluster Status

```bash
# Basic status
kina status my-cluster

# Detailed status with pods and services
kina status my-cluster --verbose
```

## Integration Test Cluster

**Option A: Using mise (if installed)**
```bash
# Create an integration test cluster with Traefik and demo app
mise run test:cluster

# Validate the most recent test cluster
mise run test:cluster:validate

# Clean up all test clusters (removes clusters with 'demo-' prefix)
mise run test:cluster:cleanup
```

**Option B: Manual setup (without mise)**
```bash
# Create cluster with Traefik
kina create demo-cluster --wait 300
kina install traefik --cluster demo-cluster

# Check status
kina status demo-cluster --verbose
```

The demo cluster setup creates:
- A timestamped cluster (e.g., `demo-20241228-143022`)
- Traefik gateway controller installation and configuration
- A sample web application with 2 replicas
- Gateway API HTTPRoute for browser/curl access
- Apple Container DNS and networking discovery for local access

## Verify Your Setup

After creating your first cluster, verify everything works:

```bash
# Check cluster status
kina status my-cluster

# List all pods (should show running status)
kubectl --kubeconfig ~/.kube/my-cluster get pods -A

# Verify nodes are ready
kubectl --kubeconfig ~/.kube/my-cluster get nodes
```

**Troubleshooting**: If cluster creation fails, check:
- Apple Container CLI is available: `container --version`
- Sufficient system resources (2GB+ RAM recommended)
- Try with `--retain` to keep failed containers for debugging: `kina create test-cluster --retain`
