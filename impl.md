# Ultimate Reference Architecture — Cloud-Scale vLLM Model Serving Platform

## Objective

Define the most robust, modular, and durable project structure for deploying ML models on the cloud, capable of serving millions of users with auto-repair, load forecasting, and multi-year durability (years, not months). This document synthesizes and extends the existing pattern (StatefulSet bjw-template + ArgoCD + Envoy AI Gateway + sync waves) into a generalized, vLLM-based system.

---

## 0. Guiding Principle

The system must **never rigidly couple the model format to the serving engine**. The correct architecture strictly separates three planes:

1. **Model Plane** (the weights + their format) — interchangeable (safetensors, AWQ, GPTQ).
2. **Engine Plane** (the runtime that executes that format) — vLLM only, consistent across all formats.
3. **Exposure Plane** (the OpenAI-compatible API exposed at the gateway) — always identical, regardless of the engine underneath.

This separation is what makes the system modular: you add a new model without touching the gateway, you upgrade engines without touching the client.

---

## 1. Complete Panorama of Model Formats and Their Engines

Here is the complete mapping of supported formats and their engine.

| Format | Typical Use Case | Recommended Open-Source Engine | Why This Engine |
|---|---|---|---|
| **Safetensors / BF16-FP16** | Full or half-precision LLMs, large datacenter GPUs | **vLLM** | PagedAttention, continuous batching, highest throughput on server GPUs (A100/H100) |
| **AWQ/GPTQ safetensors** | Quantization, compatible with server GPUs | **vLLM** (native AWQ/GPTQ support) | Avoids re-conversion; vLLM reads these formats directly |

### Decision Rule (Engine Selection Tree)

```
Is the model in safetensors/BF16/AWQ/GPTQ?
└── Yes → vLLM
```

This rule must be **codified in an internal tool** (see section 4.3) rather than left to ad hoc human decision for each new model.

---

## 2. Repository Structure (GitOps Monorepo, Inspired by and Generalizing the Existing Pattern)

```
ai-platform/
├── charts/
│   ├── model-serving-engine/          # unified vLLM engine chart
│   ├── bjw-template/                  # common base StatefulSet/PVC/Ingress (Helm dependency)
│   ├── ai-gateway/                    # Envoy AI Gateway + backends + models + pricing
│   └── apps/                          # App-of-Apps ArgoCD (ApplicationSet per environment)
├── environments/
│   ├── dev/
│   │   └── values/<app>.yaml          # per-app, per-env overrides
│   ├── staging/
│   └── prod/
├── models/
│   ├── registry.yaml                  # declarative registry: name, format, engine, VRAM budget, status
│   └── <model-name>/
│       ├── model.md                   # model datasheet (individual model sheet)
│       ├── budget.md                  # proven VRAM/CPU budget before deployment
│       └── eval-report.md             # quality validation results before promotion
├── docs/
│   ├── architecture/
│   │   ├── 00-overview.md
│   │   ├── 01-formats-and-engines.md
│   │   ├── 02-gpu-scheduling.md
│   │   ├── 03-gateway-federation.md
│   │   ├── 04-gitops-deployment.md
│   │   ├── 05-observability.md
│   │   ├── 06-resilience-and-dr.md
│   │   └── 07-capacity-forecasting.md
│   ├── adr/                           # Architecture Decision Records
│   ├── hardware/                      # hardware reference guides
│   └── runbooks/                      # step-by-step incident procedures
├── tools/
│   ├── engine-selector/               # CLI that applies the decision tree (section 4.3)
│   ├── vram-budget-calc/              # automatic memory budget calculator
│   └── model-onboarding/              # automatic scaffold for new models (generates charts + docs)
├── observability/
│   ├── grafana-dashboards/
│   ├── prometheus-rules/
│   └── alertmanager-routes.yaml
└── tests/
    ├── smoke/                         # automatic post-deployment tests per model
    ├── load/                          # k6/Locust load test scripts
    └── chaos/                         # GPU chaos engineering scenarios
```

**Why this structure is durable**: each format has its own reusable generic chart (not a chart duplicated per model ad infinitum), each model has its declarative datasheet in `models/`, and `tools/` capitalizes operational knowledge in code rather than in someone's head.

---

## 3. Infrastructure Topology (Generalizing the Two-Cluster Pattern)

### 3.1 Control Plane / Worker Plane Separation

Reuse and generalize the already-validated principle:

- **Control cluster**: hosts only ArgoCD and the `Application`/`ApplicationSet` CRDs. Never any GPU workloads here.
- **Worker cluster(s)**: one or more clusters dedicated to actual model execution, potentially distributed by region or cloud provider.

**Why this is essential at scale**: it allows horizontal scaling of worker clusters (multi-cloud, multi-region) without ever touching the GitOps control logic, which remains unique and centralized.

### 3.2 Node Pools by Hardware Type

| Pool | Hardware | Usage |
|---|---|---|
| `gpu-h100-pool` | NVIDIA H100 | High-performance LLMs, vLLM |
| `gpu-a100-pool` | NVIDIA A100 | Standard LLMs, vLLM |
| `gpu-l4-pool` | NVIDIA L4 | Lightweight inference, cost-optimized |
| `gpu-edge-pool` | Modest GPUs (e.g., A2000, like your home setup) | Small models, PoC |
| `cpu-pool` | CPU only | Preprocessing, gateway, auxiliary services |

Each pool has its own taints/tolerations and `nodeSelector`, ensuring Kueue/Volcano places each workload on hardware matching its cost/performance ratio.

### 3.3 Recommended GPU Orchestration Tools (Open-Source, Ranked by Robustness)

| Tool | Role | Production Maturity for Long-Term Use |
|---|---|---|
| **NVIDIA GPU Operator** | Driver, device plugin, DCGM exporter, toolkit | Industry reference, actively maintained by NVIDIA |
| **Kueue** (sigs.k8s.io) | Quotas, queues, priority | Official Kubernetes SIG project, built to last |
| **Volcano** (CNCF) | Gang scheduling | CNCF incubating project, broad batch/ML adoption |
| **Karpenter** | On-demand GPU node provisioning | De facto standard on AWS, portable via providers |
| **KEDA** (CNCF) | Event-driven autoscaling, scale-to-zero | CNCF graduated project, very stable |

All these tools are CNCF projects or maintained by hardware vendors themselves — this is the selection criterion for durability (no risk of abandonment by a startup).

---

## 4. Multi-Engine Abstraction Layer (The Core of Modularity)

### 4.1 Unified Interface Contract

Regardless of the format, each vLLM serving service MUST expose:

- `POST /v1/chat/completions` (OpenAI-compatible) — so the gateway never sees a difference
- `GET /health` (503 during loading, 200 when ready)
- SSE streaming (`text/event-stream`)
- Native API key authentication (`--api-key-file` or equivalent)

### 4.2 Gateway Federation (Envoy AI Gateway)

Each serving engine, once exposed via Ingress, is federated in the gateway exactly like an external SaaS backend:

```yaml
backends:
  <model>-local-01:
    schema: OpenAI
    prefix: /v1
    fqdn.hostname: <model>--poc.example.com
    securityType: APIKey
    tlsHostname: <model>--poc.example.com
models:
  <model>-local:
    info:
      displayName: "<Model> (self-hosted)"
    contextLength: <N>
    pricing: { strategy: weighted, ... }
    backends:
      - ref: <model>-local-01
        priority: 0
```

**Why this is the most important decision in the system**: from the end client's perspective, a self-hosted vLLM model and an external SaaS provider (OpenAI, Anthropic) are strictly identical. This allows migrating a model from one engine version to another, or switching to a SaaS provider in case of failure, with no client-side change — this is the foundation of long-term robustness.

### 4.3 Internal Tool `engine-selector`

A small tool (Rust CLI, consistent with your stack) that:

1. Reads the model format (extension, HuggingFace metadata, or explicit config).
2. Applies the decision tree from section 1.
3. Automatically generates the appropriate Helm chart from the corresponding generic template (`charts/model-serving-<engine>`).
4. Calculates and validates the VRAM budget before proposing deployment (see section 4.4).

**Why this tool is essential for durability**: it eliminates tribal knowledge drift ("we know to use this engine for that format") by codifying it. A new engineer 3 years from now can onboard a model without knowing the history of decisions.

### 4.4 Systematic Memory Budget Calculation (Before Any Deployment)

Reuse and generalize the calculation already applied:

```
Usable budget = Total_VRAM × util_factor(0.85–0.90)
               − weight_size(format, quantization)
               − fixed_overhead(~1 GB)
               = Available budget for KV-cache / activations
```

This calculation must be an automated test (`tools/vram-budget-calc`) executed in CI **before** the manifest is merged — refuse deployment if the budget is negative. This prevents OOM in production, which is the most frequent and most avoidable incident.

**Hardware rule to hard-code**: never deploy an FP8 checkpoint on a GPU architecture without native FP8 support (e.g., Ampere) — automatic check to integrate into the tool.

---

## 5. Complete GitOps Pipeline (CI → CD → ArgoCD)

### 5.1 Continuous Delivery Flow

```
Merge on the charts repo (main)
   → CI: lint + helm template (blank render) + value format test
   → Publish chart as OCI (chart registry, automatic semver versioning)
   → argocd-image-updater detects a new signed image (cosign)
   → Automatic commit of the tag in the values repo (separate, signed)
   → ArgoCD syncs (separate OCI chart source + separate values source)
```

**Why separate the chart repo and the values repo**: allows differentiated access control (who can change the deployment structure vs who can change which version is in prod) and clearer audit — a pattern already validated in your ADR-0055.

### 5.2 Generalized Sync Waves

| Wave | Content | Justification |
|---|---|---|
| -3 | Bootstrap namespace, base secrets | Nothing can start without this |
| -2 | Storage (PVC, metric databases) | Pods will need ready volumes |
| -1 | Operators and collectors (GPU Operator, Prometheus Operator, log collectors) | Must run before workloads to not miss any metrics at startup |
| 0 | Workloads (the model servers themselves) | The core of the system |
| 1 | Content (Grafana dashboards, gateway configuration) | Depends on workloads already in place |
| 2+ | Post-sync (automatic smoke tests, notifications) | Final validation |

### 5.3 Custom ArgoCD Health Checks for ML CRDs

Essential for KServe whose custom CRDs are not natively understood by ArgoCD:

```yaml
resource.customizations: |
  serving.kserve.io/InferenceService:
    health.lua: |
      hs = {}
      if obj.status and obj.status.conditions then
        for i, condition in ipairs(obj.status.conditions) do
          if condition.type == "Ready" and condition.status == "True" then
            hs.status = "Healthy"
            return hs
          end
        end
      end
      hs.status = "Progressing"
      return hs
```

Without this, ArgoCD will indefinitely display "Progressing" even when the model is actually ready — a critical blind spot for the requested visibility.

---

## 6. Observability and Forecasting (The System That "Predicts and Repairs")

### 6.1 Observability Stack (Open-Source, Chosen for Durability)

| Layer | Tool | Why This Specific Choice |
|---|---|---|
| Metrics | **Prometheus** + **Mimir** (long-term storage) | De facto CNCF standard, Mimir enables multi-year retention without exploding costs |
| Logs | **Loki** | Consistent with the Grafana ecosystem (LGTM stack), low storage cost |
| Traces | **Tempo** + **OpenTelemetry** | Standard distributed tracing, essential for multimodal pipelines |
| Visualization | **Grafana** | Unifies metrics/logs/traces in a single dashboard |
| Low-level GPU metrics | **DCGM Exporter** (NVIDIA) | Only official exporter giving real SM/memory/temperature utilization per GPU |
| Collection | **Grafana Alloy** (successor to Grafana Agent) | Single agent for metrics/logs/traces, reduces operational complexity |

This is exactly the LGTM stack already present in your architecture (Mimir/Loki/Tempo/Grafana) — to generalize as the mandatory foundation for any new worker cluster.

### 6.2 Load Forecasting (Capacity Forecasting)

- **Prometheus + simple time-series models (Holt-Winters via `prometheus-anomaly-detector` or seasonal recording rules)** to anticipate recurring peaks (office hours, campaign launches).
- **KEDA with predictive scalers**: combine a cron-based scaler (pre-warming before a known peak) with a reactive scaler (actual QPS) to avoid cold start at the critical moment.
- **Regular automated load tests** (k6 or Locust, in `tests/load/`) executed in CI periodically, not just before a major deployment — to detect capacity drift before it becomes an incident.

### 6.3 Auto-Repair System (Layered Auto-Healing)

| Level | Mechanism | Tool |
|---|---|---|
| Pod | Restart on liveness probe failure | Native Kubernetes |
| Failing GPU node | Xid error detection + automatic cordon/drain | **NVIDIA GPU Operator** (integrated node health check) |
| Configuration drift | Automatic re-sync to Git state | **ArgoCD self-healing** (already native) |
| Model quality degradation | Automatic failover to a simpler fallback model | Application-level circuit breaker at the gateway (Envoy) |
| Entire cluster failure | Traffic failover to another cluster/region | DNS-based failover or multi-backend gateway with priority |
| Data drift | Alert + trigger re-evaluation pipeline | **Evidently AI** (open-source, self-hosted, no SaaS dependency) |

**Key durability principle**: each repair mechanism must leave a **trace in Git** of its action (even automatic), so that 2 years from now we can understand why a rollback occurred without log archaeology.

---

## 7. Multi-Year Robustness: What to Plan from Day One

### 7.1 Dependency Choices for Longevity

Systematically prefer:
- **CNCF graduated** projects (Kubernetes, Prometheus, Envoy, Helm, etc.) over recent tools not governed by a neutral foundation.
- Model formats **with an established conversion ecosystem** (safetensors, AWQ, GPTQ) over proprietary formats from a single vendor.
- Engines **actively maintained by multiple independent contributors** (vLLM) over single-maintainer projects.

### 7.2 Living Documentation as a Safeguard

Reuse and systematize the pattern already in place:
- **ADRs** (Architecture Decision Records) for each structuring decision — why a given engine was chosen for a given format, why a two-cluster architecture.
- **Per-model datasheet** (`models/<model>/model.md`) documenting the proven VRAM budget, status (LIVE/STAGED/STANDBY), and engine migration history if applicable.
- **Incident runbooks** written BEFORE the incident, not after — a system that must last years will have team turnover, and knowledge must be in the repository, not in a person.

### 7.3 Structural Non-Regression Tests

- `helm lint --strict` + `helm template --dry-run` in CI on **all** charts at every commit, not just modified ones (detects regressions in shared Helm dependencies like `bjw-template`).
- Automatic registry consistency test (`models/registry.yaml`): each declared model must have a corresponding chart, a corresponding gateway entry, and a proven VRAM budget — otherwise CI failure.
- **Model onboarding checklist** automated by the `model-onboarding` tool (section 2), which scaffolds all necessary files and prevents forgetting a step (already present in manual form in your document — to turn into an executable tool).

### 7.4 Multi-Cloud / Anti-Lock-In Strategy

- Keep Kubernetes as the only orchestration dependency (no non-portable proprietary cloud service like AWS SageMaker endpoints).
- The OpenAI-compatible gateway pattern allows transparent switching between self-hosted and external SaaS in case of provider failure — already the foundation of your architecture, to document explicitly as a continuity strategy.
- Store model weights in an S3-compatible object store (self-hosted MinIO or S3/GCS/R2) rather than a non-portable proprietary service.

---

## 8. Summary — Complete Recommended Technology Stack

| Layer | Selected Tool | Alternative if Different Constraints |
|---|---|---|
| Orchestration | Kubernetes (Talos for nodes, or k3s for lightweight clusters) | — |
| GitOps | ArgoCD | Flux (if different multi-tenant pull preference) |
| Safetensors/AWQ/GPTQ engine | vLLM | TGI |
| GPU scheduling | Kueue + Volcano + NVIDIA GPU Operator | — |
| Autoscaling | KEDA + HPA custom metrics | — |
| Node provisioning | Karpenter | Cluster Autoscaler |
| API Gateway | Envoy AI Gateway (OpenAI-compatible) | — |
| Observability | Prometheus/Mimir + Loki + Tempo + Grafana + DCGM | — |
| Drift/quality | Evidently AI (self-hosted) | WhyLabs (if SaaS acceptable) |
| Secrets | External Secrets Operator + AWS Secrets Manager (or Vault) | — |
| Image registry | Harbor (self-hosted, integrated CVE scan) | — |
| Model registry | MLflow Model Registry (self-hosted) | — |
| Weight object store | MinIO (self-hosted, S3-compatible) | S3/GCS/R2 directly |
| Load testing | k6 or Locust | — |

---

## 9. Final Model Onboarding Checklist (Generalized, Multi-Format)

1. Identify the model's native format (safetensors, AWQ, GPTQ).
2. Run `engine-selector` → gets the recommended engine and generated chart.
3. Run `vram-budget-calc` → validates that the memory budget is positive on the target GPU pool; reject if negative or if hardware incompatibility (e.g., FP8 on Ampere).
4. Fill in the model datasheet (`models/<model>/model.md`) with budget, status, context.
5. Generate the gateway entry (`backends` + `models` in `charts/ai-gateway/values.yaml`), with appropriate pricing and timeout.
6. Open a PR on the values repo (not the chart repo) — triggers the standard GitOps flow.
7. Verify in CI: lint, template dry-run, registry consistency.
8. ArgoCD syncs according to the defined sync waves.
9. Automatic post-sync smoke tests (`tests/smoke/`): auth 401/200, real completion, non-zero cost metric.
10. Progressive promotion: low `priority` in gateway first (canary), gradual ramp-up, then normal priority once validated on real traffic.
11. Add the model to the global Grafana dashboard and Prometheus alerting rules.
12. Document in an ADR if this model introduces a new pattern (new format, new engine, new hardware constraint).

This checklist, once fully toolized (sections 2 and 4.3), transforms adding a model from a manual craft operation into a reproducible, tested, and audited operation — the necessary condition for a system that must remain correct and understandable for years, with changing teams.