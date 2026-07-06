# Capacity Forecasting

## Approach

- **Holt-Winters** seasonal models via Prometheus recording rules for predictable load patterns
- **KEDA** autoscaling on vLLM-specific metrics (NOT CPU/RAM — classic HPA is inoperant for GPU-bound LLM workloads)
- **Periodic load tests** (`tests/load/`) run in CI to detect capacity drift before it becomes an incident

## KEDA ScaledObject

The platform uses a KEDA `ScaledObject` (`charts/model-serving-engine/templates/hpa.yaml`) with two Prometheus triggers:

| Trigger | Metric | Threshold | Action |
|---|---|---|---|
| Queue depth | `vllm:num_requests_waiting` | > 5 | Scale out |
| Cache pressure | `vllm:gpu_cache_usage_perc` | > 0.85 | Scale out |

| Parameter | Prod | Staging | Dev |
|---|---|---|---|
| `enabled` | true | true | false (legacy HPA) |
| `minReplicaCount` | 2 | 1 | — |
| `maxReplicaCount` | 4 | 2 | — |
| `pollingInterval` | 15s | 15s | — |
| `cooldownPeriod` | 60s | 60s | — |

A legacy HPA on CPU/RAM is retained as a fallback for environments without KEDA installed.

## Key Metrics for Forecasting

| Metric | Recording Rule | Retention |
|--------|---------------|-----------|
| Request rate (5m avg) | `model:serving:request_rate:5m` | 2 years (Mimir) |
| P95 latency (5m) | `model:serving:latency_p95:5m` | 2 years (Mimir) |
| GPU utilisation | `model:gpu:utilization:5m` | 2 years (Mimir) |
| Active models | `model:serving:active_models` | 2 years (Mimir) |
| KV cache usage | `vllm:gpu_cache_usage_perc` | 2 years (Mimir) |
| Request queue depth | `vllm:num_requests_waiting` | 2 years (Mimir) |

## Scaling Triggers

- `vllm:num_requests_waiting` > 5 → KEDA scale out
- `vllm:gpu_cache_usage_perc` > 0.85 → KEDA scale out + alert
- GPU memory > 90% → alert + investigate model replacement
- P95 latency > 2000ms → gateway failover to SaaS