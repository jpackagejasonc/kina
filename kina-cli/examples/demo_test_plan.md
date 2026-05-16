# Kina Demo Test Plan

This document outlines the test plan for the Kina demo workflow using mise commands.

## Overview

The demo workflow demonstrates Kina's ability to:
- Create Kubernetes clusters using Apple Container runtime
- Install Traefik as a Gateway API controller (Gateway API CRDs + Traefik DaemonSet)
- Deploy demo applications with HTTPRoute-based routing
- Provide working examples for users

## Prerequisites

- macOS 26+ with Apple Container runtime
- kubectl installed and in PATH
- mise installed and configured
- Kina project cloned with mise.toml configured

## Test Commands

### 1. Demo Cluster Creation
```bash
mise run test:cluster
```

**Expected Behavior:**
- ✅ Generate unique cluster name with timestamp: `demo-YYYYMMDD-HHMMSS`
- ✅ Build kina CLI successfully
- ✅ Create cluster with control plane node
- ✅ Install Traefik with all manifests in `kina-cli/manifests/traefik/`:
  - gateway-api-crds.yaml (Gateway API v1.5.1 standard CRDs)
  - ns-and-sa.yaml (namespace and ServiceAccount)
  - rbac.yaml (RBAC)
  - traefik-config.yaml (static config)
  - gatewayclass.yaml (GatewayClass `traefik`)
  - traefik-daemonset.yaml (DaemonSet)
  - gateway.yaml (shared `traefik` Gateway)
- ✅ Wait for traefik pods to be ready
- ✅ Deploy demo application
- ✅ Create HTTPRoute attached to the shared Gateway
- ✅ Provide access URL and instructions

**Success Criteria:**
- Cluster creates without errors
- traefik pods show "1/1 Running"
- Gateway `traefik` reports `Programmed=True`
- Demo HTTPRoute reports `Accepted=True`
- Demo app responds to curl with the correct title

### 2. Demo Validation
```bash
mise run test:cluster:validate
```

**Expected Behavior:**
- ✅ Find latest demo cluster automatically
- ✅ Test cluster connectivity
- ✅ Verify Traefik is running
- ✅ Test demo app via HTTPRoute
- ✅ Display test results

### 3. Demo Cleanup
```bash
mise run test:cluster:cleanup
```

**Expected Behavior:**
- ✅ Find all demo clusters (prefix: `demo-`)
- ✅ Delete all demo clusters and kubeconfig files

## Manual Verification Steps

```bash
# 1. Verify cluster is running
CLUSTER_NAME=$(container list | grep demo | head -1 | awk '{print $1}' | sed 's/-control-plane//')
container list | grep $CLUSTER_NAME

# 2. Check traefik pods
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get pods -n traefik

# 3. Check Gateway and HTTPRoute
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get gateway -n traefik traefik
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get httproute -A
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME describe httproute kina-demo-route

# 4. Test demo app
CLUSTER_IP=$(container list | grep "$CLUSTER_NAME-control-plane" | awk '{print $NF}')
curl -H "Host: $CLUSTER_NAME-control-plane.<dns_domain>" http://$CLUSTER_IP

# 5. Verify GatewayClass
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get gatewayclass
# Should show: traefik   traefik.io/gateway-controller   Accepted   ...
```

## Common Issues and Solutions

### Issue: Traefik Installation Fails
**Symptoms:** Error during install step
**Solution:**
- Verify manifests directory exists: `ls kina-cli/manifests/traefik/`
- Check Gateway API CRDs were applied: `kubectl get crd | grep gateway.networking.k8s.io`
- Check working directory in mise script

### Issue: HTTPRoute Returns 404
**Symptoms:** curl returns 404
**Solution:**
- Check HTTPRoute is Accepted: `kubectl describe httproute <name>`
- Verify hostname matches the `Host` header used in curl
- Confirm `parentRefs` points at `traefik/traefik` with `sectionName: web`

### Issue: Gateway Not Programmed
**Symptoms:** `kubectl get gateway -n traefik traefik` shows `Programmed=False`
**Solution:**
- Check Traefik pod logs: `kubectl logs -n traefik -l app=traefik`
- Confirm GatewayClass `traefik` exists and is Accepted

## Validation Checklist

- [ ] Cluster creates successfully
- [ ] Gateway API CRDs install (`gatewayclasses`, `gateways`, `httproutes`, `referencegrants`)
- [ ] Traefik DaemonSet runs on every node (`1/1 Running`)
- [ ] GatewayClass `traefik` is Accepted
- [ ] Gateway `traefik` is Programmed and listeners are Accepted
- [ ] Demo HTTPRoute is Accepted
- [ ] Demo app responds with the expected title
- [ ] Cleanup removes all demo resources
