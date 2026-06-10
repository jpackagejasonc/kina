# Command Reference

## Cluster Management

```bash
# Create a new cluster
kina create [NAME] [OPTIONS]
  --image TEXT           Container image (default: kina/node:v1.36.1)
  --config FILE          Cluster configuration file
  --wait SECONDS         Wait for cluster readiness
  --retain               Retain cluster on failure
  --skip-csr-approval    Skip automatic kubelet CSR approval
  --workers COUNT        Number of worker nodes (default: 0)
  --cni ptp              CNI plugin (default: ptp)

# Delete a cluster
kina delete [NAME]
kina delete --all      # Delete all clusters

# List clusters
kina list              # Simple list
kina ls                # Alias for list
kina list --verbose    # Detailed information

# Show cluster status
kina status [NAME] [OPTIONS]
  --verbose              Show detailed information
  --output table|yaml|json
```

## Resource Operations

```bash
# Get cluster information
kina get clusters [NAME]
kina get kubeconfig [NAME]
kina get nodes [NAME]

# Load container images
kina load IMAGE --cluster NAME

# Export configurations
kina export [NAME] [OPTIONS]      # --format config is accepted but not implemented yet
  --format kubeconfig|config      # default: kubeconfig
  --output FILE                   # short form: -o
```

## Addon Management

```bash
# Install addons
kina install traefik --cluster NAME
kina install metrics-server --cluster NAME

# Accepted install options
kina install ADDON --cluster NAME --version TEXT
kina install ADDON --cluster NAME --config FILE
```

The `install --version` and `install --config` options are accepted by the CLI. The current built-in installers use the bundled manifests in this repository.

## Cluster Operations

```bash
# Approve kubelet Certificate Signing Requests
kina approve-csr [NAME]

# Configuration management
kina config show
kina config set KEY VALUE  # Prints guidance; direct setting is not implemented yet
kina config get KEY        # Prints guidance; direct lookup is not implemented yet
kina config reset
kina config path
```
