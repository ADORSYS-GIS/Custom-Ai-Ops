# llm-d-infrastructure

Gateway API and Gateway API Inference Extension (GAIE) CRDs required for llm-d.

## Overview

This chart installs the foundational Custom Resource Definitions needed for llm-d:
- **Gateway API v1.2.1+** - Standard Kubernetes networking API (Gateway, HTTPRoute, etc.)
- **Gateway API Inference Extension v0.3.0+** - Inference-specific extensions (InferencePool, InferenceObjective, etc.)

## Prerequisites

- Kubernetes 1.28+
- Helm 3.12+
- Cluster admin permissions (CRD installation requires cluster-wide permissions)

## Installation

### Quick Start

```bash
helm install llm-d-infrastructure charts/llm-d-infrastructure \
  --namespace llm-d-system \
  --create-namespace
```

### Verify Installation

```bash
# Check Gateway API CRDs
kubectl api-resources | grep gateway

# Check GAIE CRDs
kubectl api-resources | grep inference

# Check installation job
kubectl get jobs -n llm-d-system -l app.kubernetes.io/name=llm-d-infrastructure

# Check job logs
kubectl logs -n llm-d-system -l app.kubernetes.io/name=llm-d-infrastructure
```

### Expected Output

**Gateway API resources**:
- `gateways`
- `httproutes`
- `grpcroutes`
- `tcproutes`
- `udproutes`
- `referencegrants`
- `gatewayclasses`

**GAIE resources**:
- `inferencepools`
- `inferenceobjectives`
- `inferencemodelrewrites`

## Configuration

### Values

| Parameter | Description | Default |
|-----------|-------------|---------|
| `gatewayAPI.enabled` | Install Gateway API CRDs | `true` |
| `gatewayAPI.version` | Gateway API version | `v1.2.1` |
| `gaie.enabled` | Install GAIE CRDs | `true` |
| `gaie.version` | GAIE version | `v0.3.0` |
| `job.enabled` | Use Job for installation | `true` |
| `job.image` | kubectl image for job | `bitnami/kubectl:1.30` |

### Custom Values

```yaml
# Example: Disable GAIE (Gateway API only)
gaie:
  enabled: false

# Example: Use different versions
gatewayAPI:
  version: "v1.2.2"
gaie:
  version: "v0.4.0"
```

## Uninstallation

```bash
helm uninstall llm-d-infrastructure -n llm-d-system
```

**Note**: This does NOT automatically delete the CRDs (Helm best practice). To delete CRDs:

```bash
# Delete Gateway API CRDs
kubectl delete crd \
  gateways.gateway.networking.k8s.io \
  httproutes.gateway.networking.k8s.io \
  grpcroutes.gateway.networking.k8s.io \
  tcproutes.gateway.networking.k8s.io \
  udproutes.gateway.networking.k8s.io \
  referencegrants.gateway.networking.k8s.io \
  gatewayclasses.gateway.networking.k8s.io

# Delete GAIE CRDs
kubectl delete crd \
  inferencepools.inference.networking.x-k8s.io \
  inferenceobjectives.inference.networking.x-k8s.io \
  inferencemodelrewrites.inference.networking.x-k8s.io
```

## Troubleshooting

### Job Fails with Permission Denied

**Symptom**: Job logs show `Error from server (Forbidden): customresourcedefinitions.apiextensions.k8s.io is forbidden`

**Solution**: Ensure the ServiceAccount has cluster-admin or sufficient RBAC permissions:

```bash
kubectl get clusterrolebinding llm-d-infrastructure-installer -o yaml
```

### CRDs Already Exist

**Symptom**: Job logs show `customresourcedefinition.apiextensions.k8s.io "gateways.gateway.networking.k8s.io" already exists`

**Solution**: This is expected if Gateway API is already installed. The job will continue successfully.

### API Resources Not Showing

**Symptom**: `kubectl api-resources | grep gateway` returns nothing

**Solution**: Wait a few seconds for API server to refresh, then check again:

```bash
kubectl get crd | grep gateway
kubectl api-resources | grep gateway
```

## Next Steps

After installing this chart, you can proceed to install:

1. **llm-d-router** - Proxy + Endpoint Picker for intelligent routing
2. **llm-d-kv-cache-indexer** - KV Cache Indexer for precise cache-aware routing
3. **InferencePool resources** - Group your vLLM replicas

See documentation: `docs/explain/llm-d.md`

## References

- Gateway API: https://gateway-api.sigs.k8s.io/
- Gateway API GitHub: https://github.com/kubernetes-sigs/gateway-api
- GAIE GitHub: https://github.com/kubernetes-sigs/gateway-api-inference-extension
- llm-d: https://llm-d.ai
