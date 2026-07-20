# llm-d Router Helm Chart

## Overview

This chart deploys the **llm-d Router** components for cache-aware request routing in Kubernetes. It includes:

- **Proxy**: HTTP proxy that intercepts inference requests and applies cache-aware routing
- **Endpoint Picker (EPP)**: Selects optimal backend endpoints based on KV-cache affinity

The Router uses **InferencePool** custom resources to discover and select vLLM backend instances, routing requests to endpoints with the highest cache hit probability.

## Architecture

```
Client Request → Proxy → EPP → vLLM Backend (with cache affinity)
                    ↓
              InferencePool CRD
              (backend discovery)
```

## Prerequisites

- Kubernetes 1.24+
- Helm 3.8+
- Gateway API CRDs v1.5.1+ installed (use `llm-d-infrastructure` chart)
- Gateway API Inference Extension (GAIE) v1.5.0+ installed

## Installation

### Install with default values:

```bash
helm install llm-d-router ./charts/llm-d-router \
  --namespace llm-d-system \
  --create-namespace
```

### Install with custom values:

```bash
helm install llm-d-router ./charts/llm-d-router \
  --namespace llm-d-system \
  --create-namespace \
  --values environments/dev/llm-d-router-values.yaml
```

## Configuration

### Key Configuration Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `proxy.enabled` | Enable Proxy deployment | `true` |
| `proxy.replicaCount` | Number of Proxy replicas | `2` |
| `proxy.image.repository` | Proxy container image | `ghcr.io/llm-d/proxy` |
| `proxy.image.tag` | Proxy image tag | `v0.8.1` |
| `proxy.service.type` | Proxy service type | `ClusterIP` |
| `proxy.service.port` | Proxy service port | `8080` |
| `epp.enabled` | Enable Endpoint Picker deployment | `true` |
| `epp.replicaCount` | Number of EPP replicas | `2` |
| `epp.image.repository` | EPP container image | `ghcr.io/llm-d/endpoint-picker` |
| `epp.image.tag` | EPP image tag | `v0.8.1` |
| `epp.config.scorers` | List of scoring algorithms | See values.yaml |
| `epp.config.filters` | Request filtering rules | See values.yaml |
| `epp.config.selector.strategy` | Endpoint selection strategy | `weighted` |

### Scoring Algorithms

The EPP supports multiple scoring algorithms to rank backend endpoints:

- **prefix-match**: Matches request prompt prefixes with cached KV blocks
- **semantic-similarity**: Uses embeddings for semantic cache matching
- **load-balancing**: Considers backend load and capacity
- **latency**: Prioritizes low-latency endpoints
- **custom**: User-defined scoring logic

Configure scorers in `epp.config.scorers` array:

```yaml
epp:
  config:
    scorers:
      - name: prefix-match
        weight: 0.5
        enabled: true
      - name: load-balancing
        weight: 0.3
        enabled: true
      - name: latency
        weight: 0.2
        enabled: true
```

### Filtering Rules

Filters exclude incompatible backends before scoring:

```yaml
epp:
  config:
    filters:
      - name: model-compatibility
        enabled: true
      - name: capacity-check
        enabled: true
      - name: health-check
        enabled: true
```

### Selection Strategy

The selector determines how to choose from scored endpoints:

- **weighted**: Probabilistic selection based on scores (default)
- **top-n**: Round-robin among top N endpoints
- **best**: Always select highest-scored endpoint

```yaml
epp:
  config:
    selector:
      strategy: weighted
      topN: 3
```

## Resource Requirements

### Default Resource Limits

**Proxy:**
- CPU: 100m request, 500m limit
- Memory: 128Mi request, 512Mi limit

**EPP:**
- CPU: 200m request, 1000m limit
- Memory: 256Mi request, 1Gi limit

### Production Recommendations

For production workloads with high request volumes:

```yaml
proxy:
  resources:
    requests:
      cpu: 500m
      memory: 512Mi
    limits:
      cpu: 2000m
      memory: 2Gi

epp:
  resources:
    requests:
      cpu: 1000m
      memory: 1Gi
    limits:
      cpu: 4000m
      memory: 4Gi
```

## RBAC Permissions

The chart creates:

- **ServiceAccount**: `llm-d-router`
- **ClusterRole**: Read/list/watch InferencePools, HTTPRoutes, Services, Endpoints
- **ClusterRoleBinding**: Binds ClusterRole to ServiceAccount

These permissions allow EPP to discover vLLM backends via InferencePool CRDs.

## Monitoring

The Router components expose Prometheus metrics:

**Proxy metrics (port 9090):**
- `llm_d_proxy_requests_total`: Total requests received
- `llm_d_proxy_routing_decisions`: Routing decision distribution
- `llm_d_proxy_latency_seconds`: Request latency histogram

**EPP metrics (port 9091):**
- `llm_d_epp_scoring_duration_seconds`: Time to score endpoints
- `llm_d_epp_backend_scores`: Current scores for each backend
- `llm_d_epp_cache_hit_rate`: Estimated cache hit rate

## Integration with vLLM

The Router automatically discovers vLLM backends through **InferencePool** CRDs:

```yaml
apiVersion: llm-d.io/v1alpha1
kind: InferencePool
metadata:
  name: vllm-dev
  namespace: llm-d-system
spec:
  modelName: meta-llama/Llama-3.1-8B
  selector:
    matchLabels:
      app: vllm
      env: dev
  cachingEnabled: true
```

Create InferencePools for your vLLM deployments, and EPP will route requests accordingly.

## Troubleshooting

### Proxy cannot reach EPP

Check service connectivity:

```bash
kubectl get svc -n llm-d-system llm-d-router-epp
kubectl logs -n llm-d-system -l app.kubernetes.io/component=proxy
```

Verify EPP URL in Proxy environment variables:

```bash
kubectl describe deployment -n llm-d-system llm-d-router-proxy | grep EPP_URL
```

### EPP cannot discover backends

Check ClusterRole permissions:

```bash
kubectl describe clusterrole llm-d-router
```

Verify InferencePool CRDs exist:

```bash
kubectl get inferencepools -A
```

Check EPP logs for discovery errors:

```bash
kubectl logs -n llm-d-system -l app.kubernetes.io/component=epp
```

### Low cache hit rate

1. Verify vLLM backends are emitting KV-cache events
2. Check EPP scorer configuration (increase `prefix-match` weight)
3. Review request patterns (cache works best for similar prompts)

```bash
# Check EPP metrics
kubectl port-forward -n llm-d-system svc/llm-d-router-epp 9091:9091
curl http://localhost:9091/metrics | grep cache_hit_rate
```

## Uninstallation

```bash
helm uninstall llm-d-router --namespace llm-d-system
```

Note: This does NOT remove InferencePool CRDs or Gateway API resources. Use `llm-d-infrastructure` chart to manage those.

## References

- [llm-d Architecture](../../docs/explain/llm-d.md)
- [Gateway API Documentation](https://gateway-api.sigs.k8s.io/)
- [vLLM Integration Guide](../../docs/vllm-llm-d-integration.md)

## Support

For issues and questions:
- GitHub Issues: https://github.com/your-org/Custom-Ai-Ops/issues
- Documentation: [docs/explain/llm-d.md](../../docs/explain/llm-d.md)
