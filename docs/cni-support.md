# CNI Support

## PTP CNI (Default)
- **Compatibility**: Optimized for Apple Container
- **Simplicity**: Point-to-point networking with host-local IPAM
- **Multi-node support**: Worker nodes receive PTP config, and kina configures cross-node routes after workers join

```bash
# Create cluster with specific CNI
kina create test-ptp --cni ptp

# Create a multi-node cluster using the default PTP CNI
kina create test-ptp-multi --workers 2
```
