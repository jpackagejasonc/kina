# Apple Container Integration

kina leverages Apple Container technology for running Kubernetes nodes:

## Container Management
- **Native Integration**: Uses Apple Container CLI for container lifecycle
- **Resource Limits**: Configurable CPU, memory, and storage limits
- **Network Integration**: Uses Apple Container VM networking and per-node VM IPs
- **Kubeconfig Integration**: Rewrites kubeconfig server addresses to the control-plane VM IP and merges the context into `~/.kube/config`

## Cluster Architecture
```
┌─────────────────────────────────────────┐
│               macOS Host                │
│  ┌─────────────────────────────────────┐ │
│  │ Apple Container VM per kina node    │ │
│  │  ┌─────────────────────────────────┐ │ │
│  │  │     Kubernetes Node             │ │ │
│  │  │  • kubelet                      │ │ │
│  │  │  • containerd                   │ │ │
│  │  │  • CNI (PTP)                    │ │ │
│  │  └─────────────────────────────────┘ │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```
