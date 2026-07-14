# Observability

## Stack (LGTM)

| Layer | Tool | Purpose |
|-------|------|---------|
| Metrics | Prometheus + Mimir | Long-term metric storage (2-year retention) |
| Logs | Loki | Log aggregation |
| Traces | Tempo + OpenTelemetry | Distributed tracing |
| Dashboards | Grafana | Unified visualisation |
| GPU Metrics | DCGM Exporter | GPU utilisation, memory, temperature, ECC |
| vLLM Metrics | ServiceMonitor | Scrapes vLLM `/metrics` every 10s (KV cache, queue, prefix cache, TTFT) |

## Dashboards

- `observability/grafana-dashboards/nvidia-dcgm-dashboard.json` — GPU health
- `observability/grafana-dashboards/vllm-dashboard.json` — **18 panels**:
  - Request rate, P95 latency, error rate, tokens/s, active models, OOM kills
  - **KV cache usage (%)** — `vllm:gpu_cache_usage_perc * 100`
  - **Prefix cache hit rate (%)** — `vllm:gpu_prefix_cache_hits_total / queries_total`
  - **Request queue depth** — `vllm:num_requests_waiting`
  - **TTFT (p95 + p50, ms)** — `vllm:time_to_first_token_seconds_bucket`
  - **KV cache swap-out blocks** — `increase(vllm:swap_out_blocks[5m])`
  - **GPU VRAM usage (DCGM)** — `DCGM_FI_DEV_FB_USED / DCGM_FI_DEV_FB_TOTAL * 100`
  - **LMCache L1 (CPU) hit rate (%)** — `lmcache_l1_cache_hit_total / lmcache_l1_cache_query_total`
  - **LMCache L2 (NVMe) hit rate (%)** — `lmcache_l2_cache_hit_total / lmcache_l2_cache_query_total`
  - **LMCache L3 (Redis/S3) hit rate (%)** — `lmcache_l3_cache_hit_total / lmcache_l3_cache_query_total`
  - **Prefill skip rate (%)** — `vllm:prefill_skip_total / vllm:prefill_total`
  - **Cache affinity routing distribution** — `x-pod request rate per affinity key` (unequal distribution = desired)
  - **Cache ROI estimate ($/hour saved)** — `cache_hits * (ttft_no_cache - ttft_cached) * gpu_cost_per_second`

## ServiceMonitor

The chart `model-serving-engine` includes a `ServiceMonitor` template (`templates/servicemonitor.yaml`) that configures Prometheus to scrape vLLM's `/metrics` endpoint:

| Parameter | Value | Rationale |
|---|---|---|
| `path` | `/metrics` | vLLM's built-in metrics endpoint |
| `interval` | `10s` | 5-10s recommended for fast alert reaction |
| `scrapeTimeout` | `5s` | Must be < interval |
| `honorLabels` | `true` | Preserve vLLM's original labels |
| `releaseLabel` | `prometheus` | Must match the Prometheus Operator release label |

Enabled in all environments (`serviceMonitor.enabled: true` in dev/staging/prod values).

## Alerting Rules

`observability/prometheus-anomaly-rules.yaml` defines **7 rule groups**:

### Latency, Errors, GPU, Pods, Anomaly (existing)

- Latency: p95 > 2s (Warning), p99 > 5s (Critical)
- Errors: > 5% (Warning), > 15% (Critical)
- GPU: thermal throttle > 85°C, utilisation < 10%, memory > 95%, ECC errors > 100/h
- Pods: CrashLooping (restarts > 3/h), NotReady (10m)
- Anomaly: derivative-based latency/throughput anomaly detection

### KV Cache — vLLM engine (`model-serving.kv-cache` group)

| Alert | Expression | For | Severity |
|---|---|---|---|
| `VLLMKVCacheUsageHigh` | `vllm:gpu_cache_usage_perc > 0.85` | 30s | Warning |
| `VLLMKVCacheUsageCritical` | `vllm:gpu_cache_usage_perc >= 1.0` | 0s | Critical |
| `VLLMRequestsWaitingHigh` | `vllm:num_requests_waiting > 10` | 1m | Critical |
| `VLLMSwapOutBlocksDetected` | `increase(vllm:swap_out_blocks[5m]) > 0` | 0s | Critical |
| `NodeSwapSpaceUsageHigh` | `(1 - SwapFree/SwapTotal) > 0.10` | 2m | Critical |
| `VLLMPrefixCacheHitRateLow` | hit rate < 0.20 | 10m | Warning |

### KV Cache — LMCache distributed middleware (Bible §4.3)

| Alert | Expression | For | Severity |
|---|---|---|---|
| `LMCacheL1HitRateLow` | L1 CPU hit rate < 0.30 | 10m | Warning |
| `LMCacheL2HitRateLow` | L2 NVMe hit rate < 0.20 | 15m | Warning |
| `LMCacheL3HitRateLow` | L3 Redis/S3 hit rate < 0.10 | 15m | Warning |
| `VLLMPrefillSkipRateLow` | prefill skip < 10% while queue busy | 15m | Info |
| `SSMModelPagedAttentionMisconfigured` | SSM pod with PagedAttention args detected | 0s | Critical |
| `CacheRoutingHeaderAbsent` | `x-cache-affinity-key` header missing during traffic | 0s | Info |

The SSM alert catches a critical anti-pattern (Bible §14): Mamba/SSM models use a fixed-size recurrent state, **not** a paginable KV cache. Deploying them with `--enable-prefix-caching` or `--block-size` is a misconfiguration.

## Alert Routing

`observability/alertmanager-routes.yaml`:

- **Critical** → PagerDuty + Slack `#ml-incidents`
- **Warning** → Slack `#ml-ops`
- **GPU** → Slack `#gpu-ops`
- **Serving** → Slack `#ml-ops`
- Inhibit: critical suppresses warning for same alert

## Anomaly Detection

- Derivative-based latency anomaly (p95 increasing >0.1 s/s over 30m)
- Throughput drop anomaly (deriv < -0.5 req/s/s over 30m)

## LMCache Distributed Cache Middleware

The platform deploys **LMCache** (Bible §4.3) as a per-GPU-node DaemonSet to break the per-instance KV cache silo. Cache becomes **shared across pods**, **persistent across restarts**, and **hierarchical** across memory tiers.

### Architecture

```
vLLM pod (miss) → LMCache daemon (node-local) → L1 CPU DRAM → L2 NVMe disk → L3 Redis/S3
```

LMCache runs as an independent daemon (no fate-sharing with the engine). A vLLM crash does **not** lose the cache.

### Tiered Cache Hierarchy

| Tier | Backend | Latency | Capacity | Enabled In |
|---|---|---|---|---|
| L0 | vLLM GPU HBM (PagedAttention) | ~ns | limited by `gpu-memory-utilization` | all envs |
| L1 | CPU DRAM (LMCache) | ~µs | node-local RAM | prod, staging |
| L2 | Local NVMe disk (LMCache) | ~ms | 200 GiB (prod) / 100 GiB (staging) | prod, staging |
| L3 | Redis or S3 (LMCache) | ~10ms | cluster-wide | prod (Redis) |

### Helm Configuration

| Parameter | Prod | Staging | Dev |
|---|---|---|---|
| `lmcache.enabled` | true | true | false |
| `lmcache.cpuWorkers` | 4 | 2 | — |
| `lmcache.disk.maxSize` | 200GiB | 100GiB | — |
| `lmcache.redis.enabled` | true | false | — |
| `lmcache.resources.limits.cpu` | 2 | 1 | — |
| `lmcache.resources.limits.memory` | 16Gi | 8Gi | — |

Templates: `lmcache-daemonset.yaml`, `lmcache-configmap.yaml`, `lmcache-service.yaml` in `charts/model-serving-engine/templates/`.

### SafeTensors Cache Persistence

`cachePersistence.enabled` provisions a dedicated PVC (`/cache/kv`) via Longhorn so the KV cache survives pod restarts. On startup, vLLM restores from the persisted cache instead of cold-prefilling, reducing TTFT from ~11s to ~1.5s on a 128K context with 80% hit rate.

| Parameter | Prod | Staging | Dev |
|---|---|---|---|
| `cachePersistence.enabled` | true | true | false |
| `cachePersistence.storageClass` | longhorn | longhorn | — |
| `cachePersistence.size` | 50Gi | 30Gi | — |

See [`docs/explain/vllm-kv-cache.md`](../explain/vllm-kv-cache.md) for the theoretical foundation and ROI analysis.