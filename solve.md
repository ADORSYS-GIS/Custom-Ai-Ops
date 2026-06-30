# Ultimate Method: Fine-Grained ML Model Production Management (End-to-End → ArgoCD)

## Objective

Provide an ordered, tool-by-tool method for moving a model from the "trained" state to the "served in production, visible and managed via GitOps/ArgoCD" state, addressing the specific problems of each step. Each tool is presented with its **exact role** and **why it is indispensable** (not just a list).

---

## Complete Chain Overview

```
[1. Model Packaging] → [2. Model Registry] → [3. Optimization/Compilation]
→ [4. Containerization] → [5. Image Registry] → [6. CI] → [7. K8s/Helm Manifest]
→ [8. Git (source of truth)] → [9. ArgoCD (GitOps sync)] → [10. GPU-aware Scheduler]
→ [11. Serving runtime] → [12. Service mesh / routing] → [13. Autoscaling]
→ [14. Observability] → [15. Drift / quality detection] → [16. Automatic rollback]
```

Each step resolves a specific class of problems. Skipping any breaks the chain of guarantees.

---

## Step 1 — Model Packaging and Versioning

**Tools**: MLflow Model Registry, DVC (Data Version Control), or private Hugging Face Hub.

**Role**: Capture the model, its weights, hyperparameters, reference dataset, and evaluation metrics as a versioned and traceable unit.

**Why this is critical**: without this, you lose traceability between "which model runs in prod" and "which training run produced it". This is the #1 cause of non-reproducible incidents. MLflow provides a unique version identifier for the model (`model:/name/version`) that everything downstream references.

---

## Step 2 — Model Registry

**Tools**: MLflow Registry, Weights & Biases Model Registry, or internal registry based on object store (S3 + metadata).

**Role**: Centralize promoted versions (staging → production → archived), with an approval gate before promotion.

**Why this is critical**: prevents an unvalidated model from reaching production. This is the quality control point before anything touches Kubernetes.

---

## Step 3 — Model Optimization and Compilation

**Tools by model family**:
- **TensorRT / TensorRT-LLM** (NVIDIA): compilation for NVIDIA GPU, kernel fusion, quantization.
- **ONNX Runtime**: intermediate portable format, useful for decoupling training framework from inference runtime.
- **vLLM** or **TGI (Text Generation Inference)**: specialized LLM runtime with PagedAttention and continuous batching.
- **OpenVINO**: for CPU/Intel deployment.

**Role**: transform the raw model (PyTorch/TensorFlow) into an optimized form for real production latency and throughput.

**Why this is critical**: an uncompiled/unquantized model can cost 3 to 10x more in GPU and have 2 to 5x higher latency. This is where most KV-cache (vLLM), kernel fusion, and numerical precision (FP16/INT8/FP8) problems are resolved.

---

## Step 4 — Containerization

**Tools**: Docker with official NVIDIA CUDA/cuDNN base image, or **BentoML** / **Cog** for automatically packaging a model into a servable image.

**Role**: encapsulate the model, inference runtime, and system dependencies (compatible CUDA/cuDNN drivers) into an immutable image.

**Why this is critical**: solves the "works on my machine" problem — guarantees that the CUDA/cuDNN version used in training is compatible with the inference environment, a frequent source of silent bugs (different results) or crashes.

**Note**: never download model weights into the Docker image itself (image too heavy, rebuild unnecessary on weight update). Weights should be loaded at startup from object store via an init container (see step 10).

---

## Step 5 — Image Registry

**Tools**: Harbor (self-hosted, with integrated vulnerability scanning), or cloud registries (ECR, GCR, ACR).

**Role**: store versioned and scanned images, with access control.

**Why this is critical**: Harbor adds automatic CVE scanning on each push — essential for images containing numerous Python/CUDA dependencies, often vulnerability vectors.

---

## Step 6 — Continuous Integration (CI)

**Tools**: GitHub Actions, GitLab CI, or Jenkins.

**Role**: automate image build, serving code unit tests, smoke tests (load model and run inference test), security scan, and push to image registry.

**Why this is critical**: guarantees that no image reaches production without passing a minimal inference test. This is the last safety net before the Kubernetes manifest is updated.

**Key step**: the CI pipeline ends by updating the image tag in the Git manifest repository (not by deploying directly) — this is what enables GitOps via ArgoCD.

---

## Step 7 — Kubernetes / Helm Manifests

**Tools**: Helm charts, or Kustomize for environment overlays (dev/staging/prod).

**Role**: declaratively define the Deployment, GPU requests/limits, probes, HPA, PodDisruptionBudget, model configuration ConfigMaps.

**Why this is critical**: Helm allows templating and versioning infrastructure configuration exactly like code, with different values per environment (e.g., 1 GPU in staging, 4 GPUs in prod).

**Specialized tools to use here**: **KServe** (CRD `InferenceService`) or **Seldon Core** instead of a raw Deployment — they already encapsulate ML-native readiness probe, autoscaling based on actual load, and native canary support.

---

## Step 8 — Git as Source of Truth

**Tools**: separate Git repository for manifests (pattern "config repo" distinct from "code repo").

**Role**: become the unique source of truth for the cluster desired state. All configuration changes go through a Pull Request.

**Why this is critical**: this is the foundation of GitOps — without this, ArgoCD has nothing to synchronize. This also provides complete audit history (who changed what, when) and allows rollback via simple `git revert`.

---

## Step 9 — ArgoCD (GitOps Synchronization)

**Exact role of ArgoCD**: monitor the Git manifest repository continuously and automatically synchronize the actual cluster state with the declared state in Git. If someone manually modifies a resource in the cluster (`kubectl edit`), ArgoCD detects the drift (**infrastructure drift**, different from data drift in part 1) and can automatically correct it (self-healing) or alert.

**Why this is the central visibility tool**:
- **Visual dashboard**: ArgoCD shows the complete tree of deployed resources (Deployment → ReplicaSet → Pods → Services) with their health status in real time (Healthy/Degraded/Progressing).
- **Automatic or manual sync**: you choose whether a Git change deploys automatically or requires manual approval in the interface — useful for production where you want a human gate before promotion.
- **One-click rollback**: ArgoCD keeps sync history, allowing immediate return to a previous manifest version.
- **App of Apps pattern**: manage multiple models/services as a hierarchy of ArgoCD applications, useful when operating multiple models (UMC, NEURAX, etc. if each has its own inference service).
- **Notifications** (ArgoCD Notifications controller): automatic Slack/email alert on sync failure or application health degradation.

**Recommended configuration for ML**:
- Custom health checks in ArgoCD for KServe `InferenceService` CRDs (ArgoCD doesn't natively know CRD custom health status without `resource.customizations` configuration).
- Sync waves to order deployment (model config ConfigMap before Deployment, for example).
- ArgoCD Projects to isolate permissions by team/model (RBAC).

---

## Step 10 — GPU-Aware Scheduler (at the moment ArgoCD synchronizes)

**Tools**: Kueue or Volcano (mentioned in previous document), coupled with NVIDIA GPU Operator.

**Role**: once ArgoCD applies the manifest, it's the Kubernetes scheduler (extended by Kueue/Volcano) that decides on which GPU node to place the pod, respecting quotas and hardware affinity.

**Why this is critical at this stage**: ArgoCD only declares intent; without a GPU-aware scheduler, the pod may remain in `Pending` indefinitely or be placed on a suboptimal node. This is visible in ArgoCD as a "Progressing" state that never becomes "Healthy" — directly diagnosed via the ArgoCD interface.

---

## Step 11 — Serving Runtime

**Tools by family** (summary from previous document, applied concretely here):
- LLM: vLLM, TGI, TensorRT-LLM
- Vision/classical: Triton Inference Server (NVIDIA) — native multi-framework and format support, dynamic batching
- General multi-framework: Triton or Ray Serve

**Role**: actually execute inference with optimal batching, caching, and GPU management.

**Why Triton in particular**: it natively exposes Prometheus metrics, supports model ensemble (multi-model pipelines), and handles multi-versioning (multiple model versions served simultaneously) — directly useful for ArgoCD/KServe managed canary.

---

## Step 12 — Service Mesh / Intelligent Routing

**Tools**: Istio or Linkerd, often automatically integrated by KServe.

**Role**: manage canary routing (traffic percentage), mirroring (shadow traffic), mTLS between services, and retries/timeouts at network level.

**Why this is critical**: allows testing a new model version on a small percentage of real traffic without risk, AND this traffic distribution is itself declared in Git and synchronized by ArgoCD (the canary percentage becomes a versioned value).

---

## Step 13 — Autoscaling

**Tools**: KEDA (event-driven) + custom HPA metrics, already detailed previously.

**Role**: dynamically adjust replica count based on actual load (QPS, queue length).

**Why visible in ArgoCD**: the HPA/ScaledObject is itself a manifest managed by Git/ArgoCD — so its configuration (thresholds, min/max replicas) is versioned and auditable like the rest.

---

## Step 14 — Observability

**Tools**: Prometheus (metrics) + Grafana (dashboards) + DCGM Exporter (GPU) + OpenTelemetry (tracing) + Loki (logs).

**Role**: provide a complete and correlated view of the technical and business health of the model in production.

**Why this is the indispensable complement to ArgoCD**: ArgoCD shows that deployment matches Git (infrastructure health), but says nothing about prediction quality or actual performance. Grafana shows runtime state; ArgoCD shows declared state. Both dashboards must be consulted together.

---

## Step 15 — Drift and Quality Detection

**Tools**: Evidently AI, WhyLabs, or Arize AI.

**Role**: continuously monitor input/output distribution and alert on significant drift.

**Why this is the often-overlooked step**: a model can remain "Healthy" in ArgoCD (pod running, responding to probes) while being qualitatively degraded. This layer fills the blind spot that neither Kubernetes nor ArgoCD covers.

---

## Step 16 — Automatic Rollback

**Mechanism**: integration between drift/quality tool (step 15) and ArgoCD via webhook or custom controller that triggers automatic `git revert` on manifest repo on detected degradation.

**Role**: close the loop completely automatic — detection → corrective action without human intervention.

**Why this is the culmination of the method**: this transforms ArgoCD from a simple synchronization tool into a closed-loop control system, where Git remains the source of truth even after automatic correction (complete traceability of rollback).

---

## Summary Table: Tool Role and Importance

| Tool | Role | Problem Without This Tool |
|---|---|---|
| MLflow Registry | Versioning and model traceability | Impossible to know which run produced the model in prod |
| TensorRT/vLLM/TGI | Runtime compilation and optimization | Unmanaged GPU latency and cost |
| Docker + fixed CUDA image | Reproducible environment | Silent bugs from different CUDA versions |
| Harbor | Secure image registry | Vulnerabilities undetected before deployment |
| CI (GitHub Actions/GitLab CI) | Automatic validation before deployment | Untested images in production |
| Helm/Kustomize | Versioned infrastructure declaration | Divergent configuration between environments |
| KServe/Seldon | ML-native abstraction on K8s | Fragile manual reimplementation of canary/autoscaling |
| Git (config repo) | Unique source of truth | No audit, no reliable rollback |
| **ArgoCD** | **GitOps synchronization and central visibility** | **Undetected drift between Git and actual cluster, no centralized view** |
| Kueue/Volcano | GPU-aware scheduling | Pods in Pending, poor GPU placement |
| Triton/vLLM | Optimized inference execution | Underutilized GPU, low throughput |
| Istio/Linkerd | Canary routing and mTLS | Risky deployments without traffic control |
| KEDA/HPA | Dynamic adaptation to load | Over-provisioning costs or under-capacity at peak |
| Prometheus/Grafana/DCGM | Observability | Incidents detected too late or not at all |
| Evidently/WhyLabs | Continuous quality and drift | Silent degradation of prediction quality |

---

## Recommended Implementation Order (Realistic Prioritization)

1. Docker + fixed CUDA image (indispensable base)
2. Git config repo + Helm charts
3. ArgoCD (immediate visibility, quick win)
4. Minimal CI (build + smoke test)
5. Prometheus/Grafana + DCGM (basic observability)
6. KServe or Seldon (replaces raw Deployment)
7. Optimized runtime (vLLM/TensorRT/Triton by model family)
8. Kueue/Volcano (once multiple models compete on same GPUs)
9. Istio + canary (once new version deployment rate increases)
10. KEDA (once traffic becomes variable/unpredictable)
11. MLflow Registry (formalize versioning if not already done upstream)
12. Evidently/WhyLabs + automatic rollback (final maturity, closed-loop)

This order avoids premature over-engineering: ArgoCD and basic observability provide immediate value and should come early, while the fully closed-loop automatic rollback is the last brick, once the entire signal chain (quality, drift) is reliable.
