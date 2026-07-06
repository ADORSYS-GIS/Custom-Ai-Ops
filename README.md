<div align="center">

# Custom-Ai-Ops

### Cloud-Scale Multi-Format ML Model Serving Platform

A highly resilient, long-term, multi-format ML model serving platform with triple-layer separation, designed to serve millions of users with auto-repair, capacity forecasting, and multi-year durability.

---

![License](https://img.shields.io/badge/License-MIT-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?logo=rust)
![Tests](https://img.shields.io/badge/Tests-68%20passing-brightgreen.svg)
![Charts](https://img.shields.io/badge/Helm%20Charts-5-blue.svg?logo=helm)

---

#### Orchestration & GitOps

![Kubernetes](https://img.shields.io/badge/Kubernetes-1.28+-326CE5.svg?logo=kubernetes)
![ArgoCD](https://img.shields.io/badge/ArgoCD-2.8+-EF7B4D.svg?logo=argo)
![Helm](https://img.shields.io/badge/Helm-3.14+-0F1689.svg?logo=helm)
![Talos](https://img.shields.io/badge/Talos-1.7+-607078.svg)
![Karpenter](https://img.shields.io/badge/Karpenter-0.37+-326CE5.svg)
![Kueue](https://img.shields.io/badge/Kueue-0.6+-326CE5.svg)
![Volcano](https://img.shields.io/badge/Volcano-1.9+-326CE5.svg)
![KEDA](https://img.shields.io/badge/KEDA-2.14+-326CE5.svg)

#### Inference Engines

![vLLM](https://img.shields.io/badge/vLLM-0.6.3-blue.svg)
![ONNX Runtime GenAI](https://img.shields.io/badge/ONNX%20Runtime%20GenAI-latest-purple.svg)

#### GPU & Hardware

![NVIDIA GPU Operator](https://img.shields.io/badge/NVIDIA%20GPU%20Operator-24.9+-76B900.svg?logo=nvidia)
![DCGM Exporter](https://img.shields.io/badge/DCGM%20Exporter-3.3+-76B900.svg?logo=nvidia)
![CUDA](https://img.shields.io/badge/CUDA-12.4+-76B900.svg?logo=nvidia)

#### Gateway & API

![Envoy AI Gateway](https://img.shields.io/badge/Envoy%20AI%20Gateway-latest-AC6191.svg?logo=envoy)
![OpenAI Compatible](https://img.shields.io/badge/OpenAI%20Compatible-API-41299E.svg)

#### Observability (LGTM Stack)

![Prometheus](https://img.shields.io/badge/Prometheus-2.50+-E6522C.svg?logo=prometheus)
![Grafana](https://img.shields.io/badge/Grafana-10.4+-F46800.svg?logo=grafana)
![Loki](https://img.shields.io/badge/Loki-3.0+-F8A900.svg?logo=grafana)
![Tempo](https://img.shields.io/badge/Tempo-2.5+-29D4A8.svg?logo=grafana)
![Mimir](https://img.shields.io/badge/Mimir-2.12+-F8A900.svg?logo=grafana)
![OpenTelemetry](https://img.shields.io/badge/OpenTelemetry-1.0+-425EEA.svg?logo=opentelemetry)
![Alertmanager](https://img.shields.io/badge/Alertmanager-0.27+-E6522C.svg?logo=prometheus)

#### Storage & Registry

![Longhorn](https://img.shields.io/badge/Longhorn-1.6+-0F1689.svg?logo=helm)
![MinIO](https://img.shields.io/badge/MinIO-latest-C72E49.svg?logo=minio)
![Harbor](https://img.shields.io/badge/Harbor-2.10+-60B8E6.svg)
![MLflow](https://img.shields.io/badge/MLflow-2.12+-0CCE0C.svg)

#### Security & Supply Chain

![cosign](https://img.shields.io/badge/cosign-2.2+-76B900.svg)
![External Secrets Operator](https://img.shields.io/badge/External%20Secrets-0.9+-326CE5.svg)
![gitleaks](https://img.shields.io/badge/gitleaks-8.0+-FF4F4F.svg)

#### Testing & CI/CD

![GitHub Actions](https://img.shields.io/badge/GitHub%20Actions-CI/CD-2088FF.svg?logo=githubactions)
![k6](https://img.shields.io/badge/k6-0.50+-7D64FF.svg)
![LitmusChaos](https://img.shields.io/badge/LitmusChaos-3.0+-8B5CF6.svg)
![kubeconform](https://img.shields.io/badge/kubeconform-0.6+-326CE5.svg)

#### Languages

![Rust](https://img.shields.io/badge/Rust-Workspace-orange.svg?logo=rust)
![YAML](https://img.shields.io/badge/YAML-Helm%20Charts-CC1018.svg)
![Python](https://img.shields.io/badge/Python-CI%20Scripts-3776AB.svg?logo=python)
![Bash](https://img.shields.io/badge/Bash-Smoke%20Tests-4EAA25.svg?logo=gnubash)

</div>

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [High-Level Architecture Diagram](#high-level-architecture-diagram)
- [Request Flow Diagram](#request-flow-diagram)
- [GitOps Deployment Pipeline](#gitops-deployment-pipeline)
- [Observability Stack Diagram](#observability-stack-diagram)
- [Infrastructure Topology](#infrastructure-topology)
- [Model Onboarding Pipeline](#model-onboarding-pipeline)
- [Auto-Healing Layers](#auto-healing-layers)
- [Format → Engine Decision Tree](#format--engine-decision-tree)
- [Repository Structure](#repository-structure)
- [Quick Start](#quick-start)
- [Sync Waves](#sync-waves)
- [Model Registry](#model-registry)
- [KV Cache Management](#kv-cache-management)
- [Observability](#observability)
- [CI/CD Pipeline](#cicd-pipeline)
- [Test Suites](#test-suites)
- [Documentation](#documentation)
- [Technology Stack](#technology-stack)
- [License](#license)

---

## Architecture Overview

The platform is built on a **triple-layer separation** principle that ensures maximum modularity and long-term maintainability. The key insight: **never rigidly couple the model format to the serving engine**.

### The Three Planes

```mermaid
graph TB
    subgraph Exposure["EXPOSURE PLANE (Uniform API)"]
        GW["Envoy AI Gateway<br/>OpenAI-Compatible API (/v1/chat/completions)"]
        GW_FEAT["HTTPRoute · Auth (APIKey) · Rate Limiting<br/>Cost Metrics · SSE Streaming"]
        GW_RESIL["Priority Routing (0→1) · Circuit Breaker<br/>Retry (502/503/504) · Failover (>2000ms)"]
        GW --- GW_FEAT
        GW --- GW_RESIL
    end

    subgraph Engine["ENGINE PLANE (Runtime per Format)"]
        VLLM["vLLM<br/>port 8000"]
        ONNX["ONNX RT GenAI<br/>port 8080"]
    end

    subgraph Model["MODEL PLANE (Interchangeable Weights)"]
        SAFE["Safetensors<br/>BF16/FP16"]
        ONNXW["ONNX<br/>INT4/INT8"]
        AWQ["AWQ/GPTQ"]
    end

    subgraph Storage["Storage"]
        MINIO["MinIO<br/>(S3-compatible)"]
        PVC["RWX PVC<br/>(Longhorn)"]
        MOUNT["/models/&lt;name&gt;/"]
        MINIO -->|seed| PVC
        PVC -->|mount| MOUNT
    end

    Exposure ==> Engine
    Engine ==> Model
    Model -.->|weights loaded from| Storage
```

**Why this matters:**
- Add a new model → no gateway change needed
- Change engine → no client-side change needed
- Switch to SaaS fallback → transparent to end users
- Each plane evolves independently over years

---

## High-Level Architecture Diagram

```mermaid
graph TB
    GIT["Git Repository<br/>charts/ environments/ models/ apps/<br/>tools/ observability/ tests/ docs/"]
    CI["GitHub Actions CI<br/>Rust build+test · Helm lint (5)<br/>Registry check · VRAM validation"]
    ARGOCD["ArgoCD Control (Control Cluster)<br/>ApplicationSets: model-serving, ai-gateway,<br/>infrastructure (GPU/LH/Prom), secrets (ESO)<br/>Custom Lua Health: StatefulSet, InferenceService"]

    GIT -->|push| CI
    CI -->|if pass| ARGOCD

    ARGOCD -->|Sync Waves -3 → 2| WA
    ARGOCD -->|Sync Waves -3 → 2| WB
    ARGOCD -->|Sync Waves -3 → 2| WDEV

    subgraph WA["Worker Cluster (Region A)"]
        WA_WAVES["Waves: -3 Secrets → -2 Longhorn PVC → -1 GPU Operator<br/>→ 0 Model Pods → 1 Gateway+Dashboards → 2 Smoke Tests"]
        WA_POOLS["Node Pools: gpu-h100-pool · gpu-a100-pool<br/>gpu-l4-pool · cpu-pool"]
    end

    subgraph WB["Worker Cluster (Region B)"]
        WB_WAVES["Waves: -3 Secrets → -2 Longhorn PVC → -1 GPU Operator<br/>→ 0 Model Pods → 1 Gateway+Dashboards → 2 Smoke Tests"]
        WB_POOLS["Node Pools: gpu-h100-pool · gpu-a100-pool<br/>gpu-l4-pool · cpu-pool"]
    end

    subgraph WDEV["Worker Cluster (Edge / Dev)"]
        WDEV_WAVES["Waves: -3 Secrets → -2 local-path → -1 GPU Operator<br/>→ 0 Model Pods → 1 Gateway+Dashboards → 2 Smoke Tests"]
        WDEV_POOLS["Node Pools: gpu-edge-pool (A2000)<br/>gpu-l4-pool (L4) · cpu-pool"]
    end
```

---

## Request Flow Diagram

```mermaid
sequenceDiagram
    participant C as Client (SDK / curl)
    participant GW as Envoy AI Gateway
    participant CB as Circuit Breaker (Prioritized)
    participant V as vLLM :8000
    participant O as ONNX RT GenAI :8080
    participant S as /models/&lt;name&gt;/ (RWX PVC via Longhorn)

    C->>GW: POST /v1/chat/completions<br/>Authorization: Bearer &lt;key&gt;
    GW->>CB: HTTPRoute → BackendTrafficPolicy
    CB->>CB: priority 0 → 1, retry 502/503/504
    CB->>CB: Health Check GET /health (10s interval)
    CB->>CB: Failover if latency > 2000ms → priority 1 (SaaS fallback)

    alt Safetensors model
        CB->>V: route to vLLM
        V->>S: load weights
        V-->>GW: SSE stream
    else ONNX model
        CB->>O: route to ONNX RT GenAI
        O->>S: load weights
        O-->>GW: SSE stream
    end

    GW-->>C: SSE stream { "choices": [...] }
```

---

## GitOps Deployment Pipeline

```mermaid
graph LR
    DEV["Developer"]
    GH["GitHub<br/>(webhook)"]
    CI_RUST["rust-tools<br/>build+test<br/>clippy -D · fmt"]
    CI_HELM["helm-lint (5)<br/>lint --strict<br/>template"]
    CI_REG["registry<br/>consistency"]
    CI_VRAM["vram-budget<br/>validation"]
    ARGOCD["ArgoCD<br/>Self-Heal: ON · Prune: ON<br/>ServerSideApply"]
    K8S["Kubernetes"]

    DEV -->|git push| GH
    GH -->|webhook| CI_RUST
    CI_RUST --> CI_HELM
    CI_HELM --> CI_REG
    CI_REG --> CI_VRAM

    CI_VRAM -->|pass| ARGOCD
    GH -.->|if CI fails| DEV

    ARGOCD -->|sync waves| K8S

    subgraph Waves["Sync Waves"]
        W3["Wave -3: Secrets"]
        W2["Wave -2: PVC / Longhorn"]
        W1["Wave -1: GPU Operator / Prometheus"]
        W0["Wave 0: StatefulSet (Model Pods)"]
        W1P["Wave 1: Gateway / Dashboards"]
        W2P["Wave 2: Smoke Tests"]
        W3 --> W2 --> W1 --> W0 --> W1P --> W2P
    end

    K8S --- Waves
```

---

## Observability Stack Diagram

```mermaid
graph TB
    subgraph Cluster["Kubernetes Worker Cluster"]
        MP["Model Pods<br/>/metrics"]
        DCGM["GPU Nodes<br/>DCGM Exporter"]
        GW["Envoy GW<br/>/metrics"]
        GP["Gateway Pods"]
        NE["Node Exporter"]
        ALLOY["Grafana Alloy<br/>(agent: metrics + logs + traces)"]
        MP --> ALLOY
        DCGM --> ALLOY
        GW --> ALLOY
        GP --> ALLOY
        NE --> ALLOY
    end

    ALLOY -->|metrics| PROM["Prometheus + Mimir<br/>(2yr retention)"]
    ALLOY -->|logs| LOKI["Loki<br/>(logs)"]
    ALLOY -->|traces| TEMPO["Tempo<br/>(traces)"]

    PROM --> GRAFANA["Grafana"]
    LOKI --> GRAFANA
    TEMPO --> GRAFANA

    GRAFANA --> DASH1["Dashboard: DCGM (GPU health)"]
    GRAFANA --> DASH2["Dashboard: Model Serving<br/>(latency/err/throughput)"]
    GRAFANA --> DASH3["Dashboard: Capacity Forecasting"]

    subgraph Alerting["Alerting"]
        RULES["PrometheusRule (6 groups)<br/>latency · errors · gpu · pods · anomaly · kv-cache"]
        AM["Alertmanager Routes"]
        RULES --> AM
        AM -->|critical| PD["PagerDuty + Slack #ml-incidents"]
        AM -->|warning| SL1["Slack #ml-ops"]
        AM -->|gpu| SL2["Slack #gpu-ops"]
        AM -->|serving| SL3["Slack #ml-ops"]
    end

    PROM -.->|alerts| Alerting
```

---

## Infrastructure Topology

```mermaid
graph TB
    subgraph Control["Control Cluster (no GPU workloads)"]
        ARGOCD["ArgoCD Server + AppSet Controller"]
        REPO["ArgoCD Repo Server (Git access)"]
        REDIS["ArgoCD Redis (cache)"]
        LUA["Custom Lua Health Checks (argocd-cm)"]
        ESO["External Secrets Operator<br/>(pulls from Vault/AWS SM)"]
    end

    Control -->|GitOps Sync| Prod
    Control -->|GitOps Sync| Staging
    Control -->|GitOps Sync| Dev

    subgraph Prod["Worker Cluster (Production)"]
        P1["gpu-h100-pool<br/>H100 80GB · taint: gpu"]
        P2["gpu-a100-pool<br/>A100 40GB · taint: gpu"]
        P3["gpu-l4-pool<br/>L4 24GB"]
        P4["cpu-pool<br/>(no GPU)"]
        P_STOR["Storage: longhorn 100Gi<br/>2-4 replicas · autoscaling: on<br/>PDB: minAvail 1 · topology spread"]
    end

    subgraph Staging["Worker Cluster (Staging)"]
        S1["gpu-a100-pool<br/>A100 40GB · taint: gpu"]
        S2["gpu-l4-pool<br/>L4 24GB"]
        S3["cpu-pool<br/>(no GPU)"]
        S_STOR["Storage: longhorn 50Gi<br/>1-2 replicas · autoscaling: on<br/>PDB: on"]
    end

    subgraph Dev["Worker Cluster (Dev/Edge)"]
        D1["gpu-edge-pool<br/>A2000 8GB · taint: gpu"]
        D2["cpu-pool<br/>(no GPU)"]
        D_STOR["Storage: local-path 30Gi<br/>1 replica · autoscaling: off<br/>PDB: off"]
    end

    subgraph Scheduling["GPU Scheduling"]
        KUEUE["Kueue (quotas)"]
        VOLCANO["Volcano (gang scheduling)"]
        KARPENTER["Karpenter (node provisioning)"]
        GPUOP["NVIDIA GPU Operator<br/>(driver + DCGM + device plugin + toolkit)"]
        KUEUE --> VOLCANO --> KARPENTER
    end
```

---

## Model Onboarding Pipeline

```mermaid
graph LR
    S1["1. Identify Format<br/>Safetensors? ONNX?<br/>AWQ/GPTQ?"]
    S2["2. engine-selector<br/>Detects format → engine<br/>→ chart → confidence"]
    S3["3. vram-budget-calc<br/>VRAM = Total×0.90<br/>− model size − 1GB − KV cache<br/>FP8 check · BLOCK if &lt; 0"]
    S4["4. model-onboarding<br/>Scaffolds models/&lt;name&gt;/<br/>model.md · budget.md · eval-report.md"]
    S5["5. Generate Gateway Entry<br/>backends + models<br/>in ai-gateway/values.yaml"]
    S6["6. Open PR<br/>(values repo)"]
    S7["7. CI Validation<br/>helm lint --strict · helm template<br/>registry consistency · vram validation"]
    S8["8. ArgoCD Sync<br/>Waves -3 → 2<br/>self-heal ON · prune ON"]
    S9["9. Smoke Tests<br/>health 200 · auth 401/403<br/>chat completion · cost metric"]
    S10["10. Canary<br/>gateway priority 0<br/>canary → ramp-up"]
    S11["11. Full Traffic<br/>normal priority<br/>validate on real traffic"]
    S12["12. Document ADR<br/>(if new pattern)"]

    S1 --> S2 --> S3 --> S4 --> S5 --> S6
    S6 --> S7 --> S8 --> S9 --> S10 --> S11 --> S12
```

---

## Auto-Healing Layers

```mermaid
graph TB
    subgraph Layer1["Level 1 — Pod Level (Kubernetes native)"]
        L1A["Liveness probe fails<br/>→ Kubernetes restarts pod"]
        L1B["Startup probe (long timeout)<br/>prevents kill during model loading"]
    end

    subgraph Layer2["Level 2 — GPU Node Level (NVIDIA GPU Operator)"]
        L2A["NVIDIA Xid error detected<br/>→ GPU Operator cordons + drains node"]
        L2B["Pods migrate to healthy nodes<br/>→ Karpenter provisions replacement node"]
    end

    subgraph Layer3["Level 3 — Config Drift (ArgoCD self-healing)"]
        L3A["Manual kubectl edit<br/>→ ArgoCD detects drift"]
        L3B["Auto-re-syncs to Git state<br/>Correction in < 3 minutes"]
    end

    subgraph Layer4["Level 4 — Model Quality (Envoy AI Gateway)"]
        L4A["Latency > 2000ms or errors > 5%<br/>→ Gateway circuit breaker triggers"]
        L4B["Failover to SaaS fallback (priority 1)<br/>→ users unaffected"]
    end

    subgraph Layer5["Level 5 — Cluster Failover (External DNS + Envoy)"]
        L5A["Worker cluster unavailable<br/>→ DNS-based failover to another region"]
        L5B["Gateway multi-backend with priority routing<br/>handles transparently"]
    end

    subgraph Layer6["Level 6 — Data Drift (Evidently AI)"]
        L6A["Model quality degrades silently<br/>→ Evidently AI detects distribution shift"]
        L6B["Alert triggered<br/>→ re-evaluation pipeline started"]
    end

    Layer1 -->|"if pod restart doesn't fix it"| Layer2
    Layer2 -->|"if node-level fails"| Layer3
    Layer3 -->|"if model quality degrades"| Layer4
    Layer4 -->|"if entire cluster fails"| Layer5
    Layer5 -->|"if data drift detected"| Layer6

    Principle["KEY PRINCIPLE: Every automated action leaves a Git trace for auditability<br/>(so 2 years later, we can understand why a rollback happened without log archaeology)"]
    Layer6 -.-> Principle
```

---

## Format → Engine Decision Tree

```mermaid
graph TD
    Start["Model format?"] --> ONNX{"ONNX?"}
ONNX -->|"Yes"| ONNXRT["ONNX Runtime GenAI<br/>confidence: 0.95<br/>chart: model-serving-engine (onnxGenai)"]
    ONNX -->|"No"| ST{"Safetensors / BF16?"}
    ST -->|"Yes"| VLLM1["vLLM<br/>confidence: 0.96<br/>chart: model-serving-engine (vllm)"]
    ST -->|"No"| AWQ{"AWQ quantized?"}
    AWQ -->|"Yes"| VLLM2["vLLM<br/>confidence: 0.94<br/>chart: model-serving-engine (vllm)"]
    GPTQ -->|"Yes"| VLLM3["vLLM<br/>confidence: 0.93<br/>chart: model-serving-engine (vllm)"]
    GPTQ -->|"No"| UNSUPPORTED["Unsupported format<br/>(convert to Safetensors/ONNX/AWQ/GPTQ)"]
```

| Format | Engine | Confidence | Helm Chart | Port |
|---|---|---|---|---|
| ONNX | ONNX Runtime GenAI | 0.95 | model-serving-engine (onnxGenai) | 8080 |
| Safetensors | vLLM | 0.96 | model-serving-engine (vllm) | 8000 |
| AWQ | vLLM | 0.94 | model-serving-engine (vllm) | 8000 |
| GPTQ | vLLM | 0.93 | model-serving-engine (vllm) | 8000 |

This decision tree is codified in the `engine-selector` Rust CLI tool — not left to ad hoc human decisions.

---

## Repository Structure

```
Custom-Ai-Ops/
├── tools/                           # Rust CLI tools (workspace)
│   ├── engine-selector/             # Format→engine decision tree (29 unit tests)
│   ├── vram-budget-calc/           # VRAM budget calculator (16 unit tests)
│   └── model-onboarding/           # New model scaffold tool (23 unit tests)
│
├── charts/                          # Helm charts (5 total)
│   ├── bjw-template/               # Common base library chart
│   │                               # (security context, probes, volumes, tolerations)
│   ├── model-serving-engine/       # Unified engine chart (vllm/onnxGenai)
    │   │                               # (StatefulSet, KEDA ScaledObject, PDB, NetworkPolicy,
    │   │                               #  PVC, seed-job, swapoff DaemonSet, ServiceMonitor)
    │   ├── model-serving-vllm/         # Safetensors/vLLM chart (appVersion 0.6.3) [DEPRECATED]
│   ├── model-serving-onnx-rust/   # ONNX Runtime GenAI chart
│   └── ai-gateway/                 # Envoy AI Gateway (HTTPRoute, BackendTrafficPolicy,
                                    #  rate limiting, payload validation, sticky routing, secrets)
│
├── environments/                    # Environment-specific configurations
│   ├── dev/                         # 1 replica, local-path 30Gi, autoscaling off, PDB off
│   ├── staging/                     # 1-2 replicas, longhorn 50Gi, autoscaling on
│   └── prod/                        # 2-4 replicas, longhorn 100Gi, PDB, topology spread
│
├── apps/                            # ArgoCD ApplicationSets
│   ├── argocd-appset-prod.yaml     # Prod: serving + infrastructure + secrets + gateway
│   ├── argocd-appset-staging.yaml   # Staging: serving + gateway
│   ├── argocd-appset-dev.yaml       # Dev: serving + gateway
│   └── argocd-health-checks.yaml   # Custom Lua health checks (StatefulSet, InferenceService)
│
├── observability/                   # Monitoring and alerting
│   ├── envoy-gateway-config.yaml    # HTTPRoute + BackendTrafficPolicy + HealthCheckPolicy
│   ├── prometheus-anomaly-rules.yaml # 6 rule groups: latency, errors, GPU, pods, anomaly, kv-cache
│   ├── alertmanager-routes/         # Alert routing: critical→PagerDuty+Slack, warning→Slack
│   └── grafana-dashboards/          # DCGM dashboard + model-serving dashboard
│
├── models/                          # Model registry and per-model documentation
│   ├── registry.yaml                # Declarative registry (4 models)
│   └── llama-3-8b-instruct/         # Example model: model.md + budget.md + eval-report.md
│
├── tests/                           # Test suites
│   ├── smoke/                       # Post-deployment smoke tests (bash: health, auth, chat, cost)
│   ├── load/                        # k6 load tests (staged ramp-up, p95 < 2000ms)
│   └── chaos/                        # LitmusChaos GPU chaos (pod-delete, network-latency, node-drain)
│
├── docs/                            # Documentation
│   ├── architecture/                # 8 architecture docs (00-07)
│   │   ├── 00-overview.md           #   Three-plane architecture overview
│   │   ├── 01-formats-and-engines.md #   Format-to-engine mapping + decision tree
│   │   ├── 02-gpu-scheduling.md     #   Node pools, VRAM formula, hardware constraints
│   │   ├── 03-gateway-federation.md #   Priority routing, health checks, failover
│   │   ├── 04-gitops-deployment.md  #   Sync waves, ArgoCD AppSet, Lua health checks
│   │   ├── 05-observability.md      #   LGTM stack, dashboards, anomaly detection
│   │   ├── 06-resilience-and-dr.md  #   Auto-healing layers, rollback strategy
│   │   └── 07-capacity-forecasting.md # Holt-Winters, KEDA predictive, recording rules
│   ├── adr/                         # Architecture Decision Records
│   │   ├── 0001-multi-format-architecture.md
│   │   ├── 0002-envoy-ai-gateway.md
│   │   └── 0003-separate-engine-charts.md
│   ├── hardware/
│   │   └── gpu.md                   # In-depth GPU reference guide (348 lines)
│   └── runbooks/                    # Incident response procedures
│       ├── gpu-node-failure.md      #   Cordon/drain, ECC/Xid/temp checks
│       ├── latency-spike.md         #   Check failover, GPU throttle, scale up
│       └── pod-crashloop.md         #   OOM/model-not-found/probe-failure
│
├── .github/workflows/ci.yaml        # CI: rust-tools, helm-lint, registry-consistency, vram-validation
│
├── impl.md                          # Reference architecture document
├── tests.md                         # Certification test suite (11 categories, 48 tests)
├── namage.md                        # Production lifecycle management
├── solve.md                         # End-to-end toolchain method
├── LICENSE                          # MIT License
└── README.md                        # This file
```

---

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
cargo test --bin engine-selector    # 29 tests
cargo test --bin vram-budget-calc   # 16 tests
cargo test --bin model-onboarding   # 23 tests
```

### 3. Use the Tools

```bash
# Select the best engine for a model
./target/release/engine-selector --model /path/to/model --json

# Override format detection
./target/release/engine-selector --model /path/to/model --format onnx

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
./target/release/model-onboarding \
  --name my-model \
  --format safetensors \
  --vram-budget 8 \
  --gpu-pool "RTX A2000" \
  --dry-run
```

### 4. Validate Helm Charts

```bash
# Lint all charts
helm lint charts/bjw-template
helm lint charts/model-serving-engine
helm lint charts/model-serving-vllm
helm lint charts/model-serving-onnx-rust
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

---

## Sync Waves

The GitOps pipeline manages deployments in ordered waves — each wave must reach "Healthy" before the next starts:

| Wave | Content | Justification |
|---|---|---|
| -3 | Bootstrap namespace, base secrets | Nothing can start without this |
| -2 | Storage (RWX PVC via Longhorn), seed jobs | Pods need ready volumes |
| -1 | NVIDIA GPU Operator, Prometheus collectors | Must run before workloads to capture metrics |
| 0 | Model server StatefulSets | The core of the system |
| 1 | Gateway configuration, Grafana dashboards | Depends on workloads being in place |
| 2+ | Post-sync smoke tests, notifications | Final validation |

---

## Model Registry

The declarative registry (`models/registry.yaml`) tracks all models with their format, engine, status, VRAM budget, GPU pool, and context length:

| Model | Format | Engine | Status | VRAM | GPU | Quant | Context |
|---|---|---|---|---|---|---|---|
| mistral-7b-instruct | Safetensors | vLLM | STAGED | 40 GB | A100 | bf16 | 32768 |
| phi-3-mini-instruct | ONNX | ONNX GenAI | LIVE | 4 GB | L4 | int4 | 4096 |
| llama-3-70b-instruct | Safetensors | vLLM | STANDBY | 80 GB | H100 | fp16 | 8192 |

Each model has a dedicated directory with:
- **`model.md`** — Model datasheet (VRAM budget, status, context, fallback model)
- **`budget.md`** — Detailed VRAM calculation (proven by `vram-budget-calc`)
- **`eval-report.md`** — Quality validation results (MMLU, HellaSwag, ARC, TruthfulQA, latency benchmarks)

### VRAM Budget Formula

```
Usable VRAM     = Total VRAM × 0.90
Available       = Usable VRAM − Model Size − 1 GB Fixed Overhead − KV Cache
KV Cache        = 2 × Batch × Context × Layers × Heads × Bytes-per-weight / 1024³

If Available < 0  →  deployment BLOCKED by vram-budget-calc in CI
If FP8 on Ampere  →  deployment BLOCKED (no native FP8 support)
```

---

## Observability

### Health Checking

- Active health-checking at the Envoy AI Gateway level (`GET /health` endpoint)
- Immediate failover to fallback backend if latency > 2000ms
- Priority routing (priority 0 → priority 1) with circuit breaker (Prioritized)
- Retry on 502/503/504 (2 attempts)
- **Rate limiting**: 50 req/s per API key (HTTP 429 on excess) — protects KV cache from request floods
- **Payload validation**: max body size 4MiB (HTTP 413), required fields enforced (HTTP 400)
- **Sticky routing**: `x-sticky-session-key` header routes same-prefix requests to same replica (maximizes prefix cache hits)
- **Aggressive timeouts**: request 10s / backendRequest 8s — prevents KV cache thrashing from queued requests

### Monitoring Stack (LGTM)

| Layer | Tool | Purpose |
|---|---|---|
| Metrics | Prometheus + Mimir | Long-term metric storage (2-year retention) |
| Logs | Loki | Log aggregation (low storage cost) |
| Traces | Tempo + OpenTelemetry | Distributed tracing |
| Dashboards | Grafana | Unified visualization |
| GPU Metrics | DCGM Exporter | GPU utilization, memory, temperature, ECC errors |
| vLLM Metrics | ServiceMonitor | Scrapes vLLM `/metrics` every 10s (KV cache, queue, prefix cache, TTFT) |
| Collection | Grafana Alloy | Single agent for metrics + logs + traces |

### Grafana Dashboards

| Dashboard | Panels |
|---|---|
| `dcgm-dashboard.json` | GPU health (temperature, utilization, memory, ECC) |
| `model-serving-dashboard.json` | Request rate, P95 latency, error rate, tokens/s, OOM kills, **KV cache usage (%)**, **prefix cache hit rate (%)**, **request queue depth**, **TTFT (p95+p50)**, **KV cache swap-out blocks**, **GPU VRAM usage (DCGM)** |

### Alerting Rules

| Category | Alert | Condition | Severity |
|---|---|---|---|
| Latency | HighLatency | p95 > 2s for 3m | Warning |
| Latency | CriticalLatency | p99 > 5s for 2m | Critical |
| Errors | HighErrorRate | > 5% for 5m | Warning |
| Errors | CriticalErrorRate | > 15% for 3m | Critical |
| GPU | GPUThermalThrottle | > 85°C | Critical |
| GPU | GPUUtilizationLow | < 10% for 30m | Warning |
| GPU | GPUMemoryNearExhaustion | > 95% for 2m | Critical |
| GPU | GPUEccErrors | > 100/h | Critical |
| Pods | CrashLooping | restarts > 3/h | Warning |
| Pods | NotReady | 10m | Warning |
| Anomaly | LatencyAnomaly | deriv > 0.1 for 10m | Warning |
| Anomaly | ThroughputAnomaly | deriv < -0.5 for 10m | Warning |
| KV Cache | VLLMKVCacheUsageHigh | `vllm:gpu_cache_usage_perc` > 0.85 for 30s | Warning |
| KV Cache | VLLMKVCacheUsageCritical | `vllm:gpu_cache_usage_perc` >= 1.0 | Critical |
| KV Cache | VLLMRequestsWaitingHigh | `vllm:num_requests_waiting` > 10 for 1m | Critical |
| KV Cache | VLLMSwapOutBlocksDetected | `increase(vllm:swap_out_blocks[5m])` > 0 | Critical |
| KV Cache | NodeSwapSpaceUsageHigh | swap usage > 10% for 2m | Critical |
| KV Cache | VLLMPrefixCacheHitRateLow | prefix cache hit < 20% for 10m | Warning |

### Alert Routing

- **Critical** → PagerDuty + Slack `#ml-incidents`
- **Warning** → Slack `#ml-ops`
- **GPU** → Slack `#gpu-ops`
- **Serving** → Slack `#ml-ops`
- Inhibit: critical suppresses warning for same alert

---

## KV Cache Management

The platform implements a **6-layer defensive architecture** for vLLM KV cache management, as documented in [`docs/explain/kv-cache.md`](docs/explain/kv-cache.md). Each layer protects the KV cache from a different failure mode.

### Layer 1 — API Gateway (Edge Protection)

| Mechanism | Implementation | Failure Mode Prevented |
|---|---|---|
| Payload validation | `HTTPRouteFilter` maxBodySize 4MiB → HTTP 413 | Oversized payloads polluting KV cache |
| Rate limiting | `BackendTrafficPolicy` 50 req/s per `x-api-key` → HTTP 429 | Request floods overwhelming KV cache |
| Sticky routing | `x-sticky-session-key` header → same replica | Prefix cache misses from random routing |
| Aggressive timeouts | request 10s / backendRequest 8s | Queue thrashing from slow requests |

### Layer 2 — vLLM Engine (Cache Efficiency)

| Argument | Prod | Staging | Dev | Purpose |
|---|---|---|---|---|
| `--gpu-memory-utilization` | 0.90 | 0.88 | 0.85 | Reserve headroom for KV cache growth |
| `--max-model-len` | 8192 | 8192 | 4096 | Cap context length to business need |
| `--max-num-seqs` | 256 | 128 | 64 | Limit concurrent sequences in KV cache |
| `--kv-cache-dtype` | fp8 | fp8 | fp8 | Halve KV cache memory via quantization |
| `--enable-prefix-caching` | ✓ | ✓ | ✓ | Reuse KV cache for shared prefixes |
| `--block-size` | 16 | 16 | 16 | Optimal block size for paged attention |
| `--tensor-parallel-size` | 1 | 1 | 1 | Per-NVLink topology |

### Layer 3 — Kubernetes (Resource Protection)

| Mechanism | Implementation | Failure Mode Prevented |
|---|---|---|
| **QoS Guaranteed** | requests == limits (CPU/RAM/GPU) in all envs | Host OOM killer evicting vLLM pods |
| **swapoff DaemonSet** | `nsenter swapoff -a` on GPU nodes via DaemonSet | Host swapping KV cache pages to CPU RAM |
| **Node isolation** | `nodeSelector: nvidia.com/gpu.present: "true"` | CPU workloads competing for GPU node RAM |

### Layer 4 — Observability (Early Detection)

| Alert | Condition | Severity |
|---|---|---|
| `VLLMKVCacheUsageHigh` | `vllm:gpu_cache_usage_perc` > 0.85 for 30s | Warning |
| `VLLMKVCacheUsageCritical` | `vllm:gpu_cache_usage_perc` >= 1.0 | Critical |
| `VLLMRequestsWaitingHigh` | `vllm:num_requests_waiting` > 10 for 1m | Critical |
| `VLLMSwapOutBlocksDetected` | `increase(vllm:swap_out_blocks[5m])` > 0 | Critical |
| `NodeSwapSpaceUsageHigh` | swap usage > 10% for 2m | Critical |
| `VLLMPrefixCacheHitRateLow` | hit rate < 20% for 10m | Warning |

**ServiceMonitor** scrapes vLLM `/metrics` every 10s with `honorLabels: true`.

### Layer 5 — Autoscaling (KEDA)

Classic CPU/RAM HPA is **inoperant for LLM workloads** (GPU-bound, not CPU-bound). The platform uses a KEDA `ScaledObject` with two Prometheus triggers:

| Trigger | Metric | Threshold | Action |
|---|---|---|---|
| Queue depth | `vllm:num_requests_waiting` | > 5 | Scale out |
| Cache pressure | `vllm:gpu_cache_usage_perc` | > 0.85 | Scale out |

- `minReplicaCount`: 2 (prod), `maxReplicaCount`: 4
- `pollingInterval`: 15s, `cooldownPeriod`: 60s
- Legacy HPA fallback retained for environments without KEDA

### Layer 6 — GitOps (Change Safety)

- All critical vLLM params centralized in `environments/{dev,staging,prod}/values.yaml`
- ArgoCD sync waves with self-heal + prune + ServerSideApply
- `vram-budget-calc` CI gate blocks deployment if KV cache budget < 0
- k6 load tests validate before changes reach production
- Staging environment uses identical GPU hardware to prod

---

## CI/CD Pipeline

The GitHub Actions workflow (`.github/workflows/ci.yaml`) runs 4 jobs:

| Job | Description | Blocking |
|---|---|---|
| `rust-tools` | Build + test all 3 crates, clippy (deny warnings), fmt check | Yes |
| `helm-lint` | Lint all 5 charts + template dry-run with test values | Yes |
| `registry-consistency` | Validate each registry entry has chart dir, model dir, and required files | Yes |
| `vram-budget-validation` | Build `vram-budget-calc` and run for all LIVE/STAGED models — fails if budget exceeded | Yes |

---

## Test Suites

| Suite | Tool | Tests | Thresholds |
|---|---|---|---|
| **Smoke** (`tests/smoke/`) | Bash | Health 200, Auth 401/403, Chat completion 200 + content, Cost metric | All must pass |
| **Load** (`tests/load/`) | k6 | Staged ramp-up (5→10→20→10→0 VUs) | p95 < 2000ms, failed < 5% |
| **Chaos** (`tests/chaos/`) | LitmusChaos | pod-delete (60s), network-latency (120s/500ms), node-drain (60s) | Recovery within cold-start SLA |

### Certification Suite (`tests.md`)

The full certification suite defines **11 categories, 48 tests** with strict GO/NO-GO criteria:

| Category | Tests | Blocking |
|---|---|---|
| 1. Packaging & model integrity | 4 | Yes |
| 2. Declarative infrastructure | 5 | Yes |
| 3. ArgoCD synchronization | 5 | Yes |
| 4. Loading & startup | 3 | Yes |
| 5. Serving API | 5 | Yes |
| 6. GPU robustness & scheduling | 5 | Yes |
| 7. Load & performance | 5 | T7.1/T7.2 blocking |
| 8. Resilience & chaos engineering | 6 | T8.1/T8.2/T8.6 blocking |
| 9. Security | 5 | Yes |
| 10. Cost & governance | 2 | Before billed traffic |
| 11. End-to-end | 3 | Yes |

---

## Documentation

### Top-Level Documents

| Document | Description |
|---|---|
| [`impl.md`](impl.md) | Reference architecture (triple-layer separation, format/engine mapping, GitOps pipeline, observability, auto-healing, multi-year robustness) |
| [`tests.md`](tests.md) | Certification test suite (11 categories, 48 tests, GO/NO-GO criteria) |
| [`namage.md`](namage.md) | Production lifecycle management |
| [`solve.md`](solve.md) | End-to-end toolchain method |

### Architecture Docs (`docs/architecture/`)

| Doc | Title |
|---|---|
| [00-overview.md](docs/architecture/00-overview.md) | Three-plane architecture overview |
| [01-formats-and-engines.md](docs/architecture/01-formats-and-engines.md) | Format-to-engine mapping + decision tree |
| [02-gpu-scheduling.md](docs/architecture/02-gpu-scheduling.md) | Node pools, VRAM formula, hardware constraints |
| [03-gateway-federation.md](docs/architecture/03-gateway-federation.md) | Priority routing, health checks, failover |
| [04-gitops-deployment.md](docs/architecture/04-gitops-deployment.md) | Sync waves, ArgoCD AppSet, Lua health checks |
| [05-observability.md](docs/architecture/05-observability.md) | LGTM stack, dashboards, anomaly detection |
| [06-resilience-and-dr.md](docs/architecture/06-resilience-and-dr.md) | Auto-healing layers, rollback strategy |
| [07-capacity-forecasting.md](docs/architecture/07-capacity-forecasting.md) | Holt-Winters, KEDA predictive, recording rules |

### Architecture Decision Records (`docs/adr/`)

| ADR | Decision |
|---|---|
| [0001](docs/adr/0001-multi-format-architecture.md) | Multi-format architecture (not ONNX-only) |
| [0002](docs/adr/0002-envoy-ai-gateway.md) | Envoy AI Gateway federation |
| [0003](docs/adr/0003-separate-engine-charts.md) | Separate engine charts per format |

### Hardware Guide (`docs/hardware/`)

| Doc | Description |
|---|---|
| [gpu.md](docs/hardware/gpu.md) | In-depth GPU reference: 3 families (consumer/workstation/datacenter), prefill vs decode, CUDA gap, per-GPU datasheets (RTX 4090/5090, 6000 Ada, H100, H200, B200, MI300X, MI300A), microarchitecture comparison, runtimes (vLLM), infrastructure constraints (power, cooling, network) |

### Runbooks (`docs/runbooks/`)

| Runbook | Scenario |
|---|---|
| [gpu-node-failure.md](docs/runbooks/gpu-node-failure.md) | GPU node failure: cordon/drain, ECC/Xid/temp checks |
| [latency-spike.md](docs/runbooks/latency-spike.md) | Latency spike: check failover, GPU throttle, scale up |
| [pod-crashloop.md](docs/runbooks/pod-crashloop.md) | Pod crash loop: OOM, model-not-found, probe-failure |

---

## Technology Stack

| Layer | Tool | Version | Purpose |
|---|---|---|---|
| **Orchestration** | Kubernetes (Talos / k3s) | 1.28+ | Container orchestration |
| **GitOps** | ArgoCD | 2.8+ | Git-based continuous delivery |
| **Safetensors/AWQ/GPTQ engine** | vLLM | 0.6.3 | PagedAttention, continuous batching |
| **ONNX engine** | ONNX Runtime GenAI | latest | ONNX model inference |
| **GPU scheduling** | NVIDIA GPU Operator | 24.9+ | Driver, DCGM, device plugin |
| **GPU scheduling** | Kueue | 0.6+ | Quotas, queues, priority |
| **GPU scheduling** | Volcano | 1.9+ | Gang scheduling |
| **Autoscaling** | KEDA | 2.14+ | Event-driven autoscaling on vLLM metrics (queue depth, KV cache usage) |
| **Node provisioning** | Karpenter | 0.37+ | On-demand GPU node provisioning |
| **API Gateway** | Envoy AI Gateway | latest | OpenAI-compatible uniform API |
| **Metrics** | Prometheus + Mimir | 2.50+ / 2.12+ | Metrics collection + long-term storage |
| **Logs** | Loki | 3.0+ | Log aggregation |
| **Traces** | Tempo + OpenTelemetry | 2.5+ | Distributed tracing |
| **Visualization** | Grafana | 10.4+ | Unified dashboards |
| **GPU metrics** | DCGM Exporter | 3.3+ | GPU utilization, memory, temp, ECC |
| **Collection** | Grafana Alloy | latest | Single agent (metrics + logs + traces) |
| **Storage** | Longhorn | 1.6+ | Distributed RWX PVC |
| **Object store** | MinIO | latest | S3-compatible model weight storage |
| **Image registry** | Harbor | 2.10+ | Self-hosted registry with CVE scan |
| **Model registry** | MLflow | 2.12+ | Model version tracking |
| **Secrets** | External Secrets Operator | 0.9+ | Secrets from Vault/AWS SM |
| **Image signing** | cosign | 2.2+ | Supply chain security |
| **Secret scanning** | gitleaks | 8.0+ | Plaintext secret detection |
| **Load testing** | k6 | 0.50+ | Performance/load testing |
| **Chaos engineering** | LitmusChaos | 3.0+ | GPU chaos experiments |
| **CI/CD** | GitHub Actions | — | Build, test, lint, validate |
| **CLI tools** | Rust | 1.70+ | engine-selector, vram-budget-calc, model-onboarding |
| **Drift detection** | Evidently AI | latest | Data quality monitoring (self-hosted) |

---

## License

MIT License