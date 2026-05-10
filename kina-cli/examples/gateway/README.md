# Gateway API Examples for Kina (Traefik)

Practical examples of using the Kubernetes Gateway API with Kina clusters running Traefik as the gateway controller.

## Prerequisites

1. **Running Kina cluster with Traefik installed**:
   ```bash
   kina create test-cluster
   kina install traefik --cluster test-cluster
   ```

2. **Verify Traefik and the shared Gateway are healthy**:
   ```bash
   kubectl --kubeconfig ~/.kube/test-cluster get pods -n traefik
   kubectl --kubeconfig ~/.kube/test-cluster get gateway -n traefik traefik
   ```

   The Gateway should report `Programmed=True` and listeners `web` (:80) and `websecure` (:443) with `Accepted=True`.

## How routing works

`kina install traefik` creates one shared `Gateway` named `traefik` in the `traefik` namespace, with `allowedRoutes.namespaces.from: All`. Each example below defines an `HTTPRoute` in `default` whose `parentRefs` point at that shared Gateway:

```yaml
parentRefs:
- name: traefik
  namespace: traefik
  sectionName: web
```

## Examples

### 1. `basic-web-app.yaml`
Single Deployment + Service + HTTPRoute matched on `Host: myapp.local`.

```bash
kubectl --kubeconfig ~/.kube/test-cluster apply -f basic-web-app.yaml
curl -H "Host: myapp.local" http://<cluster-ip>
```

### 2. `multi-service-routing.yaml`
Three backends behind one HTTPRoute on `platform.local`, using `URLRewrite` filters to strip the path prefix before forwarding (`/app` → `/`, etc.).

```bash
curl -H "Host: platform.local" http://<cluster-ip>/app
curl -H "Host: platform.local" http://<cluster-ip>/api
curl -H "Host: platform.local" http://<cluster-ip>/admin
```

### 3. `virtual-hosts.yaml`
Three independent HTTPRoutes, each matching a different hostname (`webapp.local`, `api.local`, `blog.local`), all attached to the same Gateway.

## Finding your cluster IP

```bash
container list   # find <cluster-name>-control-plane row
```

Or use the helper that maps DNS names automatically:

```bash
mise run test:cluster:hosts        # add to /etc/hosts (sudo)
mise run test:cluster:hosts:clean  # remove
```

## Troubleshooting

- **HTTPRoute not Accepted**: `kubectl describe httproute <name>` — confirm `parentRefs` matches the Gateway namespace/name and that `sectionName` is `web` or `websecure`.
- **404 from Traefik**: hostname header doesn't match `hostnames:` in the route; check `curl -H "Host: ..."`.
- **Connection refused**: Traefik DaemonSet not Ready: `kubectl get pods -n traefik`.

## Cleanup

```bash
kubectl --kubeconfig ~/.kube/test-cluster delete -f basic-web-app.yaml
kubectl --kubeconfig ~/.kube/test-cluster delete -f multi-service-routing.yaml
kubectl --kubeconfig ~/.kube/test-cluster delete -f virtual-hosts.yaml
```

## Further reading

- Gateway API: https://gateway-api.sigs.k8s.io/
- Traefik Gateway API provider: https://doc.traefik.io/traefik/reference/routing-configuration/kubernetes/gateway-api/
