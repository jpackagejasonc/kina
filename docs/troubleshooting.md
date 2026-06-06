# Troubleshooting

## Common Issues

### Apple Container Not Found
```bash
# Check Apple Container installation
container --version

# Start the Apple Container service if needed
container system start

# Check if service is running
container system status

# Verify PATH configuration
echo $PATH | grep container
```

**Solution**: If Apple Container is not found, install it via Homebrew (`brew install container`) or from [Apple Container Releases](https://github.com/apple/container/releases). If installed but not working, restart the service with `container system restart`.

### Cluster Creation Fails
```bash
# Check cluster status
kina status my-cluster --verbose

# Enable verbose logging
RUST_LOG=debug kina create my-cluster --retain

# Manual cleanup
kina delete my-cluster
```

### Kubeconfig Issues
```bash
# Check kubeconfig location
kina config path
ls ~/.kube/

# Regenerate kubeconfig
kina export my-cluster --output ~/.kube/my-cluster
export KUBECONFIG=~/.kube/my-cluster
```

### CNI Readiness Issues
```bash
# Approve pending CSRs
kina approve-csr my-cluster

# Check node readiness
kubectl get nodes
```

## Debug Commands

```bash
# Comprehensive cluster status
kina status my-cluster --verbose --output yaml

# Container inspection
container list
container inspect CONTAINER_NAME

# Kubernetes debugging
kubectl get events --sort-by='.lastTimestamp'
kubectl describe nodes
```

## Getting Help

- **GitHub Issues**: [Report bugs and feature requests](https://github.com/vinnie357/kina/issues)
- **Documentation**: [Project documentation](https://github.com/vinnie357/kina/tree/main/docs)
- **Community**: [Discussions and support](https://github.com/vinnie357/kina/discussions)
