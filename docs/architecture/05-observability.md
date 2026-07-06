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

- `observability/grafana-dashboards/dcgm-dashboard.json` — GPU health
- `observability/grafana-dashboards/model-serving-dashboard.json` — 12 panels:
  - Request rate, P95 latency, error rate, tokens/s, active models, OOM kills
  - **KV cache usage (%)** — `vllm:gpu_cache_usage_perc * 100`
  - **Prefix cache hit rate (%)** — `vllm:gpu_prefix_cache_hits_total / queries_total`
  - **Request queue depth** — `vllm:num_requests_waiting`
  - **TTFT (p95 + p50, ms)** — `vllm:time_to_first_token_seconds_bucket`
  - **KV cache swap-out blocks** — `increase(vllm:swap_out_blocks[5m])`
  - **GPU VRAM usage (DCGM)** — `DCGM_FI_DEV_FB_USED / DCGM_FI_DEV_FB_TOTAL * 100`

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

`observability/prometheus-anomaly-rules.yaml` defines **6 rule groups**:

### Latency, Errors, GPU, Pods, Anomaly (existing)

- Latency: p95 > 2s (Warning), p99 > 5s (Critical)
- Errors: > 5% (Warning), > 15% (Critical)
- GPU: thermal throttle > 85°C, utilisation < 10%, memory > 95%, ECC errors > 100/h
- Pods: CrashLooping (restarts > 3/h), NotReady (10m)
- Anomaly: derivative-based latency/throughput anomaly detection

### KV Cache (new — `model-serving.kv-cache` group)

| Alert | Expression | For | Severity |
|---|---|---|---|
| `VLLMKVCacheUsageHigh` | `vllm:gpu_cache_usage_perc > 0.85` | 30s | Warning |
| `VLLMKVCacheUsageCritical` | `vllm:gpu_cache_usage_perc >= 1.0` | 0s | Critical |
| `VLLMRequestsWaitingHigh` | `vllm:num_requests_waiting > 10` | 1m | Critical |
| `VLLMSwapOutBlocksDetected` | `increase(vllm:swap_out_blocks[5m]) > 0` | 0s | Critical |
| `NodeSwapSpaceUsageHigh` | `(1 - SwapFree/SwapTotal) > 0.10` | 2m | Critical |
| `VLLMPrefixCacheHitRateLow` | hit rate < 0.20 | 10m | Warning |

## Alert Routing

`observability/alertmanager-routes/config.yaml`:

- **Critical** → PagerDuty + Slack `#ml-incidents`
- **Warning** → Slack `#ml-ops`
- **GPU** → Slack `#gpu-ops`
- **Serving** → Slack `#ml-ops`
- Inhibit: critical suppresses warning for same alert

## Anomaly Detection

- Derivative-based latency anomaly (p95 increasing >0.1 s/s over 30m)
- Throughput drop anomaly (deriv < -0.5 req/s/s over 30m)