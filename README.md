# Custom-Ai-Ops

Kubernetes LLM serving platform combining **vLLM** (inference), **LMCache** (hierarchical KV cache), and **llm-d** (cache-aware routing).

---

## Architecture

```
Client → Gateway API → llm-d Router (Envoy + EPP) → vLLM (StatefulSet)
                                    ↓                    ↓
                             InferencePool CRD     LMCache DaemonSet
                             KV-Cache Indexer      (L1 RAM / L2 NVMe / L3 Redis)
```

| Technology | Role | Version |
|------------|------|---------|
| [vLLM](https://github.com/vllm-project/vllm) | Inference engine (continuous batching, FP8 KV cache) | v0.23.0 |
| [LMCache](https://github.com/LMCache/LMCache) | Distributed multi-tier KV cache (MP server mode) | v0.5.1 |
| [llm-d](https://llm-d.ai) | Cache-aware routing (EPP + KV-Cache Indexer + InferencePool CRD) | v0.9.0 |

---

## Repository Structure

```
charts/
  model-serving-engine/   vLLM + LMCache (StatefulSet, DaemonSet, ConfigMap, Service)
  llm-d/                   Router (Envoy+EPP), KV-Cache Indexer, InferencePool CRD
  llm-d-router/            Standalone router (Proxy + EPP, 4-stage pipeline)
  llm-d-infrastructure/    Gateway API v1.6.0 + GAIE v1.5.0 CRDs
  ai-gateway/              HTTPRoute, BackendTrafficPolicy, rate limiting
environments/
  dev/                     LMCache off, llm-d off
  staging/                 LMCache L1+L2, llm-d EPP
  prod/                    LMCache L1+L2+L3 (Redis), llm-d EPP + Indexer, 2 replicas
tools/                     Rust CLI: engine-selector, vram-budget-calc, cache-roi-calc, model-onboarding
models/                    Model registry (mistral-7b, llama-3-70b)
docs/                      Architecture, ADR, explain, runbooks
tests/                     Smoke (vLLM, llm-d), load (k6), chaos (LitmusChaos)
addons/                    nvidia-gpu-operator, llm-d
```

---

## Quick Start

```bash
# 1. Rust tools
cargo build --release
cargo test                    # 76 tests

# 2. Validate Helm charts
helm lint charts/model-serving-engine
helm template charts/model-serving-engine -f environments/prod/values.yaml

# 3. Deploy
helm install vllm-prod charts/model-serving-engine \
  -f environments/prod/values.yaml \
  --namespace model-serving-prod --create-namespace

# 4. llm-d (prod)
helm install llm-d-infra charts/llm-d-infrastructure -n llm-d-system --create-namespace
helm install llm-d charts/llm-d -f environments/prod/llm-d/values.yaml -n llm-d-system
```

---

## Environments

| Parameter | Dev | Staging | Prod |
|-----------|-----|---------|------|
| LMCache | off | L1 + L2 | L1 + L2 + L3 (Redis) |
| llm-d EPP | off | on | on |
| KV-Cache Indexer | off | off | on |
| vLLM Replicas | 1 | 1 | 2 |
| QoS | Guaranteed | Guaranteed | Guaranteed |

---

## CLI Tools (Rust)

| Tool | Function |
|------|----------|
| `engine-selector` | Detects model format → engine → family → cache strategy |
| `vram-budget-calc` | Computes VRAM budget (unified and disaggregated P/D modes) |
| `cache-roi-calc` | KV cache ROI, heuristic vs EPP routing comparison |
| `model-onboarding` | Scaffolding for new model onboarding |

---

## Validation

```bash
bash tools/validate-lmcache-vllm.sh   # 9 LMCache + vLLM tests
bash tools/validate-docs.sh           # 31 doc references
bash tools/validate-registry.sh       # Model registry consistency
```

---

## Documentation

| Document | Topic |
|----------|-------|
| [docs/explain/vllm+lmcache.md](docs/explain/vllm+lmcache.md) | vLLM + LMCache integration |
| [docs/explain/llm-d.md](docs/explain/llm-d.md) | llm-d reference (20 sections) |
| [docs/explain/kv-cache.md](docs/explain/kv-cache.md) | KV cache architecture |
| [docs/adr/0004-llm-d-integration.md](docs/adr/0004-llm-d-integration.md) | llm-d integration decision |
| [docs/architecture/00-overview.md](docs/architecture/00-overview.md) | Architecture overview |
| [docs/env.md](docs/env.md) | Environment variables |

---

## License

MIT