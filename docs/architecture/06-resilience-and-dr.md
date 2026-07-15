# Resilience and Disaster Recovery

## Auto-Healing Layers

| Level | Mechanism | Tool |
|-------|-----------|------|
| Pod | Liveness/startup probe restart | Kubernetes native |
| GPU node | Xid error detection + cordon/drain | NVIDIA GPU Operator |
| Config drift | Auto-resync to Git | ArgoCD self-heal |
| Model degradation | Circuit breaker to fallback model | Application-level |
| Cluster failure | DNS-based failover to another region | External DNS |
| Data drift | Trigger + re-evaluation pipeline | Evidently AI (self-hosted) |

## Rollback

- ArgoCD auto-rollback on failed sync (configurable per ApplicationSet)
- Every automated action creates a Git trace for auditability