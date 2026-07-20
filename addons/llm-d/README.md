# llm-d Addon (Experimental)

⚠️ **EXPERIMENTAL ADDON - NOT ENABLED BY DEFAULT**

## Overview

This addon provides a reference to the official **llm-d** Helm chart for users who want to enable advanced distributed inference features:

- **Cache-aware routing** - route requests to replicas with relevant KV cache
- **Disaggregated prefill/decode** - separate compute-bound and memory-bound phases
- **SLO-aware autoscaling** - scale based on TTFT/TPOT, not just queue depth
- **Wide Expert Parallelism** - distribute MoE models across multiple nodes

## ⚠️ Important Warnings

### llm-d is CNCF Sandbox (Early-Stage)

- **Not production-stable** - expect breaking changes between releases
- **Active development** - APIs may change
- **Limited maturity** - not yet Incubating or Graduated

### Complex Operational Requirements

- Adds **7+ components** to manage (Router, EPP, KV-Cache Indexer, InferencePool CRD, Gateway API, etc.)
- Requires **dedicated MLOps team** (3+ engineers recommended)
- Requires **extensive staging validation** before production

### Network Prerequisites

- **RDMA/InfiniBand/RoCE required** for disaggregated serving and Wide EP
- On plain 1GbE/10GbE, disaggregation may be **slower** than local recompute
- Without RDMA, only cache-aware routing is beneficial

### When to Use llm-d

✅ **Use llm-d IF ALL these criteria apply**:
- ☑️ Traffic > 1M requests/day
- ☑️ Multi-tenant with strict SLAs
- ☑️ RDMA infrastructure available
- ☑️ Dedicated MLOps team (3+ engineers)
- ☑️ MoE models 70B+ requiring Wide EP
- ☑️ Cache hit rate insufficient with current stack

❌ **Don't use llm-d if**:
- Traffic < 100k requests/day
- Single-tenant or simple use case
- Plain cloud networking (no RDMA)
- Small team wanting minimal complexity
- Current stack (vLLM + LMCache + KEDA) works well

## 📚 Background

See `docs/explain/llm-d.md` for complete reference documentation (20 sections).

See `docs/adr/0004-llm-d-not-implemented.md` for decision rationale.

See `LLM_D_ANALYSIS.md` for detailed analysis.

## 🚀 Quick Start

### Prerequisites
1. **Gateway API CRDs** (v1.5.1+):

   ```bash
   kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.5.1/standard-install.yaml
   ```

2. **Gateway API Inference Extension** (GAIE v1.5.0+):

   ```bash
   kubectl apply -f https://github.com/kubernetes-sigs/gateway-api-inference-extension/releases/download/v1.5.0/manifests.yaml
   ```
3. **Secrets** (Hugging Face token):
```bash
kubectl create secret generic llm-d-hf-token \
  --from-literal=token=YOUR_HF_TOKEN \
  --namespace=model-serving-prod
```

4. **RDMA Network** (for disaggregation):
   - Verify RDMA-capable NICs: `ibv_devices` or `rdma link`
   - Configure SR-IOV or Multus for secondary network
   - Validate RDMA connectivity between nodes

### Installation

#### Option 1: Basic Router Only (No RDMA Required)

```bash
# Add llm-d Helm repo
helm repo add llm-d https://llm-d.github.io/llm-d-deployer
helm repo update

# Install router with cache-aware routing
helm install llm-d llm-d/llm-d \
  --namespace model-serving-prod \
  --create-namespace \
  --set model.name=meta-llama/Llama-3.1-8B-Instruct \
  --set router.enabled=true \
  --set disaggregation.enabled=false \
  --set autoscaling.slo.enabled=false
```

#### Option 2: Full Stack (RDMA Required)

```bash
helm install llm-d llm-d/llm-d \
  --namespace model-serving-prod \
  --create-namespace \
  --set model.name=meta-llama/Llama-3.1-8B-Instruct \
  --set router.enabled=true \
  --set indexer.enabled=true \
  --set disaggregation.enabled=true \
  --set prefill.replicas=2 \
  --set decode.replicas=4 \
  --set autoscaling.slo.enabled=true \
  --set autoscaling.slo.ttftTarget=2000ms \
  --set autoscaling.slo.tpotTarget=100ms
```

### Verification

```bash
# Check llm-d components
kubectl get pods -n model-serving-prod -l app.kubernetes.io/name=llm-d

# Check InferencePool
kubectl get inferencepool -n model-serving-prod

# Check HTTPRoute
kubectl get httproute -n model-serving-prod

# Check Gateway
kubectl get gateway -n model-serving-prod
```

## 📊 Configuration

### Incremental Adoption Path

1. **Start with Router only** (cache-aware routing)
   - No RDMA required
   - Delivers 60-80% of latency improvement
   - Simplest to operate

2. **Add KV-Cache Indexer** (precise routing)
   - Exact cache-hit routing instead of heuristic
   - Requires running indexer component

3. **Enable SLO-Aware Autoscaling**
   - Scale based on TTFT/TPOT targets
   - Better utilization than queue-depth-only

4. **Add Disaggregated Serving** (prefill/decode split)
   - **Requires RDMA network**
   - Independent scaling of prefill and decode
   - Most complex to operate

5. **Enable Wide Expert Parallelism** (MoE models)
   - **Requires RDMA + LeaderWorkerSet**
   - Only for large MoE models (70B+)
   - Highest complexity

### Example Values Files

#### dev (Router Only)

```yaml
router:
  enabled: true
  replicas: 1
  
indexer:
  enabled: false  # Heuristic routing OK for dev

disaggregation:
  enabled: false  # No RDMA in dev

autoscaling:
  slo:
    enabled: false  # Basic KEDA sufficient
```

#### staging (Router + Indexer)

```yaml
router:
  enabled: true
  replicas: 2
  
indexer:
  enabled: true  # Precise routing for staging validation

disaggregation:
  enabled: false  # Validate routing before disaggregation

autoscaling:
  slo:
    enabled: true
    ttftTarget: 3000ms
    tpotTarget: 150ms
```

#### prod (Full Stack)

```yaml
router:
  enabled: true
  replicas: 3
  
indexer:
  enabled: true
  
disaggregation:
  enabled: true
  prefill:
    replicas: 2
    resources:
      limits:
        nvidia.com/gpu: 1
        cpu: "4"
        memory: 24Gi
  decode:
    replicas: 4
    resources:
      limits:
        nvidia.com/gpu: 1
        cpu: "8"
        memory: 32Gi

autoscaling:
  slo:
    enabled: true
    ttftTarget: 2000ms
    tpotTarget: 100ms
    
wideExpertParallelism:
  enabled: false  # Only if MoE models 70B+
```

## 🔧 Integration with Current Stack

### Compatibility with model-serving-engine

llm-d **complements** the existing `model-serving-engine` chart:

**Current Stack** (default):
```
Client → K8s Service (round-robin) → vLLM StatefulSet → LMCache
```

**With llm-d** (opt-in):
```
Client → Gateway Proxy → llm-d Router (EPP) → InferencePool → vLLM StatefulSet → LMCache
                          ↓
                    KV-Cache Indexer
```

### vLLM Configuration Changes

When enabling disaggregated serving, vLLM instances need `--kv-transfer-config`:

**Prefill instance**:
```yaml
engine:
  vllm:
    args:
      # ... existing args ...
      - --kv-transfer-config
      - '{"kv_connector":"LMCacheConnectorV1","kv_role":"kv_producer",...}'
```

**Decode instance**:
```yaml
engine:
  vllm:
    args:
      # ... existing args ...
      - --kv-transfer-config
      - '{"kv_connector":"LMCacheConnectorV1","kv_role":"kv_consumer",...}'
```

See `docs/explain/llm-d.md` §11.2 for complete configuration.

### LMCache Integration

llm-d uses LMCache as its default KV-cache layer. The existing LMCache configuration works with llm-d:

- ✅ Multi-tier cache (L0→L1→L2→L3)
- ✅ NIXL-based transfer for disaggregation
- ✅ Cache event emission to KV-Cache Indexer

No LMCache reconfiguration needed for basic routing. Disaggregation requires per-instance LMCache servers (see documentation).

## 📈 Monitoring

### Key Metrics

Monitor these to validate llm-d benefit:

| Metric | Current (no llm-d) | Target (with llm-d) |
|--------|-------------------|---------------------|
| **Cache hit rate** | ~40-50% (round-robin) | > 70% (cache-aware) |
| **TTFT P95** | Variable | < 2000ms (SLO) |
| **TPOT P95** | Variable | < 100ms (SLO) |
| **GPU utilization** | ~60% | > 80% |

### Prometheus Queries

```promql
# Cache hit rate
sum(rate(vllm_cache_hits_total[5m])) / sum(rate(vllm_cache_requests_total[5m]))

# TTFT P95
histogram_quantile(0.95, sum(rate(vllm_time_to_first_token_seconds_bucket[5m])) by (le))

# TPOT P95
histogram_quantile(0.95, sum(rate(vllm_time_per_output_token_seconds_bucket[5m])) by (le))

# GPU utilization
avg(DCGM_FI_DEV_GPU_UTIL)
```

### Dashboards

Import llm-d Grafana dashboards:
```bash
kubectl apply -f https://raw.githubusercontent.com/llm-d/llm-d/main/manifests/monitoring/grafana-dashboard.yaml
```

## 🐛 Troubleshooting

### Common Issues

#### 1. HTTPRoute not found

**Symptom**: `kubectl get httproute` shows no routes

**Solution**: Verify Gateway API CRDs installed:
```bash
kubectl api-resources | grep gateway
```

#### 2. InferencePool empty

**Symptom**: `kubectl get inferencepool` shows pool but no endpoints

**Solution**: Check pod labels match selector:
```bash
kubectl get pods -n model-serving-prod --show-labels
```

#### 3. KV-Cache Indexer stale

**Symptom**: Low cache hit rate despite shared prefixes

**Solution**: Check indexer logs for event processing:
```bash
kubectl logs -n model-serving-prod -l app=llm-d-indexer -f
```

#### 4. Disaggregation slower than recompute

**Symptom**: Higher latency with disaggregation enabled

**Solution**: Verify RDMA network:
```bash
# On prefill pod
ib_send_bw <decode-pod-ip>

# Expect: > 50 Gbps for InfiniBand
```

If < 10 Gbps, **disable disaggregation** - plain networking too slow.

## 🔐 Security Considerations

### Network Policies

llm-d Router needs access to all vLLM pods:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-llm-d-router
spec:
  podSelector:
    matchLabels:
      app: vllm
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: llm-d-router
      ports:
        - protocol: TCP
          port: 8000
```

### RBAC

llm-d needs permissions to watch InferencePool and vLLM pods:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: llm-d-router
rules:
  - apiGroups: ["inference.networking.x-k8s.io"]
    resources: ["inferencepools"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["pods", "services"]
    verbs: ["get", "list", "watch"]
```

## 📚 Additional Resources

### Documentation

- **Complete Reference**: `docs/explain/llm-d.md` (20 sections)
- **Decision Rationale**: `docs/adr/0004-llm-d-not-implemented.md`
- **Analysis**: `LLM_D_ANALYSIS.md`

### External Links

- Official site: https://llm-d.ai
- GitHub: https://github.com/llm-d/llm-d
- CNCF Sandbox: https://github.com/cncf/sandbox/issues/462
- Documentation: https://llm-d.ai/docs
- Community: https://llm-d.ai/community

### Related ADRs

- ADR-0001: Multi-format architecture
- ADR-0003: Separate engine charts
- ADR-0004: llm-d not implemented (this addon is the optional path)

## ⚠️ Support and Stability

### Experimental Status

This addon is marked **EXPERIMENTAL** because:
- ❌ llm-d is CNCF Sandbox (early-stage)
- ❌ Breaking changes expected between releases
- ❌ Not validated in Custom-AI-Ops production
- ❌ Requires significant operational expertise

### Support

- **Community support only** - not officially supported by Custom-AI-Ops maintainers
- llm-d community: https://llm-d.ai/community
- GitHub issues: https://github.com/llm-d/llm-d/issues
- CNCF Slack: #llm-d channel

### Feedback

If you use this addon, please share feedback:
- What worked well?
- What was difficult?
- What could be improved?
- Would you recommend it?

File issues in Custom-AI-Ops repo with `[llm-d]` prefix.

---

**Last updated**: 15 Juillet 2026  
**Addon version**: 0.3.0  
**llm-d version**: 0.3.0  
**Status**: ⚠️ EXPERIMENTAL
