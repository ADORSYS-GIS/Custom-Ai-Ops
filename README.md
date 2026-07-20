# Custom-Ai-Ops

Kubernetes LLM serving platform combining **vLLM** (inference), **LMCache** (hierarchical KV cache), and **llm-d** (cache-aware routing).

| Technology | Role | Version |
|------------|------|---------|
| [vLLM](https://github.com/vllm-project/vllm) | Inference engine (continuous batching, FP8 KV cache) | v0.25.1 |
| [LMCache](https://github.com/LMCache/LMCache) | Distributed multi-tier KV cache (MP server mode) | v0.5.1 |
| [llm-d](https://llm-d.ai) | Cache-aware routing (EPP + KV-Cache Indexer + InferencePool CRD) | v0.8.1 |
| [Gateway API](https://gateway-api.sigs.k8s.io/) | Standardized K8s API for L7 routing | v1.5.1 |
| [GAIE](https://github.com/kubernetes-sigs/gateway-api-inference-extension) | Gateway API Inference Extension CRDs | v1.5.0 |

---

## Architecture

```
Client → Gateway API → llm-d Router (Envoy + EPP) → vLLM (StatefulSet)
                                    ↓                    ↓
                             InferencePool CRD     LMCache DaemonSet
                             KV-Cache Indexer      (L1 RAM / L2 NVMe / L3 Redis)
```

---

## Cache Hierarchy

| Tier | Location | Latency | Hit Rate |
|------|----------|---------|----------|
| **L0** | GPU VRAM | ~ns | 20% |
| **L1** | CPU RAM | ~25ms | 35% |
| **L2** | NVMe/SSD | ~40ms | 25% |
| **L3** | Redis | ~6ms | 15% |
| **Miss** | Full recompute | ~250ms (GPU) | 5% |
| **Combined** | 95% hit rate | ~35ms weighted avg | 95% |

---

## Repository Structure

```
charts/
  model-serving-engine/   vLLM + LMCache (StatefulSet, DaemonSet, ConfigMap, Service)
  llm-d/                   Router (Envoy+EPP), KV-Cache Indexer, InferencePool CRD
  llm-d-router/            Standalone router (Proxy + EPP, 4-stage pipeline)
  llm-d-infrastructure/    Gateway API v1.5.1 + GAIE v1.5.0 CRDs
  ai-gateway/              HTTPRoute, BackendTrafficPolicy, rate limiting
  redis/                   Standalone or Sentinel HA (1 primary + 2 replicas + 3 sentinels)
environments/
  dev/                     LMCache off, llm-d off, replicaCount:1
  staging/                 LMCache L1+L2, llm-d EPP (Phase 1), maxFailures:3
  prod/                    LMCache L1+L2+L3 (Redis), llm-d EPP+Indexer (Phase 2), replicaCount:2
tools/                     Rust CLIs: engine-selector, vram-budget-calc, cache-roi-calc, model-onboarding
tests/
  smoke/                   vLLM and llm-d smoke tests + TTFT cache-hit validation
  load/                    k6 load test (3-stage ramp: 5→10→20 req/s)
  chaos/                   LitmusChaos experiments (GPU + Redis failure scenarios)
  local/                   Local Qwen test suite (49+ tests)
docs/                      Architecture, ADR, explain (11 scenarios), runbooks
addons/                    nvidia-gpu-operator, llm-d (experimental), rdma-device-plugin
```

---

## Quick Start

### Prerequisites

```bash
# Required
helm                             # Helm 3.x
kubectl                          # Kubernetes CLI
cargo                            # Rust toolchain (for CLI tools)
```

### 1. Rust Tools

```bash
cargo build --release
cargo test
```

### 2. Validate Helm Charts

```bash
helm lint charts/model-serving-engine
helm template charts/model-serving-engine -f environments/prod/values.yaml | wc -l
```

### 3. Deploy Model Serving

```bash
# vLLM + LMCache
helm install vllm-prod charts/model-serving-engine \
  -f environments/prod/values.yaml \
  --namespace model-serving-prod --create-namespace

# llm-d (production)
helm install llm-d-infra charts/llm-d-infrastructure -n llm-d-system --create-namespace
helm install llm-d charts/llm-d -f environments/prod/llm-d/values.yaml -n llm-d-system
```

### 4. Redis (L3 Cache — Required for Production)

```bash
# Standalone (dev/staging)
helm install redis-cache charts/redis/ \
  --namespace model-serving-prod --set config.maxmemory=8gb

# Sentinel HA (production — auto-failover)
helm install redis-cache charts/redis/ \
  --namespace model-serving-prod \
  --set architecture=sentinel --set auth.enabled=true
```

The chart deploys a `redis-cache-primary` ClusterIP service. Sentinel mode provides automatic failover (primary → replica on node loss).

### 5. NIXL / RDMA (PD Disaggregation — Pre-Configured, Disabled)

Enable when RDMA-capable hardware is available (H100/H200 with InfiniBand/RoCE):

```bash
helm upgrade vllm-prod charts/model-serving-engine/ \
  -f environments/prod/values.yaml \
  --set disaggregation.enabled=true \
  --set disaggregation.role=prefill \
  --set disaggregation.nixl.enabled=true
```

**Requirements:** RDMA device plugin (`addons/rdma-device-plugin/`), `pip install "lmcache[nixl]"` in vLLM image, nodes labeled `rdma/device=true`.

---

## Environments

| Parameter | Dev | Staging | Prod |
|-----------|-----|---------|------|
| LMCache | off | L1 + L2 | L1 + L2 + L3 (Redis) |
| llm-d EPP | off | on (Phase 1) | on (Phase 2) |
| KV-Cache Indexer | off | off | on (Redis storage) |
| Replicas | 1 | 1 | 2 |
| Circuit Breaker | — | maxFailures:3 | maxFailures:5 |
| emitKvEvents | false | false | true |
| QoS | Guaranteed | Guaranteed | Guaranteed |
| Disaggregation | off | off | off |

---

## CLI Tools (Rust)

| Tool | Function |
|------|----------|
| `engine-selector` | Model format → engine → cache strategy |
| `vram-budget-calc` | VRAM budget (unified and disaggregated P/D) |
| `cache-roi-calc` | KV cache ROI: heuristic vs EPP routing |
| `model-onboarding` | Scaffolding for new model onboarding |

---

## Validation

```bash
# Helm chart validation
helm lint charts/model-serving-engine charts/llm-d charts/redis

# Documentation references
bash tools/validate-docs.sh

# Model registry consistency
bash tools/validate-registry.sh

# Redis chaos validation (after LitmusChaos experiments)
bash tests/chaos/redis-chaos-validate.sh

# TTFT cache-hit smoke test (requires API endpoint)
bash tests/smoke/ttft-cache-hit-test.sh http://localhost:11434 qwen2.5:1.5b

# Full local test suite (59+ tests, Qwen model via Ollama)
bash tests/local/local-test.sh
```

---

## Audit Summary (11 LMCache+vLLM+llm-d Scenarios)

| # | Scenario | Status |
|---|----------|--------|
| 1 | L0 Cache Hit (VRAM) | ✅ Validated |
| 2 | L1 Cache Hit (CPU RAM) | ✅ Validated |
| 3 | L2 Cache Hit (NVMe) | ✅ Validated |
| 4 | Cache Miss | ✅ Validated |
| 5 | Multi-Node VRAM Hit | ✅ Designed |
| 6 | L3 (Redis) Hit | ✅ Validated |
| 7 | P/D Disaggregation (intra-node) | ✅ Configured\* |
| 8 | P/D Disaggregation (inter-node) | ✅ Configured\* |
| 9 | L1 vs L0 Composite | ✅ Designed |
| 10 | L3 Failure / Circuit Breaker | ✅ Validated |
| 11 | Stale Cache / Invalidation | ⚠️ Documented |

\* Requires RDMA hardware + `lmcache[nixl]` — disabled by default, toggle with `disaggregation.nixl.enabled=true`

### Critical Findings

| Issue | Status | Detail |
|-------|--------|--------|
| ~~Redis not deployed~~ | ✅ Resolved | `charts/redis/` with Sentinel HA (auto-failover) |
| ~~NIXL/RDMA not in charts~~ | ✅ Resolved | NixlConnector + port 5600 + RDMA resources in chart templates |
| ~~kv-cache-event-endpoint commented out~~ | ✅ Resolved | Outdated comment removed — KV events emitted via `LLM_D_KV_EVENTS` env vars (set when `llmD.emitKvEvents: true`). The template handles this dynamically. |
| ~~No Redis chaos test~~ | ✅ Resolved | `tests/chaos/redis-chaos.yaml` — 5 LitmusChaos experiments (pod-delete, network-latency, cpu-hog, memory-hog, sentinel quorum loss) + `redis-chaos-validate.sh` |
| ~~No TTFT smoke test~~ | ✅ Resolved | `tests/smoke/ttft-cache-hit-test.sh` — measures TTFT cache-miss vs cache-hit improvement with session affinity validation |

---

## Documentation

| Document | Topic |
|----------|-------|
| [docs/explain/lmcache+vllm+llm-d.md](docs/explain/lmcache+vllm+llm-d.md) | 11 scenarios — Complete reference |
| [docs/explain/vllm+lmcache.md](docs/explain/vllm+lmcache.md) | vLLM + LMCache integration |
| [docs/explain/llm-d.md](docs/explain/llm-d.md) | llm-d reference (25 sections) |
| [docs/explain/kv-cache.md](docs/explain/kv-cache.md) | KV cache architecture |
| [docs/adr/0004-llm-d-integration.md](docs/adr/0004-llm-d-integration.md) | llm-d integration decision (5-phase plan) |
| [docs/architecture/00-overview.md](docs/architecture/00-overview.md) | Architecture overview |
| [docs/env.md](docs/env.md) | Environment variables, secrets, connections |

---

## License

MIT
