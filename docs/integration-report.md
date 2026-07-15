# Integration Report — ArgoCD & External Platforms

> **Status:** Architecture & Integration Plan
> **Date:** 2026-07-06
> **Scope:** Complete integration strategy for the Custom-Ai-Ops repository with ArgoCD, CI/CD, external Git providers, container registries, and secret managers.

---

## Table of Contents

1. [Repository Inventory & Current State](#1-repository-inventory--current-state)
2. [Git Provider Integration](#2-git-provider-integration)
3. [ArgoCD Integration — Complete Configuration](#3-argocd-integration--complete-configuration)
4. [Container Registry Integration](#4-container-registry-integration)
5. [Secret Management Integration](#5-secret-management-integration)
6. [CI/CD Pipeline Integration](#6cicd-pipeline-integration)
7. [Multi-Cluster & Multi-Region Integration](#7-multi-cluster--multi-region-integration)
8. [Security Hardening — End-to-End](#8-security-hardening--end-to-end)
9. [Integration Topology Diagram](#9-integration-topology-diagram)
10. [Integration Checklist](#10-integration-checklist)

---

## 1. Repository Inventory & Current State

### 1.1 What Exists Today

| Component | Location | Status |
|-----------|----------|--------|
| Helm charts (3) | `charts/` | model-serving-engine (active, vLLM-only), model-serving-vllm (deprecated), bjw-template |
| Environment values | `environments/{dev,staging,prod}/values.yaml` | QoS Guaranteed, KEDA, swapoff, nodeSelector |
| ArgoCD ApplicationSets | `apps/argocd-appset-{dev,staging,prod}.yaml` | 3 envs, sync waves -3→2, selfHeal+prune, ServerSideApply |
| ArgoCD health checks | `apps/argocd-health-checks.yaml` | Lua: StatefulSet + InferenceService |
| CI pipeline | `.github/workflows/ci.yaml` | Rust build+test, helm lint (4), registry consistency, VRAM validation |
<<<<<<< Updated upstream
| Observability rules | `observability/prometheus-anomaly-rules.yaml` | 6 groups incl. KV cache alerts |
| Grafana dashboards | `observability/grafana-dashboards/` | 12 panels incl. KV cache |
| Alertmanager routes | `observability/alertmanager-routes.yaml` | critical→PagerDuty+Slack, warning→Slack, gpu→#gpu-ops |
=======
>>>>>>> Stashed changes
| Model registry | `models/registry.yaml` | 3 models (mistral-7b, phi-3-mini, llama-3-70b) |
| Rust tools | `tools/` | engine-selector, vram-budget-calc, model-onboarding |
| Tests | `tests/` | smoke (bash), load (k6), chaos (litmus) |
| Docs | `docs/` | architecture (7), runbooks (3), explain (2) |

### 1.2 Git Remote

```
origin  git@github.com:rustnew/custom-ai-ops.git
```

The repository is currently on GitHub under the `rustnew` account. All ArgoCD ApplicationSets reference `https://github.com/Custom-Ai-Ops/Custom-Ai-Ops.git` — this URL **must be corrected** to match the actual remote, or the remote must be renamed to match.

### 1.3 Gap Analysis

| Gap | Impact | Section |
|-----|--------|---------|
| repoURL mismatch in all AppSets | ArgoCD cannot sync | §3.2 |
| No ArgoCD `Project` definitions | No RBAC isolation between envs | §3.3 |
| No repo credential secret in ArgoCD | Cannot authenticate to private repo | §3.4 |
| No External Secrets Operator manifest | Secrets referenced but not provisioned | §5 |
| No container registry credentials secret | Pods cannot pull images | §4 |
| No multi-cluster destination config | All AppSets target `kubernetes.default.svc` only | §7 |
| No image update automation (ArgoCD Image Updater) | Image tags pinned manually | §4.3 |
| No signed Helm charts (Cosign) | Supply chain unverified | §8.4 |
| No network policies | No east-west traffic isolation | §8.5 |

---

## 2. Git Provider Integration

### 2.1 GitHub (Current)

The repository lives on GitHub. ArgoCD integrates with GitHub via either:

- **HTTPS + Personal Access Token (PAT)** — for private repos, a fine-grained PAT with `Contents: Read` on the repository.
- **SSH + Deploy Key** — for private repos, a dedicated SSH key pair registered as a deploy key on the GitHub repository.
- **GitHub App** — for organizations, a GitHub App with `Contents: Read` permission, providing scoped tokens that auto-rotate.

### 2.2 GitLab (Alternative)

If migrating to or mirroring on GitLab:

- ArgoCD supports GitLab natively via the same HTTPS/SSH mechanisms.
- GitLab Deploy Tokens provide registry pull access bundled with repo access.
- GitLab CI replaces GitHub Actions — the `.gitlab-ci.yml` equivalent of `.github/workflows/ci.yaml` uses the same `cargo`/`helm` commands.

### 2.3 Bitbucket / Azure DevOps

- Both are supported by ArgoCD via HTTPS + app passwords.
- Azure DevOps requires a Personal Access Token with `Code: Read` scope.

### 2.4 Repository Mirroring (Multi-Provider Redundancy)

For disaster recovery, the repository should be mirrored to a second Git provider:

```
GitHub (primary) ──mirror──> GitLab (secondary)
```

ArgoCD can be configured with multiple repository connections and fall back to the secondary if the primary is unreachable. This is done via the `argocd-cm` ConfigMap with multiple `repository` entries.

### 2.5 Branch Protection Rules (GitHub)

| Rule | Branch | Enforcement |
|------|--------|-------------|
| Require PR review (2 approvers) | `main` | GitHub branch protection |
| Require status checks (CI) | `main` | All 4 CI jobs must pass |
| Require signed commits | `main` | GPG or SSH signing |
| No force push | `main` | Enforced |
| No branch deletion | `main` | Enforced |
| Linear history | `main` | Rebase merges only |
| Environment protection | `prod` | Deployment requires review |

---

## 3. ArgoCD Integration — Complete Configuration

### 3.1 ArgoCD Installation

ArgoCD is installed on the **Control Cluster** (no GPU workloads). The installation method depends on the platform:

| Platform | Method | Manifest |
|-----------|--------|----------|
| Vanilla K8s | `kubectl apply -f install.yaml` | ArgoCD official |
| Helm | `helm install argocd argo/argo-cd` | `charts/argocd/` (to create) |
| ArgoCD Operator | OLM / OperatorHub | For OpenShift |
| ArgoCD Autopilot | `argocd-autopilot repo bootstrap` | Git-bootstrapped |

### 3.2 Repository Connection Configuration

#### 3.2.1 Repository Credential Secret

A Kubernetes Secret in the `argocd` namespace stores the Git credentials. ArgoCD references this secret by name in the ApplicationSet `spec.generators.git.repoURL` — the secret must have the label `argocd.argoproj.io/secret-type: repository`.

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: custom-ai-ops-repo
  namespace: argocd
  labels:
    argocd.argoproj.io/secret-type: repository
type: Opaque
stringData:
  type: git
  url: git@github.com:rustnew/custom-ai-ops.git
  sshPrivateKey: |
    -----BEGIN OPENSSH PRIVATE KEY-----
    <deploy-key-private-key>
    -----END OPENSSH PRIVATE KEY-----
```

Alternatively, for HTTPS + PAT:

```yaml
stringData:
  type: git
  url: https://github.com/rustnew/custom-ai-ops.git
  username: rustnew
  password: <fine-grained-PAT>
```

#### 3.2.2 Known Hosts / SSH Cert Authority

For SSH connections, the Git server's SSH host key must be added to the `argocd-ssh-known-hosts-cm` ConfigMap:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: argocd-ssh-known-hosts-cm
  namespace: argocd
data:
  ssh_known_hosts: |
    github.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdGZU0QXa...
    gitlab.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE...
```

#### 3.2.3 repoURL Correction

All ApplicationSets currently reference `https://github.com/Custom-Ai-Ops/Custom-Ai-Ops.git`. This must be corrected to:

```
repoURL: git@github.com:rustnew/custom-ai-ops.git
```

Or, if using HTTPS:

```
repoURL: https://github.com/rustnew/custom-ai-ops.git
```

This change applies to all 4 files: `apps/argocd-appset-{dev,staging,prod}.yaml` and any infrastructure ApplicationSets.

### 3.3 ArgoCD Projects

Three ArgoCD Projects provide RBAC isolation between environments:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: model-serving
  namespace: argocd
spec:
  description: Model serving workloads (vLLM)
  sourceRepos:
    - git@github.com:rustnew/custom-ai-ops.git
  destinations:
    - server: https://kubernetes.default.svc
      namespace: model-serving-dev
    - server: https://kubernetes.default.svc
      namespace: model-serving-staging
    - server: https://kubernetes.default.svc
      namespace: model-serving-prod
  clusterResourceWhitelist:
    - group: ""
      kind: Namespace
    - group: ""
      kind: PersistentVolume
    - group: ""
      kind: PersistentVolumeClaim
    - group: "apps"
      kind: StatefulSet
    - group: "apps"
      kind: DaemonSet
    - group: "autoscaling.keda.sh"
      kind: ScaledObject
  namespaceResourceWhitelist:
    - group: "*"
      kind: "*"
  namespaceResourceBlacklist:
    - group: ""
      kind: ResourceQuota
    - group: ""
      kind: LimitRange
---
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: infrastructure
  namespace: argocd
spec:
  description: Cluster infrastructure (GPU operator, Longhorn)
  sourceRepos:
    - git@github.com:rustnew/custom-ai-ops.git
  destinations:
    - server: https://kubernetes.default.svc
      namespace: nvidia-gpu-operator
    - server: https://kubernetes.default.svc
      namespace: longhorn-system
  clusterResourceWhitelist:
    - group: "*"
      kind: "*"
---
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: security
  namespace: argocd
spec:
  description: Security & secrets management
  sourceRepos:
    - git@github.com:rustnew/custom-ai-ops.git
  destinations:
    - server: https://kubernetes.default.svc
      namespace: external-secrets
    - server: https://kubernetes.default.svc
      namespace: cert-manager
```

### 3.4 ApplicationSet Architecture

The existing ApplicationSets use two generator types:

| Generator | Used For | Files |
|------------|----------|-------|
| `git` (directory) | Per-environment model serving | `argocd-appset-{dev,staging,prod}.yaml` |
| `list` | Infrastructure, secrets | Embedded in prod AppSet |

#### 3.4.1 Sync Wave Strategy (Existing, Validated)

| Wave | Resources | Justification |
|------|-----------|---------------|
| -3 | Secrets (ESO ExternalSecrets) | Nothing starts without credentials |
| -2 | Storage (PVC, Longhorn), swapoff DaemonSet | Volumes + swap off before pods |
| -1 | NVIDIA GPU Operator | Operators must run before workloads |
| 0 | Model serving StatefulSets | Core inference workloads |
| 1 | NetworkPolicies | Depends on workloads being up |
| 2 | Smoke tests, notifications | Post-deploy validation |

#### 3.4.2 Sync Policy Configuration

All ApplicationSets use:

```yaml
syncPolicy:
  automated:
    prune: true           # Delete resources removed from Git
    selfHeal: true        # Re-apply if someone manually edits via kubectl
    allowEmpty: false     # Prevent accidental mass deletion
  syncOptions:
    - CreateNamespace=true
    - ServerSideApply=true  # Avoid conflicts with other controllers
  retry:
    limit: 3              # (5 for infrastructure)
    backoff:
      duration: 30s
      factor: 2
      maxDuration: 5m
```

### 3.5 Custom Health Checks

The existing Lua health checks in `apps/argocd-health-checks.yaml` are applied via the `argocd-cm` ConfigMap:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: argocd-cm
  namespace: argocd
data:
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
        hs.message = "InferenceService is not ready yet"
        return hs
    apps/v1.StatefulSet:
      health.lua: |
        hs = {}
        if obj.status and obj.status.readyReplicas and obj.status.replicas then
          if obj.status.readyReplicas >= obj.status.replicas then
            hs.status = "Healthy"
            return hs
          end
        end
        hs.status = "Progressing"
        hs.message = "StatefulSet is not ready"
        return hs
```

Additional health checks to add:

| CRD | Health Logic |
|-----|-------------|
| `autoscaling.keda.sh/ScaledObject` | Check `status.conditions` for `Ready=True` |

### 3.6 ArgoCD RBAC

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: argocd-rbac-cm
  namespace: argocd
data:
  policy.default: role:readonly
  policy.csv: |
    p, role:dev-admin, applications, *, model-serving-dev/*, allow
    p, role:staging-admin, applications, *, model-serving-staging/*, allow
    p, role:prod-admin, applications, *, model-serving-production/*, deny
    p, role:prod-approver, applications, sync, model-serving-production/*, allow
    p, role:prod-viewer, applications, get, model-serving-production/*, allow
    p, role:infra-admin, applications, *, model-serving-infrastructure/*, allow
    p, role:security-admin, applications, *, model-serving-secrets/*, allow
    g, dev-team, role:dev-admin
    g, staging-team, role:staging-admin
    g, prod-sre, role:prod-approver
    g, prod-sre, role:prod-viewer
    g, infra-team, role:infra-admin
    g, security-team, role:security-admin
```

### 3.7 ArgoCD Notifications

Sync status notifications are configured via the `argocd-notifications-cm` ConfigMap:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: argocd-notifications-cm
  namespace: argocd
data:
  service.slack: |
    {
      "token": "$slack-token"
    }
  trigger.on-sync-status-unknown: |
    - when: app.status.sync.status == "Unknown"
      send: [slack-sync-status]
  trigger.on-sync-failed: |
    - when: app.status.sync.status == "OutOfSync"
      oncePer: app.status.sync.revision
      send: [slack-sync-failed]
  trigger.on-sync-succeeded: |
    - when: app.status.sync.status == "Synced"
      oncePer: app.status.sync.revision
      send: [slack-sync-succeeded]
  trigger.on-health-degraded: |
    - when: app.status.health.status == "Degraded"
      send: [slack-health-degraded]
  template.slack-sync-failed: |
    {
      "channel": "#ml-ops",
      "message": "Sync FAILED: {{.app.metadata.name}} ({{.app.status.sync.revision | truncate 7}})"
    }
  template.slack-health-degraded: |
    {
      "channel": "#ml-incidents",
      "message": "HEALTH DEGRADED: {{.app.metadata.name}} — {{.app.status.health.message}}"
    }
```

### 3.8 ArgoCD Application Bootstrap Order

The bootstrap sequence for a fresh cluster:

```
1. Install ArgoCD (helm install argocd argo/argo-cd -n argocd)
2. Apply repo credential Secret (§3.2.1)
3. Apply AppProjects (§3.3)
4. Apply argocd-cm (health checks, §3.5)
5. Apply argocd-rbac-cm (§3.6)
6. Apply argocd-notifications-cm (§3.7)
7. Apply argocd-appset-prod.yaml (or dev/staging)
8. ArgoCD auto-syncs in wave order: -3 → -2 → -1 → 0 → 1 → 2
```

---

## 4. Container Registry Integration

### 4.1 Registry Selection

| Registry | Use Case | Auth Method |
|----------|----------|-------------|
| Docker Hub | Public vLLM images | Anonymous pull (rate-limited) |
| GitHub Container Registry (GHCR) | Private custom images | PAT or GitHub App |
| Harbor | Self-hosted, vulnerability scanning | Robot account |
| ECR / GCR / ACR | Cloud-native | IAM role / Workload Identity |
| Quay.io | Red Hat ecosystem | Robot account |

### 4.2 Image Pull Secret

A Kubernetes Secret of type `kubernetes.io/dockerconfigjson` allows pods to pull from private registries:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: registry-pull-secret
  namespace: model-serving-prod
  annotations:
    argocd.argoproj.io/sync-wave: "-3"
type: kubernetes.io/dockerconfigjson
data:
  .dockerconfigjson: <base64-encoded-docker-config-json>
```

This secret is referenced in the StatefulSet via:

```yaml
spec:
  template:
    spec:
      imagePullSecrets:
        - name: registry-pull-secret
```

The `imagePullSecrets` field is already present in `charts/model-serving-engine/values.yaml` as `global.imagePullSecrets: []` — it needs to be populated per environment.

### 4.3 ArgoCD Image Updater

For automatic image tag updates (when a new vLLM image is pushed):

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: model-serving-prod
  namespace: argocd
  annotations:
    argocd-image-updater.argoproj.io/image-list: >
      vllm=vllm/vllm-openai:~v0.6
    argocd-image-updater.argoproj.io/write-back-method: git
    argocd-image-updater.argoproj.io/git-branch: main
    argocd-image-updater.argoproj.io/vllm.update-strategy: semver
```

Image Updater writes new tags back to the Helm `values.yaml` in Git, triggering an ArgoCD sync. This creates a full audit trail.

### 4.4 Image Signature Verification (Cosign)

For supply chain security, images are signed with Cosign and verified at admission time:

```yaml
apiVersion: policy.sigstore.dev/v1beta1
kind: ClusterImagePolicy
metadata:
  name: vllm-image-policy
spec:
  images:
    - glob: "vllm/vllm-openai:*"
  authorities:
    - key:
        data: |
          -----BEGIN PUBLIC KEY-----
          <cosign-public-key>
          -----END PUBLIC KEY-----
```

Policy Controller (Sigstore) enforces this at the admission webhook — unsigned images are rejected.

---

## 5. Secret Management Integration

### 5.1 External Secrets Operator (ESO)

The platform uses ESO to synchronize secrets from an external vault into Kubernetes. No secret material is stored in Git.

#### 5.1.1 SecretStore

```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: vault-backend
  namespace: model-serving-prod
  annotations:
    argocd.argoproj.io/sync-wave: "-3"
spec:
  provider:
    vault:
      server: "https://vault.internal.example.com:8200"
      path: "secret"
      version: "v2"
      auth:
        kubernetes:
          mountPath: "kubernetes"
          role: "model-serving-prod"
          serviceAccountRef:
            name: external-secrets-sa
            namespace: model-serving-prod
```

#### 5.1.2 ExternalSecret

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: model-serving-secrets
  namespace: model-serving-prod
  annotations:
    argocd.argoproj.io/sync-wave: "-3"
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: vault-backend
    kind: SecretStore
  target:
    name: model-serving-secrets
    creationPolicy: Owner
  data:
    - secretKey: inference-api-key
      remoteRef:
        key: model-serving/prod/api-keys
        property: inference
```

### 5.2 Supported Secret Backends

| Backend | Use Case | Auth Method |
|---------|----------|-------------|
| HashiCorp Vault | Enterprise, on-prem | Kubernetes auth, AppRole |
| AWS Secrets Manager | AWS-native | IAM Roles for Service Accounts (IRSA) |
| Google Secret Manager | GCP-native | Workload Identity |
| Azure Key Vault | Azure-native | Managed Identity |
| 1Password | Small teams | 1Password Connect |
| Bitwarden | Open source | Bitwarden SDK |

### 5.3 Secret Rotation

| Secret | Rotation Frequency | Method |
|--------|-------------------|--------|
| Inference API key | 90 days | Vault dynamic secrets + ESO refresh |
| Registry pull token | 30 days (GHCR PAT) | GitHub App auto-rotation |
| TLS certificates | 90 days | cert-manager + Let's Encrypt |
| SSH deploy key (ArgoCD) | 180 days | Manual, documented in runbook |

### 5.4 TLS Certificate Management

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: inference-tls
  namespace: model-serving-prod
spec:
  secretName: inference-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
    - inference.example.com
    - api.example.com
```

cert-manager auto-renews certificates 30 days before expiry. The `inference-tls` secret is referenced by the model-serving service TLS config.

---

<<<<<<< Updated upstream
## 6. Observability Platform Integration

### 6.1 Metrics Pipeline

```
vLLM /metrics ──scrape 10s──> Prometheus ──remote_write──> Mimir ──query──> Grafana
DCGM Exporter ──scrape 15s──> Prometheus
Node Exporter ──scrape 30s──> Prometheus
Envoy Gateway /metrics ──scrape 10s──> Prometheus
```

### 6.2 ServiceMonitor (Already Implemented)

The `charts/model-serving-engine/templates/servicemonitor.yaml` configures Prometheus to scrape vLLM `/metrics` at 10s intervals. This is enabled in all 3 environments.

### 6.3 Grafana Integration

| Integration | Configuration |
|-------------|---------------|
| Grafana OSS | Helm chart `grafana/grafana`, deployed via ArgoCD |
| Grafana Cloud | Remote write to Grafana Cloud Mimir + Loki + Tempo |
| Dashboard provisioning | `observability/grafana-dashboards/vllm-dashboard.json` mounted via ConfigMap |
| Data sources | Prometheus (metrics), Loki (logs), Tempo (traces) |
| Alerting | Grafana Alerting → Alertmanager → PagerDuty/Slack |

### 6.4 Log Aggregation

| Component | Stack | Storage |
|-----------|-------|---------|
| Log collection | Grafana Alloy (agent) | DaemonSet on all nodes |
| Log storage | Loki | S3-compatible (MinIO) or GCS/S3 |
| Log retention | 30 days hot, 1 year cold | Loki retention policy |
| Log queries | LogQL | Grafana Explore |

### 6.5 Distributed Tracing

| Component | Stack | Sampling |
|-----------|-------|----------|
| Trace collection | Grafana Alloy (OTLP receiver) | 10% default, 100% on errors |
| Trace storage | Tempo | S3-compatible backend |
| Trace queries | TraceQL | Grafana Explore |
| Correlation | Exemplars in Prometheus → Tempo trace ID | Auto-linked |

### 6.6 External Observability Platforms

For teams using managed observability:

| Platform | Integration Method |
|----------|-------------------|
| Datadog | Datadog Agent DaemonSet + Helm chart |
| New Relic | New Relic Pixie or OpenTelemetry Collector |
| Dynatrace | Dynatrace Operator + OneAgent |
| Splunk | Splunk Connect for Kubernetes (OpenTelemetry) |
| Honeycomb | OpenTelemetry Collector → Honeycomb refiner |

---

## 7. Alerting & Notification Integration

### 7.1 Alertmanager Routing (Existing)

The existing `observability/alertmanager-routes.yaml` defines:

| Severity | Receiver | Channel | Repeat |
|----------|----------|---------|--------|
| Critical | PagerDuty + Slack | PagerDuty service + #ml-incidents | 1h |
| Warning | Slack | #ml-ops | 2h |
| GPU | Slack | #gpu-ops | 4h |
| Model serving | Slack | #ml-ops | 4h |

### 7.2 Notification Channels

| Channel | Integration | Secret Required |
|---------|-------------|-----------------|
| PagerDuty | Events API v2 | `PAGERDUTY_SERVICE_KEY` (from Vault) |
| Slack | Incoming Webhook | `SLACK_WEBHOOK_URL` (from Vault) |
| Microsoft Teams | Power Automate / Office 365 Connector | `TEAMS_WEBHOOK_URL` |
| Discord | Discord Webhook | `DISCORD_WEBHOOK_URL` |
| Email | SMTP relay | `SMTP_PASSWORD` |
| Opsgenie | Opsgenie API | `OPSGENIE_API_KEY` |

### 7.3 Alert Silencing & Maintenance Windows

```yaml
apiVersion: alertmanager.gke.io/v1beta1
kind: AlertmanagerSilence
metadata:
  name: maintenance-window-prod
  namespace: monitoring
spec:
  matchers:
    - name: namespace
      value: model-serving-prod
  startsAt: "2026-07-10T02:00:00Z"
  endsAt: "2026-07-10T04:00:00Z"
  createdBy: "sre-team"
  comment: "Scheduled maintenance — GPU driver upgrade"
```

### 7.4 On-Call Rotation

| Tool | Integration |
|------|-------------|
| PagerDuty | Schedule-based rotation, escalation policy (5m → 15m → 30m) |
| Opsgenie | Schedule + escalation, integrates with Slack |
| Grafana OnCall | Open source, integrates with Grafana Alerting |

---

## 8. SaaS Fallback Provider Integration

### 8.1 Architecture

When self-hosted models degrade (latency > 2000ms or error rate > 5%), the Envoy AI Gateway circuit breaker fails over to SaaS providers:

```
Self-hosted (priority 0) ──degraded──> SaaS fallback (priority 1)
```
=======
## 6. CI/CD Pipeline Integration
>>>>>>> Stashed changes

### 6.1 Existing CI (GitHub Actions)

The pipeline in `.github/workflows/ci.yaml` has 4 jobs:

| Job | Trigger | Purpose |
|-----|---------|---------|
| `rust-tools` | push/PR to main | Build + test 3 Rust tools, clippy, fmt |
| `helm-lint` | push/PR to main | Lint + template all 4 Helm charts |
| `registry-consistency` | push/PR to main | Validate models/registry.yaml ↔ charts/ ↔ models/ |
| `vram-budget-validation` | push/PR to main | Run vram-budget-calc on all LIVE/STAGED models |

### 6.2 CI → ArgoCD Handoff

```
Developer ──git push──> GitHub
  │
  ├──> GitHub Actions CI (4 jobs)
  │      ├── rust-tools (build+test)
  │      ├── helm-lint (4 charts)
  │      ├── registry-consistency
  │      └── vram-budget-validation
  │
  └──> GitHub branch protection blocks merge until all 4 pass
  │
  └──> PR merged to main
         │
         └──> ArgoCD detects new commit (webhook or 3min poll)
                │
                └──> Sync waves: -3 → -2 → -1 → 0 → 1 → 2
```

### 6.3 CI Secrets

| Secret | Used By | Stored In |
|--------|---------|-----------|
| `HELM_REGISTRY_TOKEN` | helm-lint (if pulling private charts) | GitHub Actions secrets |
| `CARGO_REGISTRY_TOKEN` | rust-tools (if private crates) | GitHub Actions secrets |
| `KUBECONFIG_STAGING` | Post-merge smoke tests | GitHub Actions secrets (OIDC) |

### 6.4 GitHub OIDC → Cloud Auth

For cloud deployments, GitHub Actions uses OIDC to assume cloud roles — no long-lived credentials:

```yaml
permissions:
  id-token: write
  contents: read

jobs:
  deploy-staging:
    steps:
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789:role/github-actions-staging
          aws-region: us-east-1
```

### 6.5 Alternative CI Platforms

| Platform | Migration Effort | Notes |
|----------|-----------------|-------|
| GitLab CI | Low | Translate ci.yaml → .gitlab-ci.yml |
| Jenkins | Medium | Jenkinsfile with same cargo/helm commands |
| CircleCI | Low | config.yml with same steps |
| Tekton | High | K8s-native, pipeline runs in-cluster |
| ArgoCD Workflows | Medium | ArgoCD-native CI |

---

## 7. Multi-Cluster & Multi-Region Integration

### 7.1 Current State

All ApplicationSets target `server: https://kubernetes.default.svc` — the cluster where ArgoCD runs. For multi-cluster:

### 7.2 Cluster Registration

```bash
# Register production cluster
argocd cluster add prod-context --name prod-cluster --label environment=prod

# Register staging cluster
argocd cluster add staging-context --name staging-cluster --label environment=staging

# Register DR cluster
argocd cluster add dr-context --name dr-cluster --label environment=dr
```

### 7.3 Multi-Cluster ApplicationSet

```yaml
apiVersion: argoproj.io/v1alpha1
kind: ApplicationSet
metadata:
  name: model-serving-multi-cluster
  namespace: argocd
spec:
  generators:
    - clusters:
        selector:
          matchLabels:
            environment: prod
  template:
    metadata:
      name: 'model-serving-{{name}}'
    spec:
      project: model-serving
      source:
        repoURL: git@github.com:rustnew/custom-ai-ops.git
        targetRevision: HEAD
        path: charts/model-serving-engine
        helm:
          valueFiles:
            - values.yaml
            - ../../environments/prod/values.yaml
      destination:
        server: '{{server}}'
        namespace: model-serving-prod
```

### 7.4 Multi-Region DR

| Strategy | RPO | RTO | Method |
|----------|-----|-----|--------|
| Active-Active | 0 | 0 | Traffic split across regions via global DNS |
| Active-Passive | 5m | 15m | ArgoCD syncs to DR cluster, DNS failover |
| Pilot Light | 1h | 1h | Minimal services in DR, scale up on failover |
| Backup-Restore | 24h | 4h | Git + Longhorn backups restored manually |

### 7.5 ApplicationSet Destination Override

For multi-region, the destination server is parameterized:

```yaml
generators:
  - clusters:
      selector:
        matchLabels:
          environment: prod
      values:
        region: '{{metadata.labels.region}}'
template:
  spec:
    destination:
      server: '{{server}}'
      namespace: 'model-serving-{{values.region}}'
```

---

## 8. Security Hardening — End-to-End

### 8.1 Git Access Security

| Control | Configuration |
|---------|---------------|
| Branch protection | 2 approvers, CI required, signed commits |
| Deploy key (ArgoCD) | Read-only, dedicated key, rotated 180 days |
| PAT (CI) | Fine-grained, `Contents: Read` only, 30-day expiry |
| Webhook secret | HMAC verified, stored in GitHub secrets |
| Commit signing | GPG or SSH signing required on main |

### 8.2 ArgoCD Security

| Control | Configuration |
|---------|---------------|
| SSO | OIDC (Keycloak, Okta, Google) via `argocd-cm` |
| RBAC | Per-environment roles (§3.6) |
| Repo credentials | SSH deploy key, not PAT |
| UI access | Behind VPN or OIDC only |
| API access | Service accounts with scoped tokens |
| Audit logging | `argocd-server` logs to Loki, 1-year retention |
| Pod security | `runAsNonRoot: true`, `readOnlyRootFilesystem: true` |

### 8.3 Cluster Security

| Control | Tool | Configuration |
|---------|------|---------------|
| Pod Security Standards | Kubernetes PSA | `restricted` on model-serving-* namespaces |
| Network Policies | Cilium / Calico | Deny-all default, allow explicit |
| Admission control | OPA Gatekeeper / Kyverno | Enforce image registry, labels, resource limits |
| Runtime security | Falco | Detect anomalous pod behavior |
| Vulnerability scanning | Trivy / Grype | Scan images at build + admission |
| Supply chain | Sigstore Cosign | Sign + verify images (§4.4) |
| Secrets encryption | etcd encryption at rest | KMS-backed |

### 8.4 Supply Chain Security (SLSA Level 3)

| Step | Tool | Output |
|------|------|--------|
| Source integrity | Git commit signing | Signed commits on main |
| Build provenance | GitHub Actions OIDC + SLSA generator | Attestation in Sigstore |
| Image signing | Cosign in CI | Signed image in registry |
| Image verification | Sigstore Policy Controller | Admission webhook rejects unsigned |
| SBOM | Syft in CI | SBOM artifact per image |
| Vulnerability scan | Grype in CI | Fail build on CRITICAL CVEs |

### 8.5 Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: model-serving-default-deny
  namespace: model-serving-prod
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
```

### 8.6 Secret Encryption in Transit

| Path | Encryption |
|------|-----------|
| Git ↔ ArgoCD | SSH (AES-256) or HTTPS (TLS 1.3) |
| ArgoCD ↔ K8s API | TLS 1.3 |
| Vault ↔ ESO | TLS 1.3 + Kubernetes auth (short-lived token) |
| ESO ↔ K8s Secret | etcd encryption at rest (AES-CBC) |

---

## 9. Integration Topology Diagram

```mermaid
flowchart TB
    subgraph Git["Git Provider (GitHub)"]
        REPO["custom-ai-ops repo<br/>main branch"]
        CI["GitHub Actions CI<br/>4 jobs: rust, helm, registry, vram"]
    end

    subgraph Control["Control Cluster (no GPU)"]
        ARGOCD["ArgoCD<br/>AppSets + Projects + RBAC"]
        ESO["External Secrets Operator"]
        VAULT["Vault / AWS SM<br/>(external)"]
    end

    subgraph Worker["Worker Cluster (GPU nodes)"]
        subgraph Infra["Infrastructure (wave -2/-1)"]
            GPU["NVIDIA GPU Operator"]
            LH["Longhorn Storage"]
            SWAPOFF["swapoff DaemonSet"]
        end
        subgraph Serving["Model Serving (wave 0)"]
            VLLM["vLLM Pods<br/>QoS Guaranteed"]
        end
    end

    subgraph External["External Platforms"]
        REG["Container Registry<br/>GHCR / Harbor"]
    end

    REPO -->|webhook| ARGOCD
    CI -->|blocks merge| REPO
    ARGOCD -->|sync waves| Worker
    ESO -->|sync secrets| VAULT
    ESO -->|creates K8s Secrets| Worker
    REG -->|pull images| Worker
```

---

## 10. Integration Checklist

### 10.1 Pre-Integration (Repository)

- [ ] Fix repoURL in all ApplicationSets to match actual Git remote
- [ ] Create ArgoCD AppProject manifests (`apps/argocd-projects.yaml`)
- [ ] Create repo credential Secret manifest (`apps/argocd-repo-secret.yaml` template)
- [ ] Add NetworkPolicy manifests (`charts/model-serving-engine/templates/networkpolicies.yaml`)
- [ ] Add ExternalSecret manifests (`charts/model-serving-engine/templates/externalsecret.yaml`)
- [ ] Add SecretStore manifest (`charts/model-serving-engine/templates/secretstore.yaml`)
- [ ] Add image pull secret reference in environment values
- [ ] Add ArgoCD Image Updater annotations to ApplicationSets
- [ ] Create `apps/argocd-rbac-cm.yaml` with RBAC policies
- [ ] Create `apps/argocd-notifications-cm.yaml` with Slack templates
- [ ] Add ClusterImagePolicy for Cosign verification
- [ ] Add cert-manager Certificate manifests for TLS

### 10.2 ArgoCD Installation

- [ ] Install ArgoCD on Control Cluster
- [ ] Configure SSO (OIDC) for ArgoCD
- [ ] Apply repo credential Secret
- [ ] Apply AppProjects
- [ ] Apply argocd-cm (health checks)
- [ ] Apply argocd-rbac-cm (RBAC)
- [ ] Apply argocd-notifications-cm (Slack)
- [ ] Apply ApplicationSets (dev first, then staging, then prod)
- [ ] Verify sync waves execute in order
- [ ] Verify all resources reach Healthy state

### 10.3 External Secrets

- [ ] Install External Secrets Operator
- [ ] Create SecretStore pointing to Vault/AWS SM
- [ ] Create ExternalSecret for each secret (API keys)
- [ ] Verify secrets sync to Kubernetes
- [ ] Verify pods can mount secrets

### 10.4 Container Registry

- [ ] Create registry pull secret
- [ ] Configure imagePullSecrets in environment values
- [ ] (Optional) Install ArgoCD Image Updater
- [ ] (Optional) Configure Cosign image signing in CI
- [ ] (Optional) Install Sigstore Policy Controller

### 10.5 Security

- [ ] Enable branch protection on main (2 approvers, CI required, signed commits)
- [ ] Configure ArgoCD SSO
- [ ] Apply NetworkPolicies
- [ ] Enable Pod Security Standards (restricted)
- [ ] Install Falco for runtime security
- [ ] Configure Trivy image scanning in CI
- [ ] Enable etcd encryption at rest
- [ ] Verify all secrets come from Vault (no secrets in Git)

### 10.6 Multi-Cluster (If Applicable)

- [ ] Register worker clusters with ArgoCD
- [ ] Update ApplicationSets with cluster generator
- [ ] Configure DNS-based failover (External DNS)
- [ ] Test DR failover procedure
- [ ] Document DR runbook

### 10.7 Validation

- [ ] Run smoke tests post-deploy (`tests/smoke/vllm-smoke-test.sh`)
- [ ] Run load tests (`tests/load/load-test.js` with k6)
- [ ] Test ArgoCD rollback (revert commit → auto-sync)
- [ ] Test self-heal (manual kubectl edit → ArgoCD re-syncs)
- [ ] Test prune (delete resource in Git → ArgoCD removes from cluster)

---

## Appendix A: External Platform Compatibility Matrix

| Platform | Integration Type | Auth | ArgoCD | CI | Registry | Secrets |
|----------|-----------------|------|--------|-----|----------|---------|
| GitHub | Git provider | PAT/SSH/Deploy Key | ✅ | GitHub Actions | GHCR | — |
| GitLab | Git provider | PAT/Deploy Token | ✅ | GitLab CI | Built-in | — |
| Bitbucket | Git provider | App Password | ✅ | Bitbucket Pipelines | — | — |
| Azure DevOps | Git provider | PAT | ✅ | Azure Pipelines | ACR | Key Vault |
| AWS | Cloud | IRSA | ✅ | CodeBuild | ECR | Secrets Manager |
| GCP | Cloud | Workload Identity | ✅ | Cloud Build | GCR/Artifact Registry | Secret Manager |
| Azure | Cloud | Managed Identity | ✅ | Azure Pipelines | ACR | Key Vault |
| Vault | Secrets | K8s Auth / AppRole | — | — | — | ✅ |

---

## Appendix B: Secret Inventory

| Secret Name | Namespace | Source | Used By | Sync Wave |
|-------------|----------|--------|---------|-----------|
| `custom-ai-ops-repo` | argocd | Manual (SSH key) | ArgoCD repo connection | Bootstrap |
| `registry-pull-secret` | model-serving-* | ESO (Vault) | Pod image pulls | -3 |
| `model-serving-secrets` | model-serving-* | ESO (Vault) | Inference API keys | -3 |
| `inference-tls` | model-serving-prod | cert-manager | Model serving TLS | -2 |

---

## Appendix C: Sync Wave — Complete Resource Map

| Wave | Resource | Namespace | Chart | Source |
|------|----------|-----------|-------|--------|
| -3 | SecretStore | model-serving-* | model-serving-engine | ESO |
| -3 | ExternalSecret | model-serving-* | model-serving-engine | ESO |
| -3 | registry-pull-secret | model-serving-* | model-serving-engine | ESO |
| -2 | Longhorn PVCs | model-serving-* | model-serving-engine | Helm |
| -2 | swapoff DaemonSet | (all GPU nodes) | model-serving-engine | Helm |
| -2 | TLS Certificate | model-serving-prod | cert-manager | cert-manager |
| -1 | NVIDIA GPU Operator | nvidia-gpu-operator | addons/ | Helm |
| 0 | vLLM StatefulSet | model-serving-* | model-serving-engine | Helm |
| 0 | KEDA ScaledObject | model-serving-* | model-serving-engine | Helm |
| 0 | PDB | model-serving-* | model-serving-engine | Helm |
| 1 | NetworkPolicies | model-serving-* | model-serving-engine | Helm |
| 2 | Smoke test Job | model-serving-* | model-serving-engine | Helm |
| 2 | ArgoCD notification | argocd | — | argocd-notifications |

---

*End of report.*