# Apps Directory

## Purpose

This directory is reserved for GitOps application manifests (e.g., ArgoCD ApplicationSets, FluxCD Applications, etc.).

## Current Status

**Empty** - ArgoCD integration files have been removed before GitHub publication.

## Why Was It Emptied?

The ArgoCD manifests were removed because they contained:
1. Infrastructure-specific configurations
2. Secret placeholders that could be confused with real credentials
3. Environment-specific details not suitable for a public template

For details on what was removed, see: [`REMOVED_ARGOCD.md`](../REMOVED_ARGOCD.md)

## How to Use This Project

### Option 1: Direct Helm Deployment

Deploy charts directly without GitOps:

```bash
# Deploy a model
helm install my-model charts/model-serving-engine \
  -f environments/prod/values.yaml \
  --set model.name=mistral-7b-instruct \
  --namespace model-serving-prod \
  --create-namespace
```

### Option 2: Integrate with Your GitOps Tool

#### With ArgoCD

Create your own ApplicationSet:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: ApplicationSet
metadata:
  name: model-serving
  namespace: argocd
spec:
  generators:
    - list:
        elements:
          - env: prod
            namespace: model-serving-prod
  template:
    metadata:
      name: 'model-serving-{{env}}'
    spec:
      project: default
      source:
        repoURL: https://github.com/YOUR_ORG/custom-ai-ops.git
        targetRevision: HEAD
        path: charts/model-serving-engine
        helm:
          valueFiles:
            - ../../environments/{{env}}/values.yaml
      destination:
        server: https://kubernetes.default.svc
        namespace: '{{namespace}}'
      syncPolicy:
        automated:
          prune: true
          selfHeal: true
```

See [`docs/architecture/04-gitops-deployment.md`](../docs/architecture/04-gitops-deployment.md) for complete ArgoCD integration guide.

#### With FluxCD

Create a HelmRelease:

```yaml
apiVersion: helm.toolkit.fluxcd.io/v2beta1
kind: HelmRelease
metadata:
  name: model-serving
  namespace: flux-system
spec:
  interval: 5m
  chart:
    spec:
      chart: charts/model-serving-engine
      sourceRef:
        kind: GitRepository
        name: custom-ai-ops
      interval: 1m
  values:
    # Reference your environment values
    model:
      name: mistral-7b-instruct
```

#### With Helmfile

Create a `helmfile.yaml`:

```yaml
releases:
  - name: model-serving-prod
    namespace: model-serving-prod
    chart: ./charts/model-serving-engine
    values:
      - environments/prod/values.yaml
    set:
      - name: model.name
        value: mistral-7b-instruct
```

## Documentation References

- **GitOps Guide**: [`docs/architecture/04-gitops-deployment.md`](../docs/architecture/04-gitops-deployment.md)
- **External Tools Setup**: [`docs/external-tools.md`](../docs/external-tools.md)
- **Integration Report**: [`docs/integration-report.md`](../docs/integration-report.md)
- **Environment Configs**: [`environments/`](../environments/)

## Example Structure (If You Recreate GitOps)

```
apps/
├── README.md                        # This file
├── argocd-appprojects.yaml         # (Create your own)
├── argocd-appset-{env}.yaml        # (Per environment)
├── argocd-notifications.yaml       # (Your notification config)
└── external-secrets.yaml           # (Your secrets management)
```

## Security Note

**Never commit secrets to Git!** Always use:
- External Secrets Operator (ESO) with Vault/AWS Secrets Manager
- Sealed Secrets
- SOPS encrypted values
- Your organization's secret management solution

See [`docs/external-tools.md`](../docs/external-tools.md) §5 for secret management setup.
