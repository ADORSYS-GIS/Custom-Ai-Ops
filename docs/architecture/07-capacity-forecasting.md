# Capacity Forecasting

## Approach

- **Holt-Winters** seasonal models for predictable load patterns
- **KEDA** autoscaling on vLLM-specific metrics (NOT CPU/RAM — classic HPA is inoperant for GPU-bound LLM workloads)
- **Periodic load tests** (`tests/load/`) run in CI to detect capacity drift before it becomes an incident

## KEDA ScaledObject

The platform uses a KEDA `ScaledObject` (`charts/model-serving-engine/templates/hpa.yaml`) with two vLLM metric triggers:

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

## Scaling Triggers

- `vllm:num_requests_waiting` > 5 → KEDA scale out
- `vllm:gpu_cache_usage_perc` > 0.85 → KEDA scale out
- GPU memory > 90% → investigate model replacement
- P95 latency > 2000ms → investigate and scale out