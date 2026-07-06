# GitOps Deployment

## Sync Waves

| Wave | Content | Justification |
|------|---------|---------------|
| -3 | Bootstrap namespace, secrets | Nothing starts without these |
| -2 | Storage (PVC, Longhorn, versioned seed jobs), **swapoff DaemonSet** | Pods need volumes ready; swap must be off before model pods start |
| -1 | Operators (NVIDIA GPU, Prometheus) | Must run before workloads to capture metrics |
| 0 | Workloads (StatefulSets) | Core model serving |
| 1 | Content (Grafana dashboards, gateway config, ServiceMonitor) | Depends on workloads |
| 2+ | Post-sync (smoke tests, notifications) | Validation final |

## ArgoCD ApplicationSet

Production deployment uses `apps/argocd-appset-prod.yaml` with:
- Automated prune and self-heal
- Server-side apply
- Retry with exponential backoff
- Sync waves: -3 (secrets) → -2 (infrastructure + swapoff) → -1 (GPU operator) → 0 (model pods) → 1 (gateway + dashboards) → 2 (smoke tests)

## Health Checks for ML CRDs

ArgoCD does not natively understand KServe/Triton CRDs. Custom Lua health checks are configured to avoid perpetual "Progressing" state.

## Environment Promotion

All critical vLLM parameters are centralized in `environments/{dev,staging,prod}/values.yaml`:

| Parameter | Dev | Staging | Prod |
|---|---|---|---|
| `gpu-memory-utilization` | 0.85 | 0.88 | 0.90 |
| `max-model-len` | 4096 | 8192 | 8192 |
| `max-num-seqs` | 64 | 128 | 256 |
| `kv-cache-dtype` | fp8 | fp8 | fp8 |
| `enable-prefix-caching` | ✓ | ✓ | ✓ |
| `block-size` | 16 | 16 | 16 |
| KEDA autoscaling | off | on | on |
| ServiceMonitor | on | on | on |
| swapoff DaemonSet | on | on | on |
| QoS Guaranteed | ✓ | ✓ | ✓ |