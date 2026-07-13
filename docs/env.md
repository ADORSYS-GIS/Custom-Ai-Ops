# Environment Variables, Secrets, and External Connections Reference

> Complete inventory of every environment variable, API key, secret, external URL, and connection endpoint the project expects to function properly. Each entry documents its role, where it must be placed, and how to configure it.

---

## Table of Contents

1. [Quick Reference — All Secrets and Keys](#1-quick-reference)
2. [GitHub — Repository Access and CI](#2-github)
3. [ArgoCD — Repository Credentials and Notifications](#3-argocd)
4. [External Secrets Operator — Secret Backend](#4-eso)
5. [SaaS LLM Fallback Providers (7)](#5-saas-llm)
6. [Container Registries](#6-registries)
7. [Alerting — PagerDuty and Slack](#7-alerting)
8. [Observability — Prometheus, Grafana, Alertmanager](#8-observability)
9. [LMCache — Redis and S3 Backends](#9-lmcache)
10. [cert-manager — Let's Encrypt](#10-cert-manager)
11. [KEDA — Prometheus Connection](#11-keda)
12. [NVIDIA GPU Operator — DCGM Exporter](#12-nvidia)
13. [Longhorn — Distributed Storage](#13-longhorn)
14. [Test Scripts — Environment Variables](#14-tests)
15. [Runtime Environment Variables (Pods)](#15-runtime-env)
16. [Helm Repository URLs (6)](#16-helm-repos)
17. [Bootstrap Order — Where to Place Each Secret](#17-bootstrap-order)
18. [Verification Checklist](#18-checklist)
19. [HTTP Headers — Gateway Conventions](#19-http-headers)

---

<a id="1-quick-reference"></a>
## 1. Quick Reference — All Secrets and Keys

| # | Secret Name | Key / Variable | Where Stored | Used By | Namespace |
|---|---|---|---|---|---|
| 1 | `custom-ai-ops-repo` | `password` (GitHub PAT) | ArgoCD Secret | ArgoCD repo access | `argocd` |
| 2 | `custom-ai-ops-repo` | `url` | ArgoCD Secret | ArgoCD repo access | `argocd` |
| 3 | `custom-ai-ops-repo` | `username` | ArgoCD Secret | ArgoCD repo access | `argocd` |
| 4 | `argocd-notifications-secret` | `slack-webhook-url` | ArgoCD Secret | ArgoCD Notifications | `argocd` |
| 5 | `argocd-notifications-secret` | `pagerduty-integration-key` | ArgoCD Secret | ArgoCD Notifications | `argocd` |
| 6 | `ai-gateway-saas-keys` | `openai-gpt4-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 7 | `ai-gateway-saas-keys` | `anthropic-claude-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 8 | `ai-gateway-saas-keys` | `google-vertex-ai-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 9 | `ai-gateway-saas-keys` | `azure-openai-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 10 | `ai-gateway-saas-keys` | `mistral-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 11 | `ai-gateway-saas-keys` | `cohere-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 12 | `ai-gateway-saas-keys` | `aws-bedrock-api-key` | K8s Secret (via ESO) | AI Gateway fallback | `envoy-gateway-system` |
| 13 | `alertmanager-config` | `config.yaml` (templated) | K8s Secret (via ESO) | Alertmanager | `monitoring` |
| 14 | `argocd-image-updater-git` | `GITHUB_TOKEN` | K8s Secret (via ESO) | ArgoCD Image Updater | `argocd` |
| 15 | `registry-credentials` | `.dockerconfigjson` | K8s Secret (via ESO) | Pod image pulls | `model-serving-prod` |
| 16 | `inference-tls` | `tls.crt` + `tls.key` | K8s Secret (via cert-manager) | AI Gateway TLS | `envoy-gateway-system` |
| 17 | `letsencrypt-prod-account-key` | `tls.key` | K8s Secret (via cert-manager) | ACME account | `cert-manager` |
| 18 | `letsencrypt-staging-account-key` | `tls.key` | K8s Secret (via cert-manager) | ACME account (staging) | `cert-manager` |

### Remote Secrets (in AWS Secrets Manager / Vault / GCP SM)

| Remote Path | Description | Consumed By |
|---|---|---|
| `saas/openai-api-key` | OpenAI GPT-4 API key | ExternalSecret `saas-api-keys` |
| `saas/anthropic-api-key` | Anthropic Claude API key | ExternalSecret `saas-api-keys` |
| `saas/google-vertex-ai-key` | Google Vertex AI service account JSON | ExternalSecret `saas-api-keys` |
| `saas/azure-openai-key` | Azure OpenAI API key | ExternalSecret `saas-api-keys` |
| `saas/mistral-api-key` | Mistral AI API key | ExternalSecret `saas-api-keys` |
| `saas/cohere-api-key` | Cohere API key | ExternalSecret `saas-api-keys` |
| `saas/aws-bedrock-key` | AWS Bedrock access key | ExternalSecret `saas-api-keys` |
| `alerting/pagerduty-service-key` | PagerDuty Events API v2 integration key | ExternalSecret `alertmanager-config` |
| `alerting/slack-webhook-url` | Slack incoming webhook URL | ExternalSecret `alertmanager-config` |
| `registry/github-pat` | GitHub PAT (for Image Updater + ghcr.io pull) | ExternalSecret `argocd-image-updater-token` + `registry-pull-secret` |
| `registry/docker-username` | Docker Hub username | ExternalSecret `registry-pull-secret` |
| `registry/docker-password` | Docker Hub password/token | ExternalSecret `registry-pull-secret` |

---

<a id="2-github"></a>
## 2. GitHub — Repository Access and CI

### 2.1 GitHub Repository URL

| Variable | Value | Where Used |
|---|---|---|
| Git remote (SSH) | `git@github.com:rustnew/custom-ai-ops.git` | Local git, `git push` |
| Git remote (HTTPS) | `https://github.com/rustnew/custom-ai-ops.git` | All ArgoCD AppSets (11 references across dev/staging/prod) |
| GitHub username | `rustnew` | ArgoCD repo credential Secret |
| Cargo.toml repository | `https://github.com/Custom-Ai-Ops/Custom-Ai-Ops` | Rust workspace metadata |

**Where to configure**: The HTTPS URL is hardcoded in all 3 AppSet files (`apps/argocd-appset-{dev,staging,prod}.yaml`). No change needed unless you fork the repo.

### 2.2 GitHub Personal Access Token (PAT)

| Variable | Value | Where Placed |
|---|---|---|
| `GITHUB_PAT_TOKEN` | Your PAT string | ArgoCD Secret `custom-ai-ops-repo` (field `password`) |
| `GITHUB_TOKEN` | Same PAT (or separate) | K8s Secret `argocd-image-updater-git` (via ESO from `registry/github-pat`) |
| `github_pat` | Same PAT | Remote secret `registry/github-pat` in AWS SM / Vault |

**PAT scopes required**:
- **Read-only** (ArgoCD repo access): classic PAT with `repo:read` + `read:org`, OR fine-grained with `contents:read`
- **Read+Write** (Image Updater write-back): classic PAT with `repo`, OR fine-grained with `contents:write`

**How to generate**:
```
GitHub → Settings → Developer settings → Personal access tokens →
  Tokens (classic) → Generate new token → scopes: repo, read:org
```

**Where to place it**:
1. **ArgoCD repo credential** — `apps/argocd-repo-credentials.yaml`, field `password: <GITHUB_PAT_TOKEN>`. Replace the placeholder. For production, use ExternalSecret instead of hardcoding.
2. **AWS Secrets Manager** — Create secret `registry/github-pat` with value = PAT string. ESO pulls it into `argocd-image-updater-git` and `registry-credentials` Secrets.
3. **GitHub Actions** — The CI workflow (`.github/workflows/ci.yaml`) uses `actions/checkout@v4` which uses the default `GITHUB_TOKEN` automatically. No manual configuration needed.

### 2.3 GitHub Actions CI

| Variable | Source | Role |
|---|---|---|
| `GITHUB_TOKEN` | Auto-provided by GitHub Actions | Checkout, push (if needed) |

No manual configuration required. The CI runs 4 jobs: `rust-tools`, `helm-lint`, `registry-consistency`, `vram-budget-validation`.

---

<a id="3-argocd"></a>
## 3. ArgoCD — Repository Credentials and Notifications

### 3.1 Repository Credential Secret

**File**: `apps/argocd-repo-credentials.yaml`
**Namespace**: `argocd`
**Sync wave**: `-11` (must exist before AppProjects at `-10`)

| Field | Value | Description |
|---|---|---|
| `url` | `https://github.com/rustnew/custom-ai-ops.git` | Repo URL |
| `username` | `rustnew` | GitHub username |
| `password` | `<GITHUB_PAT_TOKEN>` | Replace with real PAT |
| `insecure` | `false` | Use HTTPS with TLS verification |

**How to apply**:
```bash
# Option A: Direct apply (replace placeholder first)
sed -i 's/<GITHUB_PAT_TOKEN>/ghp_your_real_token/' apps/argocd-repo-credentials.yaml
kubectl apply -f apps/argocd-repo-credentials.yaml

# Option B: Via kubectl (recommended for production)
kubectl create secret generic custom-ai-ops-repo \
  --namespace argocd \
  --from-literal=url=https://github.com/rustnew/custom-ai-ops.git \
  --from-literal=username=rustnew \
  --from-literal=password=ghp_your_real_token \
  --from-literal=insecure=false \
  -l argocd.argoproj.io/secret-type=repository
```

### 3.2 SSH Known Hosts

**File**: `apps/argocd-repo-credentials.yaml` (ConfigMap `argocd-ssh-known-hosts-cm`)
**Namespace**: `argocd`

Contains GitHub's real SSH host keys (ed25519, ecdsa, rsa) fetched via `ssh-keyscan`. No configuration needed — these are public keys.

### 3.3 ArgoCD Notifications

**File**: `apps/argocd-notifications.yaml`
**Namespace**: `argocd`
**Sync wave**: `-11`

| Secret Key | Value | Description |
|---|---|---|
| `slack-webhook-url` | `<SLACK_WEBHOOK_URL>` | Slack incoming webhook URL (e.g., `https://hooks.slack.com/services/T000/B000/XXX`) |
| `pagerduty-integration-key` | `<PAGERDUTY_INTEGRATION_KEY>` | PagerDuty Events API v2 integration key |

**How to configure**:
```bash
# Option A: Direct apply (replace placeholders)
sed -i 's/<SLACK_WEBHOOK_URL>/https:\/\/hooks.slack.com\/services\/T000\/B000\/XXX/' apps/argocd-notifications.yaml
sed -i 's/<PAGERDUTY_INTEGRATION_KEY>/your_integration_key/' apps/argocd-notifications.yaml
kubectl apply -f apps/argocd-notifications.yaml

# Option B: Via kubectl
kubectl create secret generic argocd-notifications-secret \
  --namespace argocd \
  --from-literal=slack-webhook-url=https://hooks.slack.com/services/T000/B000/XXX \
  --from-literal=pagerduty-integration-key=your_integration_key
```

**Notifications sent**:
- Slack `#ml-ops`: sync succeeded, sync failed, sync running, health degraded (all apps)
- PagerDuty: sync failed (production apps only, via `selector: app.metadata.annotations.env == "production"`)

### 3.4 ApplicationSets

| AppSet File | AppSet Name | Entries | Sync Waves |
|---|---|---|---|
| `apps/argocd-appset-dev.yaml` | `model-serving-dev` | engine, gateway, secrets | 0, 1, -3 |
| `apps/argocd-appset-staging.yaml` | `model-serving-staging` | engine, gateway, secrets | 0, 1, -3 |
| `apps/argocd-appset-prod.yaml` | `model-serving-prod` | engine, gateway, secrets, 6 addons | 0, 1, -3, -2, -1 |

**`model-serving-secrets` AppSet entry**: In all 3 AppSet files, the secrets entry deploys `apps/external-secrets.yaml` (path: `apps`, directory.include: `external-secrets.yaml`, sync-wave -3, ServerSideApply=true). This is the AppSet that creates the 4 ExternalSecrets + ClusterSecretStore.

---

<a id="4-eso"></a>
## 4. External Secrets Operator — Secret Backend

### 4.1 ClusterSecretStore

**File**: `apps/external-secrets.yaml`
**Kind**: `ClusterSecretStore` (cluster-scoped, not namespace-scoped)
**Name**: `aws-secrets-manager`

| Parameter | Value | Description |
|---|---|---|
| `provider.aws.service` | `SecretsManager` | AWS Secrets Manager |
| `provider.aws.region` | `us-east-1` | AWS region (change to your region) |
| `provider.aws.auth.jwt.serviceAccountRef.name` | `model-serving-engine` | IRSA service account |
| `provider.aws.auth.jwt.serviceAccountRef.namespace` | `model-serving-prod` | IRSA namespace |

**Prerequisites**:
1. ESO installed (addon `addons/external-secrets/Chart.yaml`, sync-wave `-1`)
2. AWS IAM role with `secretsmanager:GetSecretValue` for the listed secrets
3. IRSA (IAM Roles for Service Accounts) annotation on the `model-serving-engine` service account:
   ```yaml
   annotations:
     eks.amazonaws.com/role-arn: arn:aws:iam::<ACCOUNT_ID>:role/model-serving-eso
   ```

**Alternative backends** (commented in the file):
- **HashiCorp Vault**: Uncomment the `SecretStore` block, set `server: "https://vault.vault.svc.cluster.local:8200"`, `path: "kv"`, `version: "v2"`, configure Kubernetes auth with `role: "model-serving"`
- **Google Secret Manager**: Change provider to `gcpsm`, set `projectID`
- **Azure Key Vault**: Change provider to `azurekv`, set `vaultUrl`, configure service principal auth

### 4.2 ExternalSecrets (4)

| ExternalSecret | Namespace | Target Secret | Keys | Remote Refs |
|---|---|---|---|---|
| `saas-api-keys` | `envoy-gateway-system` | `ai-gateway-saas-keys` | 7 SaaS API keys | `saas/openai-api-key`, `saas/anthropic-api-key`, `saas/google-vertex-ai-key`, `saas/azure-openai-key`, `saas/mistral-api-key`, `saas/cohere-api-key`, `saas/aws-bedrock-key` |
| `alertmanager-config` | `monitoring` | `alertmanager-config` | `config.yaml` (templated with `{{ .pagerduty_key }}` and `{{ .slack_webhook }}`) | `alerting/pagerduty-service-key`, `alerting/slack-webhook-url` |
| `argocd-image-updater-token` | `argocd` | `argocd-image-updater-git` | `GITHUB_TOKEN` | `registry/github-pat` |
| `registry-pull-secret` | `model-serving-prod` | `registry-credentials` | `.dockerconfigjson` (docker registry config) | `registry/github-pat`, `registry/docker-username`, `registry/docker-password` |

**How to create remote secrets in AWS Secrets Manager**:
```bash
# SaaS API keys
aws secretsmanager create-secret --name saas/openai-api-key --secret-string "sk-..."
aws secretsmanager create-secret --name saas/anthropic-api-key --secret-string "sk-ant-..."
aws secretsmanager create-secret --name saas/google-vertex-ai-key --secret-string '{"type":"service_account",...}'
aws secretsmanager create-secret --name saas/azure-openai-key --secret-string "your-azure-key"
aws secretsmanager create-secret --name saas/mistral-api-key --secret-string "your-mistral-key"
aws secretsmanager create-secret --name saas/cohere-api-key --secret-string "your-cohere-key"
aws secretsmanager create-secret --name saas/aws-bedrock-key --secret-string "your-bedrock-key"

# Alerting
aws secretsmanager create-secret --name alerting/pagerduty-service-key --secret-string "your-pagerduty-key"
aws secretsmanager create-secret --name alerting/slack-webhook-url --secret-string "https://hooks.slack.com/services/T000/B000/XXX"

# Registry
aws secretsmanager create-secret --name registry/github-pat --secret-string "ghp_your_pat"
aws secretsmanager create-secret --name registry/docker-username --secret-string "your-docker-hub-username"
aws secretsmanager create-secret --name registry/docker-password --secret-string "your-docker-hub-token"
```

### 4.3 ESO CRD Installation (one-time)

If ESO is not installed via the Helm addon, install the CRDs manually:
```bash
kubectl apply -f https://raw.githubusercontent.com/external-secrets/external-secrets/main/deploy/crds/bundle.yaml
```

This URL is referenced in the header comment of `apps/external-secrets.yaml`.

---

<a id="5-saas-llm"></a>
## 5. SaaS LLM Fallback Providers (7)

The AI Gateway uses these providers as fallback when self-hosted models are unavailable or latency exceeds 2000ms. Keys are pulled from the `ai-gateway-saas-keys` Secret (created by ExternalSecret `saas-api-keys`).

| Provider | Secret Key | Remote Path | API Endpoint | Role |
|---|---|---|---|---|
| **OpenAI** (GPT-4) | `openai-gpt4-api-key` | `saas/openai-api-key` | `https://api.openai.com/v1` | Primary SaaS fallback (priority 1) |
| **Anthropic** (Claude) | `anthropic-claude-api-key` | `saas/anthropic-api-key` | `https://api.anthropic.com/v1` | Alternative fallback |
| **Google Vertex AI** | `google-vertex-ai-key` | `saas/google-vertex-ai-key` | `https://{REGION}-aiplatform.googleapis.com` | Alternative fallback |
| **Azure OpenAI** | `azure-openai-api-key` | `saas/azure-openai-key` | `https://{RESOURCE}.openai.azure.com` | Alternative fallback |
| **Mistral AI** | `mistral-api-key` | `saas/mistral-api-key` | `https://api.mistral.ai/v1` | Alternative fallback |
| **Cohere** | `cohere-api-key` | `saas/cohere-api-key` | `https://api.cohere.ai/v1` | Alternative fallback |
| **AWS Bedrock** | `aws-bedrock-api-key` | `saas/aws-bedrock-key` | `https://bedrock-runtime.{REGION}.amazonaws.com` | Alternative fallback |

**Where to configure**:
1. Create API keys on each provider's dashboard
2. Store them in AWS Secrets Manager (or Vault/GCP SM) at the paths listed above
3. ESO automatically syncs them into the `ai-gateway-saas-keys` Secret in `envoy-gateway-system`
4. The AI Gateway chart references this Secret in its `fallback.saasBackends[].apiKeySecret` configuration

**Where referenced in code**: `charts/ai-gateway/values.yaml` → `fallback.saasBackends[].apiKeySecret: ai-gateway-saas-keys`

---

<a id="6-registries"></a>
## 6. Container Registries

### 6.1 ghcr.io (GitHub Container Registry)

| Parameter | Value | Description |
|---|---|---|
| Registry URL | `ghcr.io` | GitHub Container Registry |
| Username | `rustnew` | GitHub username (hardcoded in ExternalSecret template) |
| Password | `{{ .github_pat }}` | GitHub PAT (from `registry/github-pat` remote secret) |

### 6.2 docker.io (Docker Hub)

| Parameter | Value | Description |
|---|---|---|
| Registry URL | `docker.io` | Docker Hub |
| Username | `{{ .docker_username }}` | From `registry/docker-username` remote secret |
| Password | `{{ .docker_password }}` | From `registry/docker-password` remote secret |

### 6.3 Pull Secret

**Created by**: ExternalSecret `registry-pull-secret` → K8s Secret `registry-credentials` (type `kubernetes.io/dockerconfigjson`)
**Namespace**: `model-serving-prod`
**Used by**: StatefulSet pods (referenced in `global.imagePullSecrets`)

**How to configure**:
1. Create a Docker Hub access token: `Docker Hub → Account Settings → Security → New Access Token`
2. Store in AWS SM: `registry/docker-username` and `registry/docker-password`
3. Store GitHub PAT: `registry/github-pat`
4. ESO creates the `registry-credentials` Secret automatically

### 6.4 ArgoCD Image Updater

| Parameter | Value | Description |
|---|---|---|
| Secret name | `argocd-image-updater-git` | In `argocd` namespace |
| Key | `GITHUB_TOKEN` | GitHub PAT with `contents:write` |
| Remote path | `registry/github-pat` | Same PAT used for ghcr.io pull |

**Role**: Image Updater commits new image tags back to the Git repo. Requires write access to `rustnew/custom-ai-ops`.

---

<a id="7-alerting"></a>
## 7. Alerting — PagerDuty and Slack

### 7.1 PagerDuty

| Parameter | Value | Where Placed |
|---|---|---|
| Integration key | `<PAGERDUTY_SERVICE_KEY>` or `<PAGERDUTY_INTEGRATION_KEY>` | Two places (see below) |
| Events API URL | `https://events.pagerduty.com/v2/enqueue` | Configured in Alertmanager (implicit) |

**Where to configure**:
1. **Alertmanager** — Remote secret `alerting/pagerduty-service-key` → ESO templates it into `alertmanager-config` Secret as `{{ .pagerduty_key }}` in the `pagerduty_configs` section
2. **ArgoCD Notifications** — Secret `argocd-notifications-secret`, key `pagerduty-integration-key` (can be same or different PagerDuty service)

**How to get the key**:
```
PagerDuty → Services → New Service → Integration: Events API v2 → Copy integration key
```

**Routing**:
- `severity: critical` → PagerDuty + Slack `#ml-incidents`
- ArgoCD sync failed (production) → PagerDuty

### 7.2 Slack

| Parameter | Value | Where Placed |
|---|---|---|
| Webhook URL | `<SLACK_WEBHOOK_URL>` | Two places (see below) |
| API URL | `https://slack.com/api` | ArgoCD Notifications ConfigMap (hardcoded) |

**Where to configure**:
1. **Alertmanager** — Remote secret `alerting/slack-webhook-url` → ESO templates it into `alertmanager-config` Secret as `{{ .slack_webhook }}` in all `slack_configs` sections
2. **ArgoCD Notifications** — Secret `argocd-notifications-secret`, key `slack-webhook-url`

**How to get the webhook URL**:
```
Slack → Apps → Incoming Webhooks → Add to Slack → Choose channel → Copy webhook URL
```

**Channels used**:
| Channel | Receives |
|---|---|
| `#ml-incidents` | Critical alerts (PagerDuty + Slack) |
| `#ml-ops` | Warning alerts, model-serving alerts, ArgoCD sync notifications |
| `#gpu-ops` | GPU-specific alerts (thermal, VRAM, utilization) |

---

<a id="8-observability"></a>
## 8. Observability — Prometheus, Grafana, Alertmanager

### 8.1 Prometheus

| Parameter | Value | Where Configured |
|---|---|---|
| Scrape interval | `10s` | `addons/prometheus-stack/Chart.yaml` → `prometheus.prometheusSpec.scrapeInterval` |
| Scrape timeout | `5s` | ServiceMonitor in `charts/model-serving-engine/templates/servicemonitor.yaml` |
| Retention | `30d` | `addons/prometheus-stack/Chart.yaml` |
| `serviceMonitorSelectorNilUsesHelmValues` | `false` | CRITICAL — allows discovery of model-serving ServiceMonitors |
| `ruleSelectorNilUsesHelmValues` | `false` | CRITICAL — allows loading PrometheusRules from `observability/` |
| Prometheus URL (for KEDA) | `http://prometheus-server.monitoring.svc.cluster.local:9090` | `environments/{prod,staging}/values.yaml` → `autoscaling.keda.prometheusAddress` |

**No external credentials needed** — Prometheus is internal to the cluster.

### 8.2 Grafana

| Parameter | Value | Where Configured |
|---|---|---|
| Admin password | `admin123` | `addons/prometheus-stack/Chart.yaml` → `grafana.adminPassword` |
| Dashboard file | `model-serving-dashboard.json` | `observability/grafana-dashboards/` (18 panels) |
| Grafana host | `grafana.example.com` | `addons/prometheus-stack/Chart.yaml` → `grafana.ingress.hosts` |

**Change the admin password for production**:
```yaml
# In addons/prometheus-stack/Chart.yaml
grafana:
  adminPassword: <your-secure-password>
```

### 8.3 Alertmanager

| Parameter | Value | Where Configured |
|---|---|---|
| Config source | `useExistingSecret.name: alertmanager-config` | `addons/prometheus-stack/Chart.yaml` |
| Config key | `config.yaml` | ESO creates this Secret with templated config |
| Alertmanager host | `alertmanager.example.com` | `addons/prometheus-stack/Chart.yaml` → `alertmanager.ingress.hosts` |

The Alertmanager config is NOT stored as a plain file — it is templated by ESO with real PagerDuty and Slack credentials. See §4.2 and §7.

**Internal webhook URL**: `http://alertmanager-webhook.monitoring:9093/alerts` — used by internal services (e.g., ArgoCD notifications, custom webhook senders) to push alerts directly to Alertmanager. This is the in-cluster Service DNS name.

---

<a id="9-lmcache"></a>
## 9. LMCache — Redis and S3 Backends

### 9.1 Redis (L3 distributed cache — prod only)

| Parameter | Value | Where Configured |
|---|---|---|
| `lmcache.redis.enabled` | `true` (prod) / `false` (staging, dev) | `environments/prod/values.yaml` |
| `lmcache.redis.host` | `redis-cache.monitoring.svc.cluster.local` | `environments/prod/values.yaml` |
| `lmcache.redis.port` | `6379` | `environments/prod/values.yaml` |

**Where to deploy Redis**: Redis must be deployed separately (not included in this repo). Recommended: `bitnami/redis` Helm chart in the `monitoring` namespace.

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install redis-cache bitnami/redis \
  --namespace monitoring \
  --set architecture=standalone \
  --set auth.enabled=false \
  --set persistence.size=50Gi
```

### 9.2 S3 (L3 distributed cache — optional)

| Parameter | Value | Where Configured |
|---|---|---|
| `lmcache.s3.enabled` | `false` (default) | `charts/model-serving-engine/values.yaml` |
| `lmcache.s3.endpoint` | `https://s3.us-east-1.amazonaws.com` | Enable and set in `environments/prod/values.yaml` |
| `lmcache.s3.bucket` | `your-lmcache-bucket` | Set in `environments/prod/values.yaml` |
| `lmcache.s3.region` | `us-east-1` | Set in `environments/prod/values.yaml` |

**IAM permissions needed**: `s3:GetObject`, `s3:PutObject`, `s3:DeleteObject` on the bucket.

### 9.3 LMCache Observability

| Parameter | Value | Where Configured |
|---|---|---|
| Metrics host | `0.0.0.0` | `charts/model-serving-engine/templates/lmcache-configmap.yaml` |
| Metrics port | `8330` | ConfigMap + Service `lmcache-service.yaml` |
| Metrics path | `/metrics` | Scrape via the LMCache Service (port 8330) |

---

<a id="10-cert-manager"></a>
## 10. cert-manager — Let's Encrypt

### 10.1 Let's Encrypt Production

| Parameter | Value | Where Configured |
|---|---|---|
| ACME server | `https://acme-v02.api.letsencrypt.org/directory` | `addons/cert-manager/Chart.yaml` → ClusterIssuer `letsencrypt-prod` |
| Email | `ops@example.com` | **REPLACE** with your operations email |
| Account key secret | `letsencrypt-prod-account-key` | Auto-created by cert-manager |
| HTTP01 solver ingress class | `envoy-gateway` | ClusterIssuer solver |
| Default issuer | `letsencrypt-prod` | `addons/cert-manager/Chart.yaml` → `ingressShim.defaultIssuerName` |

### 10.2 Let's Encrypt Staging

| Parameter | Value | Where Configured |
|---|---|---|
| ACME server | `https://acme-staging-v02.api.letsencrypt.org/directory` | ClusterIssuer `letsencrypt-staging` |
| Email | `ops@example.com` | **REPLACE** with your operations email |
| Account key secret | `letsencrypt-staging-account-key` | Auto-created by cert-manager |

**How to configure**:
1. Replace `ops@example.com` with your real email in `addons/cert-manager/Chart.yaml`
2. cert-manager automatically creates the ACME account and obtains certificates
3. The AI Gateway chart creates a `Certificate` resource for `inference.example.com` → cert-manager produces the `inference-tls` Secret

**External endpoint**: Let's Encrypt ACME API (no credentials needed — cert-manager handles the challenge automatically via HTTP01).

---

<a id="11-keda"></a>
## 11. KEDA — Prometheus Connection

| Parameter | Value | Where Configured |
|---|---|---|
| KEDA namespace | `keda-system` | `addons/keda/Chart.yaml` |
| Watch namespaces | `model-serving-prod,model-serving-staging` | `addons/keda/Chart.yaml` → `watchNamespace` |
| Prometheus address | `http://prometheus-server.monitoring.svc.cluster.local:9090` | `environments/{prod,staging}/values.yaml` → `autoscaling.keda.prometheusAddress` |
| Polling interval | `15` (seconds) | `environments/{prod,staging}/values.yaml` → `autoscaling.keda.pollingInterval` |
| Cooldown period | `60` (seconds) | `environments/{prod,staging}/values.yaml` → `autoscaling.keda.cooldownPeriod` |
| Queue depth threshold | `5` | `vllm:num_requests_waiting > 5` triggers scale-out |
| Cache usage threshold | `0.85` | `vllm:gpu_cache_usage_perc > 0.85` triggers scale-out |

**No external credentials needed** — KEDA connects to the in-cluster Prometheus via HTTP.

---

<a id="12-nvidia"></a>
## 12. NVIDIA GPU Operator — DCGM Exporter

| Parameter | Value | Where Configured |
|---|---|---|
| Helm repo | `https://nvidia.github.io/gpu-operator` | `addons/nvidia-gpu-operator/Chart.yaml` |
| Chart version | `v24.9.0` | `addons/nvidia-gpu-operator/Chart.yaml` |
| DCGM metrics port | `9400` | Auto-configured by GPU Operator |
| DCGM metrics path | `/metrics` | Auto-configured, scraped via DCGM's own ServiceMonitor |
| Node label | `nvidia.com/gpu.present=true` | Must be applied to GPU nodes (auto-labeled by GPU Operator with NFD) |

**No external credentials needed** — the GPU Operator runs inside the cluster and manages NVIDIA drivers/toolkit/DCGM automatically.

**Node labeling**: The GPU Operator's Node Feature Discovery (NFD) automatically labels GPU nodes with `nvidia.com/gpu.present=true`. This label is used by:
- `model-serving-engine` nodeSelector
- `swapoff` DaemonSet nodeSelector
- `lmcache` DaemonSet nodeSelector

---

<a id="13-longhorn"></a>
## 13. Longhorn — Distributed Storage

| Parameter | Value | Where Configured |
|---|---|---|
| Helm repo | `https://charts.longhorn.io` | `addons/longhorn/Chart.yaml` |
| Chart version | `1.7.2` | `addons/longhorn/Chart.yaml` |
| StorageClass (RWO) | `longhorn` | Used by `persistence.storageClass` in all envs |
| StorageClass (RWX) | `longhorn-rwx` | Used by `persistenceRWX.storageClass` in all envs |
| Longhorn UI | `longhorn-ui.example.com` | `addons/longhorn/Chart.yaml` → `longhornUI` |

**No external credentials needed** — Longhorn runs inside the cluster using local node disks.

**Manual step**: The `longhorn-rwx` StorageClass must be created manually after Longhorn is installed:
```bash
kubectl apply -f - <<EOF
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: longhorn-rwx
provisioner: driver.longhorn.io
parameters:
  dataLocality: disabled
  numberOfReplicas: "3"
  staleReplicaTimeout: "30"
  fromBackup: ""
  fsType: ext4
  accessMode: rwx
EOF
```

---

<a id="14-tests"></a>
## 14. Test Scripts — Environment Variables

### 14.1 Smoke Test (`tests/smoke/smoke-test.sh`)

| Variable | How Passed | Default | Description |
|---|---|---|---|
| `MODEL_URL` | Positional arg 1 | (required) | Model endpoint URL (e.g., `http://localhost:8000`) |
| `API_KEY` | Positional arg 2 | (empty) | Bearer token for authentication |

**Usage**:
```bash
./tests/smoke/smoke-test.sh http://localhost:8000 your-api-key
```

### 14.2 Load Test (`tests/load/load-test.js`)

| Variable | How Passed | Default | Description |
|---|---|---|---|
| `MODEL_URL` | `__ENV.MODEL_URL` | `http://localhost:8000` | Model endpoint URL |
| `API_KEY` | `__ENV.API_KEY` | (empty) | Bearer token for authentication |
| `MODEL_NAME` | `__ENV.MODEL_NAME` | `test` | Model name to send in request body |

**Usage**:
```bash
MODEL_URL=http://localhost:8000 API_KEY=your-key MODEL_NAME=mistral-7b k6 run tests/load/load-test.js
```

### 14.3 Chaos Test (`tests/chaos/gpu-chaos.yaml`)

Litmus chaos experiment environment variables. These are experiment configuration values (not external secrets) set in the chaos engine manifest.

| Variable | Default | Description |
|---|---|---|
| `TOTAL_CHAOS_DURATION` | `30` | Total chaos duration in seconds |
| `CHAOS_INTERVAL` | `10` | Time between chaos iterations in seconds |
| `FORCE` | `false` | Force delete pods without graceful termination |
| `NETWORK_LATENCY` | `2000` | Injected network latency in milliseconds (pod-network-latency experiment) |
| `JITTER` | `0` | Variability in latency injection (ms) |
| `NODE_LABEL` | `nvidia.com/gpu.present=true` | Target node selector for chaos injection |

**Usage**: These are set in the LitmusChaos ` ChaosEngine` spec, not as shell env vars. See `tests/chaos/gpu-chaos.yaml` for the full manifest.

---

<a id="15-runtime-env"></a>
## 15. Runtime Environment Variables (Pods)

These environment variables are injected into the vLLM container at runtime by the Helm chart.

### 15.1 ConfigMap (`charts/model-serving-engine/templates/configmap.yaml`)

| Variable | Value | Description |
|---|---|---|
| `ENGINE_TYPE` | `vllm` | Which inference engine to use |
| `MODEL_NAME` | From `model.name` | Model name (e.g., `mistral-7b`) |
| `VLLM_HOST` | `0.0.0.0` | Bind address |
| `VLLM_PORT` | `8000` | Listen port |

### 15.2 StatefulSet (`charts/model-serving-engine/templates/statefulset.yaml`)

| Variable | Condition | Description |
|---|---|---|
| `MODEL_NAME` | Always | Model name (from `model.name` value) |
| `LMCACHE_HOST` | `lmcache.enabled: true` | Pod's host IP (for LMCache daemon connection) |
| `LMCACHE_ENABLED` | `lmcache.enabled: true` | `"true"` — tells vLLM to use distributed cache |
| `VLLM_KV_CACHE_PERSIST_PATH` | `cachePersistence.enabled: true` | Path to SafeTensors cache (default `/cache/kv`) |
| `MODEL_DEST_PATH` | `seedJob.enabled: true` | Destination for model download (value: `/models/$(MODEL_NAME)`) — used by the seed Job |

### 15.3 LMCache DaemonSet (`charts/model-serving-engine/templates/lmcache-daemonset.yaml`)

| Variable | Condition | Value | Description |
|---|---|---|---|
| `LMCACHE_CONFIG` | `lmcache.enabled: true` | `/etc/lmcache/lmcache.toml` | Path to the mounted LMCache TOML config file |
| `MODEL_NAME` | `lmcache.enabled: true` | From `model.name` value | Model name (passed to LMCache daemon for cache key namespacing) |

### 15.4 LMCache ConfigMap (`charts/model-serving-engine/templates/lmcache-configmap.yaml`)

These are TOML config values (not env vars), but documented here for completeness:

| Config Key | Value | Description |
|---|---|---|
| `lmcache.chunk_size` | `256` | KV cache chunk size |
| `lmcache.local_cpu` | `true` | Enable L1 CPU cache |
| `local_cpu.num_cpus` | `4` (prod) / `2` (staging) | CPU workers for L1 |
| `local_disk.path` | `/var/lib/lmcache` | L2 NVMe path |
| `local_disk.max_size` | `200GiB` (prod) / `100GiB` (staging) | L2 max disk usage |
| `redis.host` | `redis-cache.monitoring.svc.cluster.local` | L3 Redis host (prod only) |
| `redis.port` | `6379` | L3 Redis port |
| `s3.endpoint` | (not set by default) | L3 S3 endpoint |
| `s3.bucket` | (not set by default) | L3 S3 bucket |
| `s3.region` | `us-east-1` | L3 S3 region |
| `observability.metrics_port` | `8330` | Prometheus metrics port |

---

<a id="16-helm-repos"></a>
## 16. Helm Repository URLs (6)

These are external Helm chart repositories referenced by the addon ArgoCD Applications. No credentials needed — all are public.

| Addon | Helm Repo URL | Chart | Version |
|---|---|---|---|
| NVIDIA GPU Operator | `https://nvidia.github.io/gpu-operator` | `gpu-operator` | `v24.9.0` |
| Longhorn | `https://charts.longhorn.io` | `longhorn` | `1.7.2` |
| Prometheus Stack | `https://prometheus-community.github.io/helm-charts` | `kube-prometheus-stack` | `65.5.0` |
| KEDA | `https://kedacore.github.io/charts` | `keda` | `2.16.0` |
| External Secrets | `https://charts.external-secrets.io` | `external-secrets` | `0.10.0` |
| cert-manager | `https://charts.jetstack.io` | `cert-manager` | `v1.16.0` |

**How to add manually** (for local testing):
```bash
helm repo add nvidia https://nvidia.github.io/gpu-operator
helm repo add longhorn https://charts.longhorn.io
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo add kedacore https://kedacore.github.io/charts
helm repo add external-secrets https://charts.external-secrets.io
helm repo add jetstack https://charts.jetstack.io
helm repo update
```

---

<a id="17-bootstrap-order"></a>
## 17. Bootstrap Order — Where to Place Each Secret

The sync waves determine the order in which ArgoCD applies resources. Secrets must exist before the resources that consume them.

| Sync Wave | Resource | Secrets/Creds Needed | Action Required |
|---|---|---|---|
| `-11` | ArgoCD repo credentials | GitHub PAT | Replace `<GITHUB_PAT_TOKEN>` in `apps/argocd-repo-credentials.yaml` |
| `-11` | ArgoCD notifications | Slack webhook + PagerDuty key | Replace placeholders in `apps/argocd-notifications.yaml` |
| `-10` | AppProjects | None | Auto-applied |
| `-3` | ExternalSecrets | AWS SM secrets must exist | Create 11 remote secrets in AWS SM (see §4.2) |
| `-2` | Longhorn + swapoff DaemonSet | None | Auto-applied |
| `-1` | GPU Operator, Prometheus, KEDA, cert-manager, ESO | AWS IAM role for ESO (IRSA) | Configure IAM role + service account annotation |
| `0` | Model-serving StatefulSets | `registry-credentials` Secret (via ESO) | ESO creates it at wave -3 |
| `1` | AI Gateway, ServiceMonitor, Grafana | `ai-gateway-saas-keys` Secret (via ESO), `inference-tls` (via cert-manager) | ESO creates SaaS keys at -3; cert-manager creates TLS at -1 |
| `2+` | Smoke tests, notifications | None | Auto-applied |

### One-Time Manual Setup (before first ArgoCD sync)

```bash
# 1. Create GitHub PAT (see §2.2)

# 2. Create AWS IAM role for ESO (IRSA)
#    Attach policy with secretsmanager:GetSecretValue for all 11 secrets
#    Annotate the model-serving-engine service account:
#    eks.amazonaws.com/role-arn: arn:aws:iam::<ACCOUNT_ID>:role/model-serving-eso

# 3. Create 11 remote secrets in AWS Secrets Manager (see §4.2)

# 4. Create Slack webhook (see §7.2)
# 5. Create PagerDuty integration key (see §7.1)

# 6. Replace placeholders in ArgoCD credential files
sed -i 's/<GITHUB_PAT_TOKEN>/ghp_your_real_token/' apps/argocd-repo-credentials.yaml
sed -i 's/<SLACK_WEBHOOK_URL>/https:\/\/hooks.slack.com\/services\/T000\/B000\/XXX/' apps/argocd-notifications.yaml
sed -i 's/<PAGERDUTY_INTEGRATION_KEY>/your_integration_key/' apps/argocd-notifications.yaml

# 7. Replace email in cert-manager
sed -i 's/ops@example.com/your-real-email@company.com/' addons/cert-manager/Chart.yaml

# 8. Replace Grafana admin password
sed -i 's/admin123/your-secure-password/' addons/prometheus-stack/Chart.yaml

# 9. Apply repo credentials first
kubectl apply -f apps/argocd-repo-credentials.yaml
kubectl apply -f apps/argocd-notifications.yaml

# 10. Apply AppProjects
kubectl apply -f apps/argocd-appprojects.yaml

# 11. Apply ExternalSecrets (AWS SM secrets must already exist)
kubectl apply -f apps/external-secrets.yaml

# 12. Apply AppSets (this triggers the full deployment)
kubectl apply -f apps/argocd-appset-dev.yaml
kubectl apply -f apps/argocd-appset-staging.yaml
kubectl apply -f apps/argocd-appset-prod.yaml
```

---

<a id="18-checklist"></a>
## 18. Verification Checklist

After all secrets and connections are configured, verify each integration:

| # | Check | Command | Expected Result |
|---|---|---|---|
| 1 | ArgoCD repo connection | `argocd repo list` | `rustnew/custom-ai-ops` shows `CONNECTION OK` |
| 2 | ExternalSecrets synced | `kubectl get externalsecrets -A` | All 4 show `READY=True` |
| 3 | SaaS keys Secret exists | `kubectl get secret ai-gateway-saas-keys -n envoy-gateway-system` | Secret found with 7 keys |
| 4 | Alertmanager config exists | `kubectl get secret alertmanager-config -n monitoring` | Secret found with `config.yaml` key |
| 5 | Registry pull secret exists | `kubectl get secret registry-credentials -n model-serving-prod` | Secret found, type `kubernetes.io/dockerconfigjson` |
| 6 | TLS certificate issued | `kubectl get certificate inference-tls -n envoy-gateway-system` | `READY=True` |
| 7 | Prometheus scraping vLLM | `kubectl port-forward svc/prometheus-server 9090:9090 -n monitoring` then open `/targets` | vLLM target shows `UP` |
| 8 | KEDA ScaledObject active | `kubectl get scaledobject -n model-serving-prod` | `READY=True`, `ACTIVE=True` |
| 9 | LMCache daemon running | `kubectl get ds -n model-serving-prod` | `lmcache` DaemonSet shows desired=pod count |
| 10 | GPU nodes labeled | `kubectl get nodes -l nvidia.com/gpu.present=true` | GPU nodes listed |
| 11 | Slack notification received | Trigger a sync in ArgoCD | Message appears in `#ml-ops` Slack channel |
| 12 | PagerDuty alert received | Trigger a critical Prometheus alert | Incident created in PagerDuty |
| 13 | Longhorn storage classes | `kubectl get storageclass` | `longhorn` and `longhorn-rwx` listed |
| 14 | Grafana dashboard loaded | Open Grafana → Dashboards | `model-serving` dashboard with 18 panels visible |
| 15 | Redis connectivity (prod) | `kubectl exec -it <lmcache-pod> -n model-serving-prod -- nc -zv redis-cache.monitoring 6379` | Connection successful |

---

<a id="19-http-headers"></a>
## 19. HTTP Headers — Gateway Conventions

These are HTTP header names (not env vars or secrets) used by the AI Gateway for routing and rate limiting. Clients must send them; the gateway reads them.

| Header | Purpose | Where Used | Required? |
|---|---|---|---|
| `x-api-key` | Rate limiting key — BackendTrafficPolicy limits 50 req/s per unique `x-api-key` value | `charts/ai-gateway/templates/backend-traffic-policy.yaml` | Yes (for rate limiting to work) |
| `x-sticky-session-key` | Sticky routing — HTTPRoute RequestHeaderModifier reads this to route to the same backend pod | `charts/ai-gateway/templates/httproute.yaml` | No (only for sticky routing) |
| `x-cache-affinity-key` | Cache-aware routing — Lua FNV-1a hash of request prefix, used by ConsistentHash LB to route to the pod holding the KV cache | `charts/ai-gateway/templates/cache-routing-policy.yaml` + `backend-traffic-policy.yaml` | No (injected by gateway Lua filter) |
| `x-rag-version` | Cache invalidation — bump this value when RAG corpus changes to invalidate prefix cache entries | `charts/model-serving-engine/templates/cache-invalidation-configmap.yaml` | No (only when RAG is used) |

**How clients use them**:
```bash
# Rate limiting (required for all requests)
curl -H "x-api-key: customer-123" https://inference.example.com/v1/chat/completions ...

# Sticky routing (optional, for session affinity)
curl -H "x-api-key: customer-123" -H "x-sticky-session-key: session-abc" https://...

# RAG version (optional, bump when knowledge base changes)
curl -H "x-api-key: customer-123" -H "x-rag-version: v2.1" https://...
```

---

*This document is the single source of truth for all environment variables, secrets, and external connections. Update it whenever a new external platform is added or a configuration changes.*