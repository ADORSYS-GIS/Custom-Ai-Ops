# Custom-Ai-Ops — ML Model Serving Platform

A highly resilient, long-term, multi-format ML model serving platform with triple-layer separation, designed to serve millions of users with auto-repair, capacity forecasting, and multi-year durability.

## Architecture — Triple-Layer Separation

The system strictly decouples three planes to ensure modularity and long-term maintainability:

1. **Model Plane** — Interchangeable weights and formats (GGUF, Safetensors, ONNX, AWQ, GPTQ, TensorRT, PyTorch)
2. **Engine Plane** — Runtime containers per format (llama.cpp, vLLM, ONNX Runtime GenAI, Triton, Ray Serve)
3. **Exposure Plane** — Uniform OpenAI-compatible API via Envoy AI Gateway (FQDN, Hetzner)

This separation means you can add a new model without touching the gateway, change engines without touching the client, and switch between self-hosted and SaaS providers transparently.

## Format → Engine Decision Tree

| Format | Engine | Confidence | Helm Chart |
|---|---|---|---|
| GGUF | llama.cpp | 0.97 | model-serving-llamacpp |
| ONNX | ONNX Runtime GenAI | 0.95 | model-serving-onnx-rust |
| Safetensors | vLLM | 0.96 | model-serving-vllm |
| AWQ | vLLM | 0.94 | model-serving-vllm |
| GPTQ | vLLM | 0.93 | model-serving-vllm |
| TensorRT | Triton Inference Server | 0.98 | model-serving-triton |
| PyTorch | Ray Serve | 0.70 (transitional) | model-serving-rayserve |

This decision tree is codified in the `engine-selector` Rust CLI tool, not left to ad hoc human decisions.

## Repository Structure

```
Custom-Ai-Ops/
├── tools/                           # Rust CLI tools
│   ├── engine-selector/             # Format→engine decision tree (29 unit tests)
│   ├── vram-budget-calc/           # VRAM budget calculator (16 unit tests)
│   └── model-onboarding/           # New model scaffold tool (23 unit tests)
├── charts/                          # Helm charts
│   ├── bjw-template/               # Common base library chart (security, probes, volumes)
│   ├── model-serving-engine/       # Unified engine chart (vllm/llamacpp/onnxGenai)
│   ├── model-serving-llamacpp/     # GGUF/llama.cpp chart
│   ├── model-serving-vllm/         # Safetensors/vLLM chart
│   ├── model-serving-onnx-rust/   # ONNX Runtime GenAI chart
│   ├── model-serving-triton/       # Triton multi-format chart
│   ├── model-serving-rayserve/     # Transitional PyTorch/Ray Serve chart
│   └── ai-gateway/                 # Envoy AI Gateway + backends + models + pricing
├── environments/                    # Environment-specific configurations
│   ├── dev/                         # 1 replica, local-path 30Gi, autoscaling off
│   ├── staging/                     # 1-2 replicas, longhorn 50Gi, autoscaling on
│   └── prod/                        # 2-4 replicas, longhorn 100Gi, PDB, topology spread
├── apps/                            # ArgoCD ApplicationSets
│   ├── argocd-appset-prod.yaml     # Production: serving + infrastructure + secrets + gateway
│   ├── argocd-appset-staging.yaml   # Staging: serving + gateway
│   ├── argocd-appset-dev.yaml       # Dev: serving + gateway
│   └── argocd-health-checks.yaml   # Custom Lua health checks for ML CRDs
├── observability/                   # Monitoring and alerting
│   ├── envoy-gateway-config.yaml    # HTTPRoute + BackendTrafficPolicy + HealthCheckPolicy
│   ├── prometheus-anomaly-rules.yaml # Latency/error/GPU/pod/anomaly alert rules
│   ├── alertmanager-routes/         # Alert routing (PagerDuty, Slack)
│   └── grafana-dashboards/          # DCGM + model-serving dashboards
├── models/                          # Model registry and per-model documentation
│   ├── registry.yaml                # Declarative registry (4 models)
│   └── llama-3-8b-instruct/         # Example: model.md + budget.md + eval-report.md
├── tests/                           # Test suites
│   ├── smoke/                       # Post-deployment smoke tests (bash)
│   ├── load/                        # k6 load tests
│   └── chaos/                        # LitmusChaos GPU chaos scenarios
├── docs/                            # Documentation
│   ├── architecture/                # Architecture docs (00-07) + ADRs + runbooks
│   ├── hardware/                    # GPU reference guide
│   └── ...
├── .github/workflows/ci.yaml        # CI: rust-tools, helm-lint, registry-consistency, vram-validation
├── impl.md                          # Reference architecture document
├── tests.md                         # Certification test suite (11 categories)
├── namage.md                         # Production lifecycle document
├── solve.md                         # End-to-end toolchain method
├── LICENSE
└── README.md
```

## Quick Start

### 1. Build Rust CLI Tools

```bash
# Build all tools in the workspace
cargo build --release

# Or build individually
cargo build --release --bin engine-selector
cargo build --release --bin vram-budget-calc
cargo build --release --bin model-onboarding
```

### 2. Run Tests

```bash
# Run all unit tests (68 tests across 3 crates)
cargo test

# Run tests for a specific tool
cargo test --bin engine-selector
cargo test --bin vram-budget-calc
cargo test --bin model-onboarding
```

### 3. Use the Tools

```bash
# Select the best engine for a model
./target/release/engine-selector --model /path/to/model --json

# Override format detection
./target/release/engine-selector --model /path/to/model --format gguf

# Calculate VRAM budget
./target/release/vram-budget-calc \
  --total-vram 8 \
  --model-size 4.7 \
  --quant q4_km \
  --gpu "RTX A2000" \
  --batch 1 \
  --context 8192 \
  --layers 32 \
  --heads 32 \
  --json

# Onboard a new model (scaffolds files)
./target/release/model-onboarding --name my-model --format gguf --vram-budget 8 --gpu-pool "RTX A2000" --dry-run
```

### 4. Validate Helm Charts

```bash
# Lint all charts
helm lint charts/bjw-template
helm lint charts/model-serving-engine
helm lint charts/model-serving-llamacpp
helm lint charts/model-serving-vllm
helm lint charts/model-serving-onnx-rust
helm lint charts/model-serving-triton
helm lint charts/model-serving-rayserve
helm lint charts/ai-gateway

# Template dry-run
helm template charts/model-serving-engine --set model.name=test-model
```

### 5. Deploy via GitOps (ArgoCD)

```bash
# Apply ArgoCD ApplicationSets
kubectl apply -f apps/argocd-appset-dev.yaml
kubectl apply -f apps/argocd-appset-staging.yaml
kubectl apply -f apps/argocd-appset-prod.yaml
kubectl apply -f apps/argocd-health-checks.yaml
```

## Sync Waves

The GitOps pipeline manages deployments in ordered waves:

| Wave | Content | Justification |
|---|---|---|
| -3 | Bootstrap namespace, base secrets | Nothing can start without this |
| -2 | Storage (RWX PVC via Longhorn), seed jobs | Pods need ready volumes |
| -1 | NVIDIA GPU Operator, Prometheus collectors | Must run before workloads |
| 0 | Model server StatefulSets | The core of the system |
| 1 | Gateway configuration, Grafana dashboards | Depends on workloads |
| 2+ | Post-sync smoke tests, notifications | Final validation |

## Model Registry

The declarative registry (`models/registry.yaml`) tracks all models with their format, engine, status, VRAM budget, GPU pool, and context length:

| Model | Format | Engine | Status | VRAM | GPU | Quant |
|---|---|---|---|---|---|---|
| llama-3-8b-instruct | GGUF | llama.cpp | LIVE | 8 GB | RTX A2000 | q4_km |
| mistral-7b-instruct | Safetensors | vLLM | STAGED | 40 GB | A100 | bf16 |
| phi-3-mini-instruct | ONNX | ONNX GenAI | LIVE | 4 GB | L4 | int4 |
| llama-3-70b-instruct | Safetensors | vLLM | STANDBY | 80 GB | H100 | fp16 |

Each model has a dedicated directory with:
- `model.md` — Model datasheet (VRAM budget, status, context, fallback)
- `budget.md` — Detailed VRAM calculation (proven by `vram-budget-calc`)
- `eval-report.md` — Quality validation results (MMLU, HellaSwag, ARC, TruthfulQA, latency benchmarks)

## Observability

### Health Checking
- Active health-checking at the Envoy AI Gateway level (`/health` endpoint)
- Immediate failover to fallback backend if latency > 2000ms
- Priority routing (priority 0 → priority 1) with circuit breaker (Prioritized)
- Retry on 502/503/504 (2 attempts)

### Monitoring Stack (LGTM)
- **Prometheus** + **Mimir** (long-term metrics storage, multi-year retention)
- **Loki** (logs, low storage cost)
- **Tempo** + **OpenTelemetry** (distributed tracing)
- **Grafana** (unified dashboards)
- **DCGM Exporter** (NVIDIA GPU metrics: SM utilization, memory, temperature, ECC errors)

### Alerting
- **Latency**: HighLatency (p95 > 2s/3m warning), CriticalLatency (p99 > 5s/2m critical)
- **Errors**: HighErrorRate (>5%/5m), CriticalErrorRate (>15%/3m)
- **GPU**: Thermal throttle (>85°C), low utilization (<10%/30m), memory near exhaustion (>95%/2m), ECC errors (>100/h)
- **Pods**: CrashLooping (restarts > 3/h), NotReady (10m)
- **Anomaly**: LatencyAnomaly (deriv > 0.1/10m), ThroughputAnomaly (deriv < -0.5/10m)
- **Routing**: Critical → PagerDuty + Slack #ml-incidents, Warning → Slack #ml-ops, GPU → #gpu-ops

## CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/ci.yaml`) runs:

1. **Rust tools**: Build + test all 3 crates, clippy (deny warnings), fmt check
2. **Helm lint**: Lint all 8 charts + template dry-run with test values
3. **Registry consistency**: Validate each registry entry has a chart dir, model dir, and required files (model.md, budget.md, eval-report.md)
4. **VRAM budget validation**: Build `vram-budget-calc` and run it for all LIVE/STAGED models from the registry — CI fails if any budget is exceeded

## Test Suites

- **Smoke tests** (`tests/smoke/`): Health check (200), auth (401/403), chat completion (200 + content validation), cost metric presence
- **Load tests** (`tests/load/`): k6 with staged ramp-up (5→10→20→10→0 VUs), thresholds p95 < 2000ms, failed < 5%
- **Chaos engineering** (`tests/chaos/`): LitmusChaos experiments — pod-delete, network-latency, node-drain on GPU nodes

## Documentation

- **`impl.md`** — Reference architecture (triple-layer separation, format/engine mapping, GitOps pipeline, observability, auto-healing, multi-year robustness)
- **`tests.md`** — Certification test suite (11 categories, 48 tests, GO/NO-GO criteria)
- **`namage.md`** — Production lifecycle management
- **`solve.md`** — End-to-end toolchain method
- **`docs/architecture/`** — Architecture docs (overview, formats-and-engines, GPU scheduling, gateway federation, GitOps deployment, observability, resilience-and-DR, capacity-forecasting)
- **`docs/adr/`** — Architecture Decision Records (0001: multi-format architecture, 0002: Envoy AI Gateway federation, 0003: separate engine charts)
- **`docs/hardware/gpu.md`** — In-depth GPU reference guide (consumer/workstation/datacenter families, prefill vs decode, CUDA gap, per-GPU datasheets, microarchitecture comparison, runtimes, infrastructure constraints)
- **`docs/runbooks/`** — Incident runbooks (gpu-node-failure, latency-spike, pod-crashloop)

## Technology Stack

| Layer | Tool |
|---|---|
| Orchestration | Kubernetes (Talos / k3s) |
| GitOps | ArgoCD (ApplicationSets, custom Lua health checks) |
| GGUF engine | llama.cpp |
| Safetensors/AWQ/GPTQ engine | vLLM |
| ONNX engine | ONNX Runtime GenAI |
| Multi-format engine | Triton Inference Server |
| Transitional engine | Ray Serve |
| GPU scheduling | NVIDIA GPU Operator + Kueue + Volcano |
| Autoscaling | KEDA + HPA (custom metrics) |
| Node provisioning | Karpenter |
| API Gateway | Envoy AI Gateway (OpenAI-compatible) |
| Observability | Prometheus/Mimir + Loki + Tempo + Grafana + DCGM |
| Storage | Longhorn (RWX PVC) |
| Load testing | k6 |
| Chaos engineering | LitmusChaos |

## License

MIT License