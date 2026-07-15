# External Tools Integration Guide

This document provides complete, step-by-step configuration instructions for every external platform connected to the Custom-Ai-Ops repository. For each tool, you will find:

1. **Prerequisites** — accounts, credentials, cluster state
2. **Configuration** — exact files to edit, commands to run
3. **Verification** — how to confirm the integration is working
4. **Troubleshooting** — common errors and fixes

| # | Platform | Role | File in Project |
|---|----------|------|-----------------|
| 1 | GitHub | Git hosting + CI/CD + Container Registry | `.github/workflows/ci.yaml`, `apps/argocd-repo-credentials.yaml` |
| 2 | ArgoCD | GitOps controller (sync waves, AppProjects) | `apps/argocd-*.yaml` |
| 3 | Helm Repositories | Addon distribution (5 repos) | `addons/*/Chart.yaml` |
| 4 | External Secrets Operator + AWS Secrets Manager | Sync external secrets → K8s Secrets | `apps/external-secrets.yaml` |
<<<<<<< Updated upstream
| 5 | SaaS LLM Fallback Providers (7) | Fallback inference when self-hosted models are unavailable | `charts/ai-gateway/values.yaml` |
| 6 | Container Registries (ghcr.io + Docker Hub) | Private image pulls | `apps/external-secrets.yaml` (registry-pull-secret) |
| 7 | PagerDuty + Slack | Alert routing and on-call notifications | `observability/alertmanager-routes.yaml`, `apps/argocd-notifications.yaml` |
| 8 | Prometheus + Grafana + Alertmanager | Observability stack (18 panels, 13 KV cache alerts) | `observability/`, `addons/prometheus-stack/` |
| 9 | Longhorn | Distributed storage (RWO + RWX) | `addons/longhorn/` |
| 10 | NVIDIA GPU Operator + DCGM | GPU driver + metrics + device plugin | `addons/nvidia-gpu-operator/` |
| 11 | cert-manager + Let's Encrypt | TLS certificates for AI Gateway | `addons/cert-manager/` |
| 12 | KEDA | Autoscaling on vLLM metrics (not CPU/RAM) | `addons/keda/`, `charts/model-serving-engine/templates/hpa.yaml` |
=======
| 5 | Container Registries (ghcr.io + Docker Hub) | Private image pulls | `apps/external-secrets.yaml` (registry-pull-secret) |
| 6 | Longhorn | Distributed storage (RWO + RWX) | `addons/longhorn/` |
| 7 | NVIDIA GPU Operator | GPU driver + device plugin | `addons/nvidia-gpu-operator/` |
| 8 | cert-manager + Let's Encrypt | TLS certificates for inference endpoint | `addons/cert-manager/` |
| 9 | KEDA | Autoscaling on vLLM metrics (not CPU/RAM) | `addons/keda/`, `charts/model-serving-engine/templates/hpa.yaml` |
>>>>>>> Stashed changes

---

## Table of Contents

1. [GitHub](#1-github)
2. [ArgoCD](#2-argocd)
3. [Helm Repositories](#3-helm-repositories)
4. [External Secrets Operator + AWS Secrets Manager](#4-external-secrets-operator--aws-secrets-manager)
5. [Container Registries](#5-container-registries)
6. [Longhorn](#6-longhorn)
7. [NVIDIA GPU Operator](#7-nvidia-gpu-operator)
8. [cert-manager + Let's Encrypt](#8-cert-manager--lets-encrypt)
9. [KEDA](#9-keda)
10. [Bootstrap Order](#10-bootstrap-order)
11. [Troubleshooting Quick Reference](#11-troubleshooting-quick-reference)

---

## 1. GitHub

**Role**: Git hosting, CI/CD (GitHub Actions), Container Registry (ghcr.io), and ArgoCD Image Updater write-back.

### 1.1 Repository

The project is hosted at:
```
HTTPS:  https://github.com/rustnew/custom-ai-ops.git
SSH:    git@github.com:rustnew/custom-ai-ops.git
```

All ArgoCD ApplicationSets reference the HTTPS URL. The SSH URL is used for local development.

### 1.2 GitHub Personal Access Token (PAT)

ArgoCD needs a PAT to read the repository. Image Updater needs write access to commit image tag updates back to Git.

#### Create a Classic PAT

1. Go to GitHub → **Settings** → **Developer settings** → **Personal access tokens** → **Tokens (classic)**
2. Click **Generate new token**
3. Set scopes:
   - `repo` (full repository access — required for Image Updater write-back)
   - `read:org` (read org membership for private repos)
4. Copy the token immediately — GitHub will not show it again

#### Create a Fine-Grained PAT (recommended for production)

1. Go to GitHub → **Settings** → **Developer settings** → **Personal access tokens** → **Fine-grained tokens**
2. Click **Generate new token**
3. Set:
   - **Repository access**: Only `rustnew/custom-ai-ops`
   - **Permissions**:
     - **Contents**: Read (or Read and Write for Image Updater)
     - **Metadata**: Read (auto-selected)
4. Copy the token

#### Store the PAT in AWS Secrets Manager

```bash
aws secretsmanager create-secret \
  --name registry/github-pat \
  --secret-string "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" \
  --region us-east-1
```

The ExternalSecret `argocd-image-updater-token` in `apps/external-secrets.yaml` pulls this into Kubernetes Secret `argocd-image-updater-git` (key `GITHUB_TOKEN`).

### 1.3 ArgoCD Repository Credential

The file `apps/argocd-repo-credentials.yaml` defines:
- **Secret** `custom-ai-ops-repo` (label `argocd.argoproj.io/secret-type: repository`) with `url`, `username`, `password` fields
- **ConfigMap** `argocd-ssh-known-hosts-cm` with real GitHub SSH host keys (ed25519, ecdsa, rsa)

#### Apply

```bash
kubectl apply -f apps/argocd-repo-credentials.yaml
```

However, the manifest has `<GITHUB_PAT_TOKEN>` as a placeholder. For production, either:

**Option A — Patch the Secret directly**:
```bash
kubectl create secret generic custom-ai-ops-repo \
  --namespace argocd \
  --from-literal=url=https://github.com/rustnew/custom-ai-ops.git \
  --from-literal=username=rustnew \
  --from-literal=password=ghp_xxxxxxxxxxxx \
  --type=Opaque \
  -l argocd.argoproj.io/secret-type=repository \
  --dry-run=client -o yaml | kubectl apply -f -
```

**Option B — Use ExternalSecret** (recommended): Add a GitHub PAT to AWS Secrets Manager (key `registry/github-pat`), then add an ExternalSecret that writes to the `custom-ai-ops-repo` secret.

### 1.4 GitHub Actions CI

The file `.github/workflows/ci.yaml` defines 4 jobs:

| Job | Trigger | Action |
|-----|---------|--------|
| `rust-tools` | Push/PR | `cargo build` + `cargo test` on all 4 tools + clippy + fmt |
| `helm-lint` | Push/PR | `helm lint` on all 4 charts + `helm template` dry-run |
| `registry-consistency` | Push/PR | Validates `models/registry.yaml` references real chart/model dirs |
| `vram-budget-validation` | Push/PR | Runs `vram-budget-calc` to block deploys that exceed VRAM |

No additional configuration is needed — GitHub Actions runs automatically on push/PR to `main`.

### 1.5 Verification

```bash
# Verify Git remote
git remote -v
# → origin  git@github.com:rustnew/custom-ai-ops.git (fetch)

# Verify ArgoCD can access the repo
argocd repo list
# → TYPE   NAME                       REPO                                              INSECURE
#   git    rustnew/custom-ai-ops      https://github.com/rustnew/custom-ai-ops.git      false

# Verify CI is running
gh run list --limit 5
```

### 1.6 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `Authentication failed` in ArgoCD | PAT expired or wrong scope | Regenerate token with `repo` scope, update Secret |
| `repository not found` | repoURL casing mismatch | All AppSets must use `https://github.com/rustnew/custom-ai-ops.git` (lowercase) |
| `Permission denied (publickey)` | SSH key not in GitHub | Add your SSH public key to GitHub → Settings → SSH keys |
| CI `cargo test` fails on cache-roi-calc | Tool not in CI workflow | Verify `.github/workflows/ci.yaml` includes cache-roi-calc build+test steps |

---

## 2. ArgoCD

**Role**: GitOps controller. Syncs Git state → cluster state via ApplicationSets, AppProjects, sync waves, and health checks.

### 2.1 Prerequisites

- ArgoCD v2.7+ installed in the `argocd` namespace
- CLI installed: `argocd` command

### 2.2 AppProjects

The file `apps/argocd-appprojects.yaml` defines two AppProjects at sync-wave `-10`:

| Project | Source Repos | Destinations | Use |
|---------|-------------|--------------|-----|
| `model-serving` | `github.com/rustnew/custom-ai-ops` | `model-serving-*` | All serving workloads |
| `infrastructure` | `custom-ai-ops` + Helm repos | `gpu-operator`, `longhorn-system`, `keda-system`, `cert-manager`, `external-secrets` | Cluster infrastructure |

#### Apply

```bash
kubectl apply -f apps/argocd-appprojects.yaml
```

### 2.3 ApplicationSets

Three AppSets deploy applications per environment:

| File | Environments | Apps |
|------|-------------|------|
| `apps/argocd-appset-dev.yaml` | dev | model-serving, secrets |
| `apps/argocd-appset-staging.yaml` | staging | model-serving, secrets |
| `apps/argocd-appset-prod.yaml` | prod | 5 addons + model-serving + secrets |

Each AppSet uses `git://` (HTTPS) source with `repoURL: https://github.com/rustnew/custom-ai-ops.git`.

#### Apply

```bash
kubectl apply -f apps/argocd-appset-dev.yaml
kubectl apply -f apps/argocd-appset-staging.yaml
kubectl apply -f apps/argocd-appset-prod.yaml
```

### 2.4 Health Checks

The file `apps/argocd-health-checks.yaml` defines custom health checks for resources like `ScaledObject` (KEDA) and `ExternalSecret` (ESO).

```bash
kubectl apply -f apps/argocd-health-checks.yaml
```

### 2.5 Verification

```bash
# List all Applications
argocd app list

# Check sync status of a specific app
argocd app get model-serving-prod

# Check sync waves are progressing
kubectl get applications -n argocd -o custom-columns=NAME:.metadata.name,PHASE:.status.operationState.phase,WAVE:.metadata.annotations.argocd\*sync-wave
```

### 2.6 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `Unknown project "model-serving"` | AppProject not applied | Run `kubectl apply -f apps/argocd-appprojects.yaml` |
| `repository has no committed changes` | repoURL casing mismatch | Use `https://github.com/rustnew/custom-ai-ops.git` (all lowercase) |
| `Resource not found` for ClusterSecretStore | AppProject missing clusterResourceWhitelist | Verify `external-secrets.io/ClusterSecretStore` is in model-serving AppProject whitelist |
| ScaledObject shows `Unknown` health | ArgoCD health check missing | Apply `apps/argocd-health-checks.yaml` |

---

## 3. Helm Repositories

**Role**: Five external Helm repositories provide cluster add-ons (GPU operator, storage, autoscaling, secrets, TLS).

### 3.1 Repository List

| Addon | Helm Repo URL | Chart | Version | Install Command |
|-------|-------------|-------|---------|-----------------|
| NVIDIA GPU Operator | `https://nvidia.github.io/gpu-operator` | `gpu-operator` | v24.9.0 | `helm repo add nvidia https://nvidia.github.io/gpu-operator` |
| Longhorn | `https://charts.longhorn.io` | `longhorn` | 1.7.2 | `helm repo add longhorn https://charts.longhorn.io` |
| KEDA | `https://kedacore.github.io/charts` | `keda` | 2.16.0 | `helm repo add kedacore https://kedacore.github.io/charts` |
| External Secrets | `https://charts.external-secrets.io` | `external-secrets` | 0.10.0 | `helm repo add external-secrets https://charts.external-secrets.io` |
| cert-manager | `https://charts.jetstack.io` | `cert-manager` | v1.16.0 | `helm repo add jetstack https://charts.jetstack.io` |

### 3.2 Manual Installation (optional — ArgoCD deploys these automatically)

```bash
# Add all repositories
helm repo add nvidia https://nvidia.github.io/gpu-operator
helm repo add longhorn https://charts.longhorn.io
helm repo add kedacore https://kedacore.github.io/charts
helm repo add external-secrets https://charts.external-secrets.io
helm repo add jetstack https://charts.jetstack.io
helm repo update

# Example: install KEDA manually
helm install keda kedacore/keda \
  --namespace keda-system \
  --create-namespace \
  --set watchNamespace=model-serving-prod,model-serving-staging
```

### 3.3 ArgoCD Auto-Deployment

Each addon has a `Chart.yaml` defining an ArgoCD `Application` resource. The prod AppSet includes all 5 addons:

```yaml
# apps/argocd-appset-prod.yaml — infrastructure AppSet
list:
  - path: addons/nvidia-gpu-operator      # sync-wave: -1
  - path: addons/longhorn                  # sync-wave: -2
  - path: addons/keda                      # sync-wave: -1
  - path: addons/external-secrets          # sync-wave: -1
  - path: addons/cert-manager              # sync-wave: -1
```

### 3.4 Verification

```bash
# Verify Helm repos are registered
helm repo list

# Verify addon Apps are synced in ArgoCD
argocd app list | grep -E "nvidia|longhorn|keda|external-secrets|cert-manager"

# Verify addons are running
kubectl get pods -n gpu-operator
kubectl get pods -n longhorn-system
kubectl get pods -n keda-system
kubectl get pods -n external-secrets
kubectl get pods -n cert-manager
```

### 3.5 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `CRD already exists` | Manual Helm install conflicts with ArgoCD | Use `ServerSideApply=true` (already set in syncOptions) or `helm uninstall` the manual one |
| `failed to download chart` | Network/firewall blocking Helm repo | Add egress firewall rule to allow HTTP/HTTPS to the Helm repo URLs |

---

## 4. External Secrets Operator + AWS Secrets Manager

**Role**: Pulls secrets from AWS Secrets Manager and creates Kubernetes Secrets automatically. Used for registry and CI credentials.

### 4.1 Prerequisites

1. **External Secrets Operator** installed in the cluster:
   ```bash
   helm install external-secrets external-secrets/external-secrets \
     --namespace external-secrets \
     --create-namespace \
     --set installCRDs=true
   ```

2. **AWS IAM Role for Service Account (IRSA)**: The ESO uses JWT-based authentication via a Kubernetes ServiceAccount. You need:
   - An IAM OIDC provider for your EKS cluster
   - An IAM role with a policy allowing `secretsmanager:GetSecretValue` on the secrets you create

3. **ServiceAccount** with IRSA annotation: The chart's ServiceAccount must have the IAM role ARN annotation:
   ```yaml
   serviceAccount:
     create: true
     annotations:
       eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/model-serving-secrets
   ```

### 4.2 Create the IAM Role and Policy

```bash
# Create IAM policy
aws iam create-policy \
  --policy-name custom-ai-ops-secrets-reader \
  --policy-document '{
    "Version": "2012-10-17",
    "Statement": [
      {
        "Effect": "Allow",
        "Action": ["secretsmanager:GetSecretValue", "secretsmanager:DescribeSecret"],
        "Resource": "arn:aws:secretsmanager:us-east-1:123456789012:secret:registry/*"
      }
    ]
  }'

# Create IAM role for IRSA (requires oidc-provider URL)
OIDC_URL=$(aws eks describe-cluster --name my-cluster --query "cluster.identity.oidc.issuer" --output text)
OIDC_ID=$(echo $OIDC_URL | cut -d/ -f5)

aws iam create-role \
  --role-name model-serving-secrets \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [
      {
        "Effect": "Allow",
        "Principal": {"Federated": "arn:aws:iam::123456789012:oidc-provider/'$OIDC_URL'"},
        "Action": "sts:AssumeRoleWithWebIdentity",
        "Condition": {
          "StringEquals": {
            "'$OIDC_URL':sub": "system:serviceaccount:model-serving-prod:model-serving-engine"
          }
        }
      }
    ]
  }'

# Attach policy
aws iam attach-role-policy \
  --role-name model-serving-secrets \
  --policy-arn arn:aws:iam::123456789012:policy/custom-ai-ops-secrets-reader
```

Annotate the ServiceAccount in your Helm values:
```yaml
serviceAccount:
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/model-serving-secrets
```

### 4.3 Create Secrets in AWS Secrets Manager

```bash
# Registry credentials
aws secretsmanager create-secret --name registry/github-pat \
  --secret-string "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" --region us-east-1
aws secretsmanager create-secret --name registry/docker-username \
  --secret-string "your-dockerhub-username" --region us-east-1
aws secretsmanager create-secret --name registry/docker-password \
  --secret-string "your-dockerhub-password-or-token" --region us-east-1
```

### 4.4 Apply the Manifests

```bash
kubectl apply -f apps/external-secrets.yaml
```

The manifest defines:

| Resource | Name | Namespace | Target Secret | Purpose |
|----------|------|-----------|----------------|---------|
| ClusterSecretStore | `aws-secrets-manager` | (cluster-scoped) | — | Points ESO to AWS Secrets Manager |
| ExternalSecret | `argocd-image-updater-token` | `argocd` | `argocd-image-updater-git` | GitHub PAT for Image Updater |
| ExternalSecret | `registry-pull-secret` | `model-serving-prod` | `registry-credentials` | Docker registry pull secret (ghcr.io + docker.io) |

### 4.5 Alternative Backends (Vault / GCP / Azure)

The `apps/external-secrets.yaml` file has commented sections for HashiCorp Vault, Google Secret Manager, and Azure Key Vault. Uncomment and modify the `SecretStore`/`ClusterSecretStore` provider block:

**HashiCorp Vault**:
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ClusterSecretStore
metadata:
  name: vault-backend
spec:
  provider:
    vault:
      server: "https://vault.vault.svc.cluster.local:8200"
      path: "kv"
      version: "v2"
      auth:
        kubernetes:
          mountPath: "kubernetes"
          role: "model-serving"
          serviceAccountRef:
            name: model-serving-engine
            namespace: model-serving-prod
```

**Google Secret Manager**:
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ClusterSecretStore
metadata:
  name: gcp-secret-manager
spec:
  provider:
    gcpsm:
      projectID: my-gcp-project
      auth:
        workloadIdentity:
          serviceAccountRef:
            name: model-serving-engine
            namespace: model-serving-prod
```

**Azure Key Vault**:
```yaml
apiVersion: external-secrets.io/v1beta1
kind: ClusterSecretStore
metadata:
  name: azure-key-vault
spec:
  provider:
    azurekv:
      vaultUrl: "https://my-vault.vault.azure.net"
      authType: WorkloadIdentity
      serviceAccountRef:
        name: model-serving-engine
        namespace: model-serving-prod
```

### 4.6 Verification

```bash
# Verify ClusterSecretStore is ready
kubectl get clustersecretstore

# Verify ExternalSecrets are synced
kubectl get externalsecrets -A
# → NAME                        STORE               STATUS    READY
#   argocd-image-updater-token  aws-secrets-manager Secret    True
#   registry-pull-secret        aws-secrets-manager Secret    True

# Verify Kubernetes Secrets are created
kubectl get secret argocd-image-updater-git -n argocd
kubectl get secret registry-credentials -n model-serving-prod

# Check ESO controller logs
kubectl logs -n external-secrets deploy/external-secrets-controller
```

### 4.7 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `ExternalSecret status: SecretSyncedError` | IAM role cannot read secret | Verify IAM policy allows `secretsmanager:GetSecretValue` on the secret ARN |
| `SecretStore status: InvalidProviderConfig` | Wrong region or missing credentials | Verify `region: us-east-1` matches where secrets were created |
| `Secret not found in store` | Secret key path mismatch | Verify AWS secret name matches `remoteRef.key` exactly (e.g., `saas/openai-api-key`) |
| ServiceAccount not annotated | IRSA not working | Verify `eks.amazonaws.com/role-arn` annotation on the ServiceAccount |
| `ClusterSecretStore scope error` | ExternalSecret in different namespace than SecretStore | Use `ClusterSecretStore` (not `SecretStore`) — already fixed in the manifest |

---

## 5. Container Registries

**Role**: Pull private images from GHCR (GitHub Container Registry) and Docker Hub. Used for vLLM and LMCache images.

### 5.1 Prerequisites

- GitHub account with access to `ghcr.io` packages
- Docker Hub account (for public images that may hit rate limits)

### 5.2 Registry Credentials

Store the credentials in AWS Secrets Manager:

```bash
# GitHub PAT (used for ghcr.io)
aws secretsmanager create-secret --name registry/github-pat \
  --secret-string "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" --region us-east-1

# Docker Hub credentials
aws secretsmanager create-secret --name registry/docker-username \
  --secret-string "your-dockerhub-username" --region us-east-1
aws secretsmanager create-secret --name registry/docker-password \
  --secret-string "your-dockerhub-password-or-token" --region us-east-1
```

### 5.3 Pull Secret

The ExternalSecret `registry-pull-secret` in `apps/external-secrets.yaml` creates a `kubernetes.io/dockerconfigjson` Secret in `model-serving-prod`:

```json
{
  "auths": {
    "ghcr.io": {
      "username": "rustnew",
      "password": "{{ .github_pat }}"
    },
    "docker.io": {
      "username": "{{ .docker_username }}",
      "password": "{{ .docker_password }}"
    }
  }
}
```

### 5.4 Reference in Helm Values

In `environments/prod/values.yaml` or `charts/model-serving-engine/values.yaml`:

```yaml
global:
  imageRegistry: "ghcr.io"
  imagePullSecrets:
    - name: registry-credentials
```

### 5.5 ArgoCD Image Updater

ArgoCD Image Updater can automatically update image tags in Git when new images are pushed to the registry. The Secret `argocd-image-updater-git` (ExternalSecret `argocd-image-updater-token`) provides a GitHub PAT with `contents:write` permission for write-back:

```bash
# Install Image Updater
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj-labs/argocd-image-updater/stable/manifests/install.yaml

# Verify
kubectl get pods -n argocd | grep image-updater
```

Add annotations to your Application (in the AppSet or Application manifest):

```yaml
metadata:
  annotations:
    argocd-image-updater.argoproj.io/image-list: vllm=ghcr.io/rustnew/vllm-openai
    argocd-image-updater.argoproj.io/vllm.update-strategy: latest
    argocd-image-updater.argoproj.io/write-back-method: git
```

### 5.6 Verification

```bash
# Verify pull secret exists
kubectl get secret registry-credentials -n model-serving-prod
# → NAME                  TYPE                             DATA   AGE
#   registry-credentials  kubernetes.io/dockerconfigjson  1      5m

# Test image pull
kubectl run test-pull --image=vllm/vllm-openai:v0.6.3 \
  --overrides='{"spec":{"imagePullSecrets":[{"name":"registry-credentials"}]}}' \
  -n model-serving-prod --rm -it --restart=Never -- command

# Verify Image Updater is running
kubectl logs -n argocd deploy/argocd-image-updater
```

### 5.7 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `ImagePullBackOff` | Pull secret missing or wrong credentials | Verify secret exists in the pod's namespace and `imagePullSecrets` references it |
| `403 Forbidden` from ghcr.io | PAT doesn't have `package:read` scope | Regenerate PAT with `read:packages` scope |
| Docker Hub rate limit | Anonymous pulls limited to 100/6h | Use authenticated pulls via `registry-credentials` secret |
| Image Updater not writing to Git | GITHUB_TOKEN expired or no `contents:write` scope | Verify `registry/github-pat` in AWS Secrets Manager has write permission |

---

<<<<<<< Updated upstream
## 7. PagerDuty + Slack

**Role**: Two alerting platforms — PagerDuty for critical alerts (on-call paging), Slack for team notifications (warnings, info, sync status).

### 7.1 PagerDuty Setup

#### Create a PagerDuty Service and Integration

1. Go to https://pagerduty.com → **Services** → **New Service**
2. Set name: `ML Model Serving`
3. Add an integration:
   - **Integration type**: `Events API v2` (or `Prometheus`)
   - Copy the **Integration Key** (also called `service_key`)
4. Store the key in AWS Secrets Manager:
   ```bash
   aws secretsmanager create-secret --name alerting/pagerduty-service-key \
     --secret-string "pd_integration_key_xxxxxxxxxxxxxxxx" --region us-east-1
   ```

### 7.2 Slack Setup

#### Create a Slack Incoming Webhook

1. Go to https://api.slack.com/apps → **Create New App**
2. Choose **From scratch**, name it `Custom-Ai-Ops Alerts`, select your workspace
3. Go to **Incoming Webhooks** → Enable webhooks → **Add New Webhook to Workspace**
4. Choose a channel (e.g., `#ml-ops`) and copy the webhook URL
5. Store the URL in AWS Secrets Manager:
   ```bash
   aws secretsmanager create-secret --name alerting/slack-webhook-url \
     --secret-string "https://hooks.slack.com/services/Txxx/Bxxx/xxxxxxx" --region us-east-1
   ```

### 7.3 Channel Routing

The Alertmanager config (templated via ESO into `alertmanager-config` secret) routes alerts:

| Severity / Component | Receiver | Channel |
|---------------------|----------|---------|
| `critical` | critical-receiver | PagerDuty + Slack `#ml-incidents` |
| `warning` | warning-receiver | Slack `#ml-ops` |
| `component: gpu` | gpu-receiver | Slack `#gpu-ops` |
| `component: model-serving` | serving-receiver | Slack `#ml-ops` |

### 7.4 ArgoCD Notifications (separate from Alertmanager)

The file `apps/argocd-notifications.yaml` configures ArgoCD-specific notifications (sync status, health degraded). These use a separate Slack webhook and PagerDuty integration key stored in `argocd-notifications-secret`:

```bash
kubectl create secret generic argocd-notifications-secret \
  --namespace argocd \
  --from-literal=slack-webhook-url="https://hooks.slack.com/services/Txxx/Bxxx/xxxxxxx" \
  --from-literal=pagerduty-integration-key="pd_integration_key_xxxxxxxxxxxxxxxx" \
  --type=Opaque \
  --dry-run=client -o yaml | kubectl apply -f -
```

Subscriptions:
- **slack-ops**: All apps → Slack `#ml-ops` (sync success, fail, running, health degraded)
- **pagerduty-prod**: Production apps only → PagerDuty (sync failed)

### 7.5 Verification

```bash
# Verify Alertmanager config is loaded
kubectl get secret alertmanager-config -n monitoring
kubectl port-forward svc/prometheus-alertmanager 9093:9093 -n monitoring
# → Open http://localhost:9093/#/status — config should show 5 receivers

# Test a critical alert (triggers PagerDuty + Slack #ml-incidents)
kubectl exec -n monitoring prometheus-alertmanager-0 -- \
  wget -qO- --post-data='{"alerts":[{"status":"firing","labels":{"alertname":"TestAlert","severity":"critical"}}]}' \
  http://localhost:9093/api/v2/alerts

# Test ArgoCD notifications
argocd app sync model-serving-prod  # Should trigger Slack notification
kubectl logs -n argocd deploy/argocd-notifications-controller
```

### 7.6 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `Alerts not reaching PagerDuty` | Wrong `service_key` or Events API v1 vs v2 mismatch | Verify key is from an Events API v2 integration |
| `Message not appearing in Slack` | Wrong webhook URL or channel doesn't exist | Verify webhook URL in `alerting/slack-webhook-url` and channel exists in Slack workspace |
| `PagerDuty shows "n/a" for severity` | Alert labels don't include `severity` | Verify PrometheusRule has `severity` label on all rules |
| ArgoCD notifications silent | Secret has placeholder values | Replace `<SLACK_WEBHOOK_URL>` and `<PAGERDUTY_INTEGRATION_KEY>` with real values |
| `template: panic` in Alertmanager | ESO didn't substitute `{{ .pagerduty_key }}` | Verify ExternalSecret `alertmanager-config` shows status `SecretSynced` |

---

## 8. Prometheus + Grafana + Alertmanager

**Role**: Observability stack — scrapes vLLM metrics every 10s, evaluates 13 alert rules, displays 18 Grafana panels, and sends alerts to PagerDuty/Slack.

### 8.1 Prerequisites

The `addons/prometheus-stack/Chart.yaml` deploys kube-prometheus-stack (Helm chart v65.5.0). It requires:
- ESO installed and `alertmanager-config` secret populated (see [§4](#4-external-secrets-operator--aws-secrets-manager))
- Longhorn storage class available for Prometheus PVC (50Gi) and Grafana PVC (10Gi)

### 8.2 Critical Configuration

The addon Chart.yaml embeds 3 CRITICAL settings in the Helm values:

```yaml
prometheus:
  prometheusSpec:
    # 1. Allow discovery of ServiceMonitors created by model-serving-engine
    serviceMonitorSelectorNilUsesHelmValues: false
    # 2. Allow loading PrometheusRules from observability/ directory
    ruleSelectorNilUsesHelmValues: false
    # 3. Scrape interval for vLLM metrics (5-10s recommended)
    scrapeInterval: 10s
    evaluationInterval: 10s
    retention: 30d
```

**Why these matter**:
- `serviceMonitorSelectorNilUsesHelmValues: false` → Prometheus discovers ALL ServiceMonitors (including the one created by model-serving-engine with label `release: prometheus`)
- `ruleSelectorNilUsesHelmValues: false` → Prometheus loads ALL PrometheusRules (including `observability/prometheus-anomaly-rules.yaml`)
- `scrapeInterval: 10s` → Fast enough for KV cache alerts (usage spike must be detected within 30s)

### 8.3 Apply Alert Rules

```bash
kubectl apply -f observability/prometheus-anomaly-rules.yaml
```

This creates a `PrometheusRule` with 7 alert groups:

| Group | Alerts |
|-------|--------|
| `model-serving.latency` | HighLatency (p95>2s), CriticalLatency (p99>5s) |
| `model-serving.errors` | HighErrorRate (>5%), CriticalErrorRate (>15%) |
| `model-serving.gpu` | ThermalThrottle (>85°C), UtilizationLow (<10%), MemoryNearExhaustion (>95%) |
| `model-serving.pods` | HighRestartRate, OOMKills |
| `model-serving.anomaly` | UnexpectedPodCount, HighRestartRate |
| `model-serving.kv-cache` | VLLMKVCacheUsageHigh, VLLMKVCacheUsageCritical, VLLMRequestsWaitingHigh, VLLMSwapOutBlocksDetected, NodeSwapSpaceUsageHigh, VLLMPrefixCacheHitRateLow, LMCacheL1/L2/L3HitRateLow, VLLMPrefillSkipRateLow, SSMModelPagedAttentionMisconfigured, CacheRoutingHeaderAbsent |
| `model-serving.pods` | (see above) |

### 8.4 Apply Grafana Dashboard

The dashboard is bundled in the prometheus-stack addon via Helm values. To import manually:

```bash
kubectl create configmap vllm-dashboard \
  --from-file=vllm-dashboard.json=observability/grafana-dashboards/vllm-dashboard.json \
  -n monitoring \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl label configmap vllm-dashboard \
  grafana_dashboard=1 -n monitoring
```

The dashboard has 18 panels:

| Panel | Query |
|-------|-------|
| Request Rate (req/s) | `rate(http_requests_total[5m])` |
| P95 Latency (s) | `histogram_quantile(0.95, ...)` |
| Error Rate (%) | `rate(http_requests_total{code=~"5.."}[5m])` |
| Tokens/Second | `rate(vllm:tokens_generated[5m])` |
| Active Models per Pod | `count by (pod) (vllm:active_models)` |
| OOM Kills | `increase(kube_pod_container_status_oom_killed[5m])` |
| KV Cache Usage (%) | `vllm:gpu_cache_usage_perc` |
| Prefix Cache Hit Rate (%) | `vllm:gpu_prefix_cache_hits_total / requests` |
| Request Queue Depth | `vllm:num_requests_waiting` |
| TTFT (ms) | `histogram_quantile(0.95, vllm:time_to_first_token)` |
| KV Cache Swap-Out Blocks | `vllm:swap_out_blocks` |
| GPU VRAM Usage (DCGM) | `DCGM_FI_DEV_FB_USED / DCGM_FI_DEV_FB_TOTAL` |
| LMCache L1 (CPU) Hit Rate | `lmcache_l1_hit_rate` |
| LMCache L2 (NVMe) Hit Rate | `lmcache_l2_hit_rate` |
| LMCache L3 (Redis/S3) Hit Rate | `lmcache_l3_hit_rate` |
| Prefill Skip Rate | `vllm:prefill_skip_rate` |
| Cache Affinity Routing | `rate(x_cache_affinity_key_requests[5m] by pod)` |
| Cache ROI Estimate ($/h) | `cache_hits * 11 * 22.88/3600` |

### 8.5 Apply Alertmanager Config

The Alertmanager config is templated via ESO (see [§4](#4-external-secrets-operator--aws-secrets-manager)) and loaded via:

```yaml
alertmanager:
  config:
    useExistingSecret:
      name: alertmanager-config
      key: config.yaml
```

### 8.6 Verification

```bash
# Verify Prometheus is scraping vLLM
kubectl port-forward svc/prometheus-server 9090:9090 -n monitoring
# → http://localhost:9090/targets — vllm should appear as up
# → http://localhost:9090/rules — all 7 alert groups should appear
# → http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job | contains("model-serving"))'

# Verify Grafana
kubectl port-forward svc/prometheus-grafana 3000:80 -n monitoring
# → http://localhost:3000 — login with admin / admin123
# → Dashboard "Model Serving" should show 18 panels

# Verify Alertmanager
kubectl port-forward svc/prometheus-alertmanager 9093:9093 -n monitoring
# → http://localhost:9093/#/status — config should show 5 receivers

# Verify ServiceMonitor is discovered
kubectl get servicemonitor -A | grep model-serving
```

### 8.7 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| vLLM not appearing in Prometheus targets | `serviceMonitorSelectorNilUsesHelmValues: true` (default) | Set to `false` in prometheus-stack addon values |
| Alert rules not loaded | `ruleSelectorNilUsesHelmValues: true` (default) | Set to `false` in prometheus-stack addon values |
| `vllm:gpu_cache_usage_perc` shows no data | vLLM doesn't have `--enable-prefix-caching` or metrics endpoint not exposed | Verify `serviceMonitor.enabled: true` in env values and vLLM args |
| Grafana dashboard empty | ConfigMap not labeled `grafana_dashboard=1` | Apply label or import via UI |
| Alertmanager config has `{{ .pagerduty_key }}` literal | ESO template substitution failed | Verify ExternalSecret `alertmanager-config` is `SecretSynced` |

---

## 9. Longhorn
=======
## 6. Longhorn
>>>>>>> Stashed changes

**Role**: Distributed block storage for Kubernetes. Provides two StorageClasses:
- `longhorn` (RWO) — used for model weights PVC and cache persistence PVC
- `longhorn-rwx` (RWX) — used for shared model storage across pods

### 6.1 Prerequisites

- At least 3 worker nodes with local disks (SSD recommended)
- Each node should have at least 200 GiB free disk space

### 6.2 Installation

#### ArgoCD auto-installs Longhorn at sync-wave `-2`

When deployed through the prod AppSet, Longhorn installs automatically:

```yaml
# apps/argocd-appset-prod.yaml
- path: addons/longhorn      # sync-wave: -2 (before GPU operator at -1)
```

#### Manual installation

```bash
helm repo add longhorn https://charts.longhorn.io
helm install longhorn longhorn/longhorn \
  --namespace longhorn-system \
  --create-namespace \
  --version 1.7.2
```

### 6.3 Create the RWX StorageClass

After Longhorn is installed, create the RWX storage class:

```bash
cat <<'EOF' | kubectl apply -f -
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: longhorn-rwx
provisioner: driver.longhorn.io
allowVolumeExpansion: true
reclaimPolicy: Delete
volumeBindingMode: Immediate
parameters:
  numberOfReplicas: "3"
  staleReplicaTimeout: "30"
  fromBackup: ""
  fsType: "ext4"
  dataLocality: "disabled"
  # RWX access mode requires this engine
  rebalanceThreshold: "20"
EOF
```

### 6.4 Helm Values Using Longhorn

In `environments/prod/values.yaml`:
```yaml
persistence:
  storageClass: longhorn       # RWO
  size: 50Gi

persistenceRWX:
  storageClass: longhorn-rwx   # RWX for shared model storage
  size: 50Gi

cachePersistence:
  enabled: true
  storageClass: longhorn       # RWO, for SafeTensors cache
  size: 50Gi
```

### 6.5 GPU Node Tolerations

The addon Chart.yaml adds tolerations so Longhorn pods run on GPU nodes:

```yaml
tolerations:
  - key: nvidia.com/gpu
    operator: Exists
    effect: NoSchedule
```

### 6.6 Verification

```bash
# Verify Longhorn pods are running
kubectl get pods -n longhorn-system

# Verify StorageClasses
kubectl get sc | grep longhorn
# → longhorn         driver.longhorn.io   Delete   Immediate  false    10m
# → longhorn-rwx     driver.longhorn.io   Delete   Immediate  false    5m

# Verify PVCs are bound
kubectl get pvc -A | grep longhorn
# → model-weights     Bound   pvc-xxx   longhorn       50Gi     RWO
#   model-cache       Bound   pvc-yyy   longhorn       50Gi     RWO

# Access Longhorn UI
kubectl port-forward svc/longhorn-frontend 8080:80 -n longhorn-system
# → http://localhost:8080
```

### 6.7 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `PVC stuck in Pending` | No available disks or all nodes tainted | Verify `kubectl get nodes -o wide` and Longhorn UI for disk health |
| `Multi-Attach error for volume` | Using RWO storage where RWX needed | Use `longhorn-rwx` for shared volumes |
| Longhorn pods on GPU nodes only | Not all worker nodes have disks | Add tolerations for non-GPU nodes or label storage nodes |
| PVC becoming read-only | Longhorn replica rebuilding | Check Longhorn UI → volumes, wait for rebuild to complete |

---

## 7. NVIDIA GPU Operator

**Role**: Automates installation and management of NVIDIA GPU driver, Container Toolkit, Device Plugin, and Node Feature Discovery.

### 7.1 Prerequisites

- Cluster nodes have NVIDIA GPUs (e.g., A100, H100, L4)
- Ubuntu 22.04 or RHEL 9 with kernel headers (for driver building)
- For driver pre-installed clusters: disable driver installation (`driver.enabled: false`)

### 7.2 Installation

#### ArgoCD auto-installs at sync-wave `-1`

`deployed via the prod AppSet:
```yaml
- path: addons/nvidia-gpu-operator    # sync-wave: -1
```

#### Manual installation

```bash
helm repo add nvidia https://nvidia.github.io/gpu-operator
helm install gpu-operator nvidia/gpu-operator \
  --namespace gpu-operator \
  --create-namespace \
  --version v24.9.0
```

### 7.3 Node Labelling

After GPU Operator installs, label your GPU nodes:

```bash
kubectl label nodes <gpu-node-1> <gpu-node-2> nvidia.com/gpu.present=true
```

This label is used by:
- `model-serving-engine` nodeSelector (vLLM pods)
- `swapoff` DaemonSet
- `lmcache` DaemonSet

### 7.4 Verification

```bash
# Verify GPU Operator pods
kubectl get pods -n gpu-operator
# → gpu-driver-daemonset-xxx     Running
#   gpu-operator-xxx             Running
#   nvidia-device-plugin-xxx     Running

# Verify GPUs are available to pods
kubectl run gpu-test --image=nvidia/cuda:12.2.2-base-ubuntu22.04 \
  --overrides='{"spec":{"containers":[{"name":"gpu-test","image":"nvidia/cuda:12.2.2-base-ubuntu22.04","command":["nvidia-smi"],"resources":{"limits":{"nvidia.com/gpu":1}}}],"restartPolicy":"Never"}}' \
  --rm -it
```

### 7.5 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `nvidia-smi: command not found` in pods | Driver not loaded | Verify `kubectl get pods -n gpu-operator` shows `gpu-driver-daemonset` as Running |
| `0/6 nodes are available: 6 Insufficient nvidia.com/gpu` | Device plugin not running | Verify `nvidia-device-plugin` DaemonSet is Running on all GPU nodes |
| GPU node label missing | NodeFeatureDiscovery (NFD) not labelling | Add `nvidia.com/gpu.present=true` label manually: `kubectl label node <name> nvidia.com/gpu.present=true` |
| Driver installation fails | Kernel headers missing | Install `linux-headers-$(uname -r)` on the node or switch to driver pre-installed mode |

---

## 8. cert-manager + Let's Encrypt

**Role**: Automatic TLS certificate provisioning for the inference endpoint (inference.example.com). Uses Let's Encrypt ACME with HTTP01 challenge.

### 8.1 Prerequisites

- A domain name (e.g., `inference.example.com`) with DNS pointing to your cluster's LoadBalancer
- An email address for Let's Encrypt account registration
- An ingress controller installed and configured

### 8.2 Installation

#### ArgoCD auto-installs at sync-wave `-1`

```yaml
- path: addons/cert-manager    # sync-wave: -1
```

#### Manual installation

```bash
helm repo add jetstack https://charts.jetstack.io
helm install cert-manager jetstack/cert-manager \
  --namespace cert-manager \
  --create-namespace \
  --version v1.16.0 \
  --set installCRDs=true \
  --set featureGates.additionalCertificateOutputFormats=true
```

### 8.3 ClusterIssuers

The `addons/cert-manager/Chart.yaml` defines two ClusterIssuers at the end of the file:

| Name | ACME Server | Use For |
|------|------------|---------|
| `letsencrypt-prod` | `https://acme-v02.api.letsencrypt.org/directory` | Production TLS |
| `letsencrypt-staging` | `https://acme-staging-v02.api.letsencrypt.org/directory` | Testing (rate limit friendly) |

Both use HTTP01 solver via your ingress controller.

Update the email in `addons/cert-manager/Chart.yaml`:

```yaml
spec:
  acme:
    email: ops@example.com    # ← Change to your email
```

### 8.4 Certificate for Inference Endpoint

The gateway chart creates a Certificate resource:

```bash
# After deploying the gateway
kubectl get certificate -n gateway-system
# → NAME             READY   SECRET           AGE
#   inference-tls     True    inference-tls    5m
```

### 8.5 Verification

```bash
# Verify cert-manager pods
kubectl get pods -n cert-manager
# → cert-manager-xxx            Running
#   cert-manager-cainjector-xxx Running
#   cert-manager-webhook-xxx    Running

# Verify ClusterIssuers are Ready
kubectl get clusterissuer
# → NAME                     READY   AGE
#   letsencrypt-prod          True    5m
#   letsencrypt-staging       True    5m

# Verify Certificate is issued
kubectl get certificate -A
# → inference-tls   True    inference-tls   10m

# Describe the certificate for details
kubectl describe certificate inference-tls -n gateway-system
# → Conditions: Ready=True, Message: Certificate is up to date

# Verify TLS is working
curl -vI https://inference.example.com 2>&1 | grep -E "SSL|issuer|subject"
# → subject: CN=inference.example.com
#   issuer: C=US, O=Let's Encrypt, CN=R3
```

### 8.6 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| ClusterIssuer `Ready=False` | ACME account not registered | Check cert-manager logs: `kubectl logs -n cert-manager deploy/cert-manager` |
| Certificate stuck in `Ready=False` | HTTP01 challenge can't reach gateway | Verify DNS `inference.example.com` points to your cluster's external IP |
| `failed to solve challenge: 404` | Ingress class mismatch | Verify `ingress.class` matches your ingress controller |
| Using staging issuer in prod | Wrong ClusterIssuer in Certificate | Verify Certificate references `letsencrypt-prod` not `letsencrypt-staging` |
| Rate limited by Let's Encrypt | Too many cert requests | Use staging issuer until ready, check failure logs |

---

## 9. KEDA

**Role**: Autoscaling on vLLM-specific metrics (NOT CPU/RAM). Classic HPA is inoperant for GPU-bound LLM workloads. KEDA uses two metric triggers: `vllm:num_requests_waiting` and `vllm:gpu_cache_usage_perc`.

### 9.1 Prerequisites

- Helm `kedacore` repo added (ArgoCD will do this, but verify)

### 9.2 Installation

#### ArgoCD auto-installs at sync-wave `-1`

```yaml
- path: addons/keda    # sync-wave: -1
```

KEDA is installed in the `keda-system` namespace with `watchNamespace=model-serving-prod,model-serving-staging`.

#### Manual installation

```bash
helm repo add kedacore https://kedacore.github.io/charts
helm install keda kedacore/keda \
  --namespace keda-system \
  --create-namespace \
  --version 2.16.0 \
  --set watchNamespace=model-serving-prod,model-serving-staging
```

### 9.3 ScaledObject Configuration

Set the following in your environment values:

```yaml
autoscaling:
  enabled: true
  minReplicas: 1
  maxReplicas: 4
  keda:
    enabled: true
    pollingInterval: 15         # KEDA polls the metrics source every 15s
    cooldownPeriod: 60          # Wait 60s before scaling down
    queueDepthThreshold: "5"
    cacheUsageThreshold: "0.85"
```

### 9.4 ScaledObject Rendering

Check the rendered template:

```bash
helm template prod charts/model-serving-engine \
  -f environments/prod/values.yaml \
  -s templates/hpa.yaml
```

Expected output:
```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: model-serving-engine-prod
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: model-serving-engine-prod
  minReplicaCount: 2
  maxReplicaCount: 4
  pollingInterval: 15
  cooldownPeriod: 60
  triggers:
    - type: metrics-api
      metadata:
        metricName: vllm_num_requests_waiting
        threshold: "5"
        query: vllm:num_requests_waiting
    - type: metrics-api
      metadata:
        metricName: vllm_gpu_cache_usage_perc
        threshold: "0.85"
        query: vllm:gpu_cache_usage_perc
```

### 9.5 Verification

```bash
# Verify KEDA pods
kubectl get pods -n keda-system
# → keda-operator-xxx                               Running
#   keda-operator-metrics-apiserver-xxx            Running

# Verify ScaledObject is Ready
kubectl get scaledobject -n model-serving-prod
# → NAME                           SCALETARGETNAME                        ACTIVE   READY
#   model-serving-engine-prod     StatefulSet/model-serving-engine-prod True     True

# Describe to see current replica count and trigger status
kubectl describe scaledobject model-serving-engine-prod -n model-serving-prod

# Send load to trigger scaling
# (k6 load test will trigger vllm:num_requests_waiting > 5)
k6 run tests/load/load-test.js --env MODEL_URL=https://inference.example.com
```

### 9.6 Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| ScaledObject shows `Ready=False` | KEDA not installed or wrong namespace | Verify KEDA in `keda-system` and `watchNamespace` includes `model-serving-prod` |
| No scaling observed despite high queue | Trigger threshold too high or metric absent | Lower `queueDepthThreshold` to `2` temporarily and verify the metrics source has `vllm:num_requests_waiting` data |
| Scaling back to minReplicas too fast | `cooldownPeriod` too short | Increase from 60 to 300 (5 min) for stable scale-down |
| Both CPU HPA and KEDA active | Conflict in chart values | Ensure `autoscaling.keda.enabled: true` and the legacy HPA path is not rendered (verify with `helm template -s templates/hpa.yaml`) |

---

## 10. Bootstrap Order

The complete bootstrap order, from empty cluster to fully operational model serving:

```
Phase 1: ArgoCD prerequisites (sync-wave -11 to -10)
    -11: argocd-repo-credentials         Secret + ConfigMap (GitHub PAT + SSH known_hosts)
    -10: argocd-appprojects              model-serving + infrastructure AppProjects
    -10: argocd-health-checks            Custom health checks for ScaledObject, ExternalSecret

Phase 2: Cluster infrastructure (sync-wave -3 to -1)
    -3:  external-secrets                ClusterSecretStore + ExternalSecrets
    -2:  longhorn                        Longhorn storage + RWX storage classes created
    -2:  swapoff DaemonSet               Disable swap on GPU nodes
    -1:  nvidia-gpu-operator             NVIDIA driver + device plugin + NFD
    -1:  keda                            KEDA autoscaler
    -1:  external-secrets-operator       ESO operator itself
    -1:  cert-manager                    cert-manager + Let's Encrypt ClusterIssuers

Phase 3: Model serving (sync-wave 0)
     0:  model-serving-engine            StatefulSet + Service + PDB + NetworkPolicy
     0:  model-seeding                    Model download/init Job

Phase 4: Tests + post-deploy (sync-wave 1-2+)
     1:  lmcache                          (prod/staging) Distributed cache DaemonSet
     2:  smoke/load tests                 Post-deploy validation
```

### Manual bootstrap commands (one-time setup)

```bash
# 1. Install ArgoCD
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/v2.12.0/manifests/install.yaml

# 2. Apply AppProjects + repo credentials
kubectl apply -f apps/argocd-repo-credentials.yaml
kubectl apply -f apps/argocd-appprojects.yaml
kubectl apply -f apps/argocd-health-checks.yaml

# 3. Apply ExternalSecrets (after ESO is installed by addon)
kubectl apply -f apps/external-secrets.yaml

# 4. Apply AppSets (this triggers addon deployment + model serving)
kubectl apply -f apps/argocd-appset-dev.yaml
kubectl apply -f apps/argocd-appset-staging.yaml
kubectl apply -f apps/argocd-appset-prod.yaml

# 5. Label GPU nodes
kubectl label nodes <gpu-node-1> nvidia.com/gpu.present=true
kubectl label nodes <gpu-node-2> nvidia.com/gpu.present=true

# 6. Create AWS Secrets Manager secrets (see §4.3)
# ...

# 7. Create Longhorn RWX StorageClass (see §6.3)
# ...

# 8. Wait for all apps to sync
argocd app sync model-serving-prod
argocd app wait model-serving-prod --timeout 600
```

---

## 11. Troubleshooting Quick Reference

### Cross-source issues

| Symptom | Diagnostic | Likely Cause |
|---------|-----------|--------------|
| `0/3 pods pending` | `kubectl describe pod <name>` | No GPU nodes labelled `nvidia.com/gpu.present=true` |
| `0/3 pods pending: Insufficient nvidia.com/gpu` | `kubectl get nodes -o wide` + `nvidia-smi` | Driver not loaded or device plugin not running |
| `ImagePullBackOff` | `kubectl get secret registry-credentials -n model-serving-prod` | Pull secret missing, wrong credentials |
| `CrashLoopBackOff` after Deploy | `kubectl logs <pod>` | vLLM args wrong (e.g., model path not found, no KV cache dtype supported) |
| `OOMKilled` | `kubectl describe pod <name>` | QoS Burstable (requests < limits) — verify requests == limits for ALL resources |
| High TTFT (11s) | `kubectl logs <pod>` + verify LMCache hit rates | No distributed cache (LMCache disabled) or sticky routing not working |
| KV cache > 100% | `kubectl describe pod` / vLLM logs | `--max-num-seqs` too high for model size, reduce or scale out |
| ArgoCD `Unknown project "model-serving"` | `kubectl get appproject model-serving` | AppProject not applied or wrong namespace |
| ArgoCD can't clone repo | `argocd repo list` | repoURL casing or PAT expired |
| ExternalSecrets status Failed | `kubectl get externalsecrets -A` | IAM role missing permissions, secret not in AWS, wrong region |
| KEDA ScaledObject stuck on minReplicas | `kubectl describe scaledobject <name>` | metric absent or threshold too high |

### Verification — end-to-end smoke test

```bash
# 1. All cluster addons are Ready
kubectl get pods -n gpu-operator | grep -v RUNNING | wc -l    # → 0
kubectl get pods -n longhorn-system | grep -v RUNNING | wc -l # → 0
kubectl get pods -n keda-system | grep -v RUNNING | wc -l     # → 0
kubectl get pods -n external-secrets | grep -v RUNNING | wc -l # → 0
kubectl get pods -n cert-manager | grep -v RUNNING | wc -l    # → 0

# 2. Model serving pods are Ready
kubectl get pods -n model-serving-prod | grep -v RUNNING | grep -v COMPLETED | wc -l  # → 0

# 3. End-to-end request
curl -X POST https://inference.example.com/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "x-model: mistral-7b-local" \
  -d '{"model":"mistral-7b-local","messages":[{"role":"user","content":"hello"}]}' \
  | jq '.choices[0].message.content'

# 4. KEDA scales up
kubectl get scaledobject -n model-serving-prod -w
# → Should show replicas increasing from 1 to 2+ when queue depth > 5
```

---

*Cross-references*:
- `docs/integration-report.md` — higher-level architecture context for each integration
- `docs/explain/kv-cache.md` — Master guide to KV cache management (Bible details)
<<<<<<< Updated upstream
- `docs/explain/vllm-kv-cache.md` — KV cache reference (ROI formulas, architecture gaps)
- `docs/architecture/05-observability.md` — Observability architecture detail
=======
- `docs/explain/bible-kv-cache.md` — KV cache reference (ROI formulas, architecture gaps)
>>>>>>> Stashed changes
- `docs/architecture/04-gitops-deployment.md` — GitOps deployment flow with sync waves
- `docs/runbooks/latency-spike.md` — Operational runbook for latency/failover incidents