# Apps Directory

## Purpose

This directory contains ArgoCD ApplicationSet manifests for deploying the
LMCache + vLLM + llm-d stack across dev, staging, and prod environments.

## Files

```
apps/
├── README.md                        # This file
├── argocd-appset-dev.yaml           # Dev environment ApplicationSets
├── argocd-appset-staging.yaml       # Staging environment ApplicationSets
└── argocd-appset-prod.yaml          # Prod environment ApplicationSets
```

## What Each AppSet Deploys

### Per-Environment AppSets (dev / staging / prod)

Each `argocd-appset-{env}.yaml` deploys:

1. **model-serving-engine** — vLLM + LMCache + llm-d integration chart
   (source: `charts/model-serving-engine`, values: `environments/{env}/values.yaml`)
2. **ai-gateway** — AI Gateway with rate limiting and llm-d HTTPRoute integration
   (source: `charts/ai-gateway`, values: `environments/{env}/ai-gateway/values.yaml`)
3. **llm-d** — llm-d router (Envoy + EPP) + KV-Cache Indexer + InferencePool
   (source: `charts/llm-d`, values: `environments/{env}/llm-d/values.yaml`)

### Prod-Only: Infrastructure AppSet

The prod AppSet additionally deploys:

- **nvidia-gpu-operator** — NVIDIA GPU Operator (required for vLLM GPU support)

## Deployment Order (Sync Waves)

| Wave | Component | Purpose |
|------|-----------|---------|
| -1 | NVIDIA GPU Operator | GPU drivers + device plugin |
| 0 | model-serving-engine, llm-d | vLLM + LMCache + llm-d router |
| 1 | ai-gateway | Gateway routing to InferencePool |

## How to Deploy

### Option 1: ArgoCD (GitOps)

```bash
kubectl apply -f apps/argocd-appset-dev.yaml
kubectl apply -f apps/argocd-appset-staging.yaml
kubectl apply -f apps/argocd-appset-prod.yaml
```

### Option 2: Direct Helm Deployment

```bash
# Deploy vLLM + LMCache
helm install my-model charts/model-serving-engine \
  -f environments/prod/values.yaml \
  --set model.name=mistral-7b-instruct \
  --namespace model-serving-prod \
  --create-namespace

# Deploy llm-d router
helm install llm-d charts/llm-d \
  -f environments/prod/llm-d/values.yaml \
  --namespace llm-d-system \
  --create-namespace

# Deploy AI gateway
helm install ai-gateway charts/ai-gateway \
  -f environments/prod/ai-gateway/values.yaml \
  --namespace envoy-gateway-system \
  --create-namespace
```

## Documentation References

- **GitOps Guide**: [`docs/architecture/04-gitops-deployment.md`](../docs/architecture/04-gitops-deployment.md)
- **Environment Configs**: [`environments/`](../environments/)
- **Architecture Overview**: [`docs/architecture/00-overview.md`](../docs/architecture/00-overview.md)

## Security Note

**Never commit secrets to Git!** Always use:
- External Secrets Operator (ESO) with Vault/AWS Secrets Manager
- Sealed Secrets
- SOPS encrypted values
- Your organization's secret management solution