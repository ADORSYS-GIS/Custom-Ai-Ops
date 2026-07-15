# llm-d Implementation Guide

## Overview

This document describes the **llm-d** (LLM Disaggregation) implementation in Custom-AI-Ops. llm-d is a CNCF Sandbox project that provides cache-aware, intelligent routing for LLM inference workloads on Kubernetes.

**Implementation Status**: ✅ **MVP Complete (Option 1)**

**What's Included**:
- ✅ Gateway API + GAIE CRDs infrastructure
- ✅ llm-d Router (Proxy + Endpoint Picker)
- ✅ InferencePool CRDs for service discovery
- ✅ Gateway + HTTPRoute for intelligent routing
- ✅ vLLM configuration for cache-aware routing
- ✅ Environment-specific configurations (dev/staging/prod)

**What's NOT Included (Future Enhancements)**:
- ❌ KV-Cache Indexer (precise cache-hit routing)
- ❌ Disaggregated Serving (prefill/decode separation)
- ❌ SLO-aware autoscaling
- ❌ Multi-tenant flow control

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Application                        │
└─────────────────────────┬───────────────────────────────────────┘
                          │ HTTP/HTTPS
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Gateway API Gateway (Ingress)                    │
│                  (llm-d-gateway-{env})                          │
└─────────────────────────┬───────────────────────────────────────┘
                          │ HTTPRoute with ext-proc
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              llm-d Router (Proxy + EPP)                         │
│  ┌──────────────┐              ┌──────────────────────────┐    │
│  │   Proxy      │─────────────▶│  Endpoint Picker (EPP)   │    │
│  │ (HTTP proxy) │              │  (Scheduling brain)      │    │
│  └──────────────┘              └──────────┬───────────────┘    │
│                                            │                     │
│                                            ▼                     │
│                                 ┌──────────────────────┐        │
│                                 │  InferencePool CRD   │        │
│                                 │  (Service Discovery) │        │
│                                 └──────────┬───────────┘        │
└────────────────────────────────────────────┼────────────────────┘
                                              │
                          ┌───────────────────┴───────────────────┐
                          ▼                                       ▼
                  ┌───────────────┐                      ┌───────────────┐
                  │  vLLM Pod 1   │                      │  vLLM Pod 2   │
                  │  + LMCache    │                      │  + LMCache    │
                  └───────────────┘                      └───────────────┘
```

## Components

### 1. llm-d-infrastructure Chart

**Location**: `charts/llm-d-infrastructure/`

**Purpose**: Installs Gateway API and GAIE (Gateway API Inference Extension) CRDs.

**What it installs**:
- Gateway API v1.2.1 CRDs (Gateway, HTTPRoute, GatewayClass, etc.)
- GAIE v0.3.0 CRDs (InferencePool, InferenceObjective, InferenceModelRewrite)
- RBAC for CRD installation job

**Installation**:
```bash
# Install once per cluster (cluster-scoped)
helm install llm-d-infrastructure ./charts/llm-d-infrastructure \
  --namespace llm-d-system \
  --create-namespace
```

**Verification**:
```bash
# Check CRDs are installed
kubectl get crd | grep gateway.networking.k8s.io
kubectl get crd | grep inference.networking.x-k8s.io

# Check job completed successfully
kubectl get job -n llm-d-system llm-d-infrastructure-crds
```

### 2. llm-d-router Chart

**Location**: `charts/llm-d-router/`

**Purpose**: Deploys the llm-d Router components (Proxy + Endpoint Picker).

**Components**:
- **Proxy**: HTTP proxy that intercepts inference requests
- **Endpoint Picker (EPP)**: Selects optimal backend based on scoring algorithms

**Key Features**:
- Cache-aware routing (heuristic mode in MVP)
- Load-balanced endpoint selection
- Latency-aware scoring
- Health checking and circuit breaking
- Prometheus metrics

**Installation** (per environment):
```bash
# Dev
helm install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-dev \
  --create-namespace \
  --values environments/dev/llm-d-router-values.yaml

# Staging
helm install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-staging \
  --create-namespace \
  --values environments/staging/llm-d-router-values.yaml

# Production
helm install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-prod \
  --create-namespace \
  --values environments/prod/llm-d-router-values.yaml
```

**Configuration Files**:
- `environments/dev/llm-d-router-values.yaml` - Dev config (1 replica, simple load balancing)
- `environments/staging/llm-d-router-values.yaml` - Staging config (2 replicas, cache-aware routing)
- `environments/prod/llm-d-router-values.yaml` - Production config (3 replicas, optimized routing, HA)

### 3. InferencePool Resources

**Location**: `environments/{env}/llm-d-inferencepool.yaml`

**Purpose**: Defines logical groups of vLLM pods serving the same model.

**Key Configuration**:
```yaml
apiVersion: inference.networking.x-k8s.io/v1alpha2
kind: InferencePool
metadata:
  name: vllm-{env}-pool
spec:
  targetPortNumber: 8000
  selector:
    matchLabels:
      app.kubernetes.io/name: model-serving-engine
      app.kubernetes.io/instance: vllm-{env}
  endpointPickerConfig:
    extensionRef:
      name: llm-d-epp
    config:
      cacheAwareRouting: true  # false in dev
      scoringAlgorithms: [...]
```

**Deployment**:
```bash
# Apply per environment
kubectl apply -f environments/dev/llm-d-inferencepool.yaml
kubectl apply -f environments/staging/llm-d-inferencepool.yaml
kubectl apply -f environments/prod/llm-d-inferencepool.yaml
```

### 4. Gateway + HTTPRoute Resources

**Location**: `environments/{env}/llm-d-gateway.yaml`

**Purpose**: Defines Gateway API resources for routing inference requests.

**Key Components**:
- **Gateway**: Entry point for HTTP/HTTPS traffic
- **HTTPRoute**: Routes requests to InferencePool via EPP
- **InferenceObjective**: SLO targets (staging/prod only)
- **NetworkPolicy**: Network isolation (prod only)

**Deployment**:
```bash
# Apply per environment
kubectl apply -f environments/dev/llm-d-gateway.yaml
kubectl apply -f environments/staging/llm-d-gateway.yaml
kubectl apply -f environments/prod/llm-d-gateway.yaml
```

**Routes Configured**:
- `/v1/chat/completions` → Cache-aware routing via EPP
- `/v1/completions` → Cache-aware routing via EPP
- `/v1/embeddings` → Cache-aware routing via EPP (prod only)
- `/health` → Direct passthrough (no EPP)
- `/metrics` → Direct passthrough (no EPP)
- `/v1/models` → Direct passthrough (no EPP)

### 5. vLLM Configuration Changes

**Modified Files**:
- `environments/staging/values.yaml`
- `environments/prod/values.yaml`

**Changes Made**:
```yaml
engine:
  vllm:
    args:
      # ... existing args ...
      # NEW: Enable chunked prefill for better batching
      - --enable-chunked-prefill
      - --max-num-batched-tokens
      - "8192"  # staging
      - "16384"  # prod
      
      # FUTURE: Uncomment when KV-Cache Indexer is deployed
      # - --kv-cache-event-endpoint
      # - "http://llm-d-kv-cache-indexer.llm-d-system.svc.cluster.local:8080/events"
```

**Why These Changes**:
- `--enable-chunked-prefill`: Allows vLLM to process prefill in chunks, improving batching efficiency
- `--max-num-batched-tokens`: Sets maximum tokens per batch, optimizing throughput

## Deployment Guide

### Prerequisites

1. **Kubernetes Cluster**: v1.24+
2. **Helm**: v3.8+
3. **kubectl**: v1.24+
4. **Existing vLLM deployment**: Custom-AI-Ops model-serving-engine chart deployed

### Step-by-Step Deployment

#### Step 1: Install llm-d Infrastructure (One-Time, Cluster-Wide)

```bash
# Create namespace
kubectl create namespace llm-d-system

# Install CRDs
helm install llm-d-infrastructure ./charts/llm-d-infrastructure \
  --namespace llm-d-system

# Wait for job to complete
kubectl wait --for=condition=complete --timeout=300s \
  job/llm-d-infrastructure-crds -n llm-d-system

# Verify CRDs
kubectl get crd | grep -E "(gateway|inference)"
```

Expected output:
```
gatewayclasses.gateway.networking.k8s.io
gateways.gateway.networking.k8s.io
httproutes.gateway.networking.k8s.io
inferencemodelrewrites.inference.networking.x-k8s.io
inferenceobjectives.inference.networking.x-k8s.io
inferencepools.inference.networking.x-k8s.io
```

#### Step 2: Deploy llm-d Router (Per Environment)

**For Development**:
```bash
helm install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-dev \
  --create-namespace \
  --values environments/dev/llm-d-router-values.yaml

# Verify deployment
kubectl get pods -n model-serving-dev -l app.kubernetes.io/name=llm-d-router
```

**For Staging**:
```bash
helm upgrade --install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-staging \
  --create-namespace \
  --values environments/staging/llm-d-router-values.yaml

kubectl get pods -n model-serving-staging -l app.kubernetes.io/name=llm-d-router
```

**For Production**:
```bash
helm upgrade --install llm-d-router ./charts/llm-d-router \
  --namespace model-serving-prod \
  --create-namespace \
  --values environments/prod/llm-d-router-values.yaml

kubectl get pods -n model-serving-prod -l app.kubernetes.io/name=llm-d-router
```

#### Step 3: Deploy InferencePool Resources

```bash
# Dev
kubectl apply -f environments/dev/llm-d-inferencepool.yaml

# Staging
kubectl apply -f environments/staging/llm-d-inferencepool.yaml

# Production
kubectl apply -f environments/prod/llm-d-inferencepool.yaml

# Verify InferencePools
kubectl get inferencepools -A
```

Expected output:
```
NAMESPACE                NAME                 AGE
model-serving-dev        vllm-dev-pool        10s
model-serving-staging    vllm-staging-pool    10s
model-serving-prod       vllm-prod-pool       10s
```

#### Step 4: Deploy Gateway + HTTPRoute

```bash
# Dev
kubectl apply -f environments/dev/llm-d-gateway.yaml

# Staging
kubectl apply -f environments/staging/llm-d-gateway.yaml

# Production
kubectl apply -f environments/prod/llm-d-gateway.yaml

# Verify Gateways
kubectl get gateways -A

# Verify HTTPRoutes
kubectl get httproutes -A
```

#### Step 5: Update vLLM Deployments

```bash
# Staging: Upgrade vLLM with new args
helm upgrade vllm-staging ./charts/model-serving-engine \
  --namespace model-serving-staging \
  --values environments/staging/values.yaml

# Production: Upgrade vLLM with new args
helm upgrade vllm-prod ./charts/model-serving-engine \
  --namespace model-serving-prod \
  --values environments/prod/values.yaml

# Wait for rollout
kubectl rollout status statefulset/vllm-staging -n model-serving-staging
kubectl rollout status statefulset/vllm-prod -n model-serving-prod
```

#### Step 6: Verification

**Check all components are running**:
```bash
# Infrastructure
kubectl get pods -n llm-d-system

# Router components (per env)
kubectl get pods -n model-serving-{dev,staging,prod} -l app.kubernetes.io/name=llm-d-router

# InferencePools
kubectl get inferencepools -A

# Gateways
kubectl get gateways -A

# vLLM pods
kubectl get pods -n model-serving-{dev,staging,prod} -l app.kubernetes.io/name=model-serving-engine
```

**Test inference request**:
```bash
# Get Gateway service endpoint
GATEWAY_IP=$(kubectl get svc -n model-serving-prod llm-d-gateway-prod -o jsonpath='{.status.loadBalancer.ingress[0].ip}')

# Send test request
curl -X POST http://${GATEWAY_IP}:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'
```

**Check metrics**:
```bash
# Proxy metrics
kubectl port-forward -n model-serving-prod svc/llm-d-router-proxy 9090:9090
curl http://localhost:9090/metrics | grep llm_d_proxy

# EPP metrics
kubectl port-forward -n model-serving-prod svc/llm-d-router-epp 9091:9091
curl http://localhost:9091/metrics | grep llm_d_epp
```

## Configuration Reference

### Environment-Specific Differences

| Feature | Dev | Staging | Production |
|---------|-----|---------|------------|
| **Router Replicas** | Proxy: 1, EPP: 1 | Proxy: 2, EPP: 2 | Proxy: 3, EPP: 3 |
| **Cache-Aware Routing** | ❌ Disabled | ✅ Enabled (heuristic) | ✅ Enabled (heuristic) |
| **Scoring Algorithms** | Load-balancing only | Prefix-match + Load + Latency | Prefix-match + Load + Latency (optimized) |
| **Selection Strategy** | Round-robin | Weighted | Weighted (top-2) |
| **Circuit Breaker** | ❌ Disabled | ✅ Enabled (3 failures) | ✅ Enabled (5 failures) |
| **HPA** | ❌ Disabled | ❌ Disabled | ✅ Enabled (3-10 replicas) |
| **PDB** | ❌ Disabled | ✅ Enabled (min 1) | ✅ Enabled (min 2) |
| **HTTPS/TLS** | ❌ HTTP only | ❌ HTTP only | ✅ HTTP + HTTPS |
| **NetworkPolicy** | ❌ Disabled | ❌ Disabled | ✅ Enabled |
| **Monitoring Alerts** | ❌ Disabled | ❌ Disabled | ✅ Enabled |

### Scoring Algorithm Configuration

**Prefix-Match Scorer** (Cache-Aware Routing):
- **Weight**: 0.6 (staging), 0.7 (prod)
- **Mode**: Heuristic (estimates cache hits based on recent request patterns)
- **Prefix Length**: 512 tokens (staging), 1024 tokens (prod)

**Load-Balancing Scorer**:
- **Weight**: 0.3 (staging), 0.2 (prod)
- **Queue Depth Weight**: 0.7 (staging), 0.6 (prod)
- **GPU Utilization Weight**: 0.3 (staging), 0.4 (prod)

**Latency Scorer**:
- **Weight**: 0.1
- **Window**: 60s (staging), 120s (prod)
- **Target P99**: Not set (staging), 500ms (prod)

### Filter Configuration

**Capacity Check**:
- Max Queue Depth: 20 (staging), 30 (prod)
- Max GPU Utilization: 95%
- Min Available Memory: 1GB (prod only)

**Health Check**:
- Enabled in all environments
- Consecutive Successes: 1 (staging), 2 (prod)

**Model Compatibility** (prod only):
- Enforce exact model name match

**Version Check** (prod only):
- Allowed vLLM versions: v0.6.0, v0.6.1, v0.6.2, v0.6.3

## Monitoring and Observability

### Prometheus Metrics

**Proxy Metrics** (port 9090):
```
# Request metrics
llm_d_proxy_requests_total{method, status}
llm_d_proxy_request_duration_seconds{method, status}

# Routing metrics
llm_d_proxy_routing_decisions_total{decision}
llm_d_proxy_backend_selection_duration_seconds

# Error metrics
llm_d_proxy_errors_total{type}
```

**EPP Metrics** (port 9091):
```
# Scoring metrics
llm_d_epp_scoring_duration_seconds{algorithm}
llm_d_epp_backend_scores{backend, algorithm}

# Cache metrics
llm_d_epp_cache_hit_rate{backend}
llm_d_epp_cache_hit_total{backend}
llm_d_epp_cache_miss_total{backend}

# Discovery metrics
llm_d_epp_backends_discovered{pool}
llm_d_epp_backends_healthy{pool}
```

### Grafana Dashboards

**Production Dashboard** (included):
- Request rate, latency, error rate
- Backend selection distribution
- Cache hit rate per backend
- EPP scoring duration
- Circuit breaker state

**Import Dashboard**:
```bash
kubectl get configmap -n model-serving-prod llm-d-router-dashboard -o yaml
```

### Alerting Rules

**Production Alerts** (Prometheus rules):

1. **LLMDRouterHighLatency**: P99 > 1s for 5 minutes
2. **LLMDEPPHighScoringDuration**: P99 scoring > 100ms for 5 minutes
3. **LLMDCacheHitRateLow**: Cache hit rate < 50% for 10 minutes

**Alert Configuration**:
```yaml
# See: environments/prod/llm-d-router-values.yaml
monitoring:
  prometheusRule:
    enabled: true
```

## Troubleshooting

### Common Issues

#### 1. InferencePool Shows No Endpoints

**Symptom**:
```bash
kubectl get inferencepool vllm-prod-pool -n model-serving-prod
# Status: 0 endpoints ready
```

**Root Causes**:
- vLLM pods not labeled correctly
- vLLM pods not ready (health checks failing)
- EPP cannot discover pods (RBAC issue)

**Fix**:
```bash
# Check vLLM pod labels
kubectl get pods -n model-serving-prod -l app.kubernetes.io/name=model-serving-engine --show-labels

# Check EPP logs
kubectl logs -n model-serving-prod -l app.kubernetes.io/component=epp

# Check RBAC
kubectl auth can-i list pods --as=system:serviceaccount:model-serving-prod:llm-d-router -n model-serving-prod
```

#### 2. Proxy Cannot Reach EPP

**Symptom**:
```bash
kubectl logs -n model-serving-prod -l app.kubernetes.io/component=proxy
# Error: connection refused to llm-d-router-epp:8081
```

**Fix**:
```bash
# Check EPP service
kubectl get svc -n model-serving-prod llm-d-router-epp

# Check EPP pods are running
kubectl get pods -n model-serving-prod -l app.kubernetes.io/component=epp

# Test connectivity from proxy pod
kubectl exec -n model-serving-prod -it <proxy-pod> -- curl http://llm-d-router-epp:8081/health
```

#### 3. Gateway Not Routing to vLLM

**Symptom**:
Requests to Gateway return 503 Service Unavailable

**Fix**:
```bash
# Check Gateway status
kubectl get gateway -n model-serving-prod llm-d-gateway-prod -o yaml

# Check HTTPRoute status
kubectl get httproute -n model-serving-prod llm-d-inference-route-prod -o yaml

# Check InferencePool has endpoints
kubectl describe inferencepool -n model-serving-prod vllm-prod-pool
```

#### 4. Low Cache Hit Rate

**Symptom**:
Metrics show cache hit rate < 30%

**Root Causes**:
- Requests have low prefix overlap
- LMCache not configured correctly
- vLLM prefix caching disabled

**Fix**:
```bash
# Check vLLM args include --enable-prefix-caching
kubectl get statefulset -n model-serving-prod vllm-prod -o yaml | grep enable-prefix-caching

# Check LMCache is enabled
kubectl get configmap -n model-serving-prod vllm-prod-lmcache -o yaml

# Check EPP scorer weights
kubectl get configmap -n model-serving-prod llm-d-router-epp -o yaml | grep -A5 scorers
```

#### 5. High EPP Scoring Duration

**Symptom**:
EPP P99 scoring duration > 200ms

**Root Causes**:
- Too many backends to score
- CPU limits too low
- Scoring algorithms too complex

**Fix**:
```bash
# Increase EPP CPU limits
helm upgrade llm-d-router ./charts/llm-d-router \
  --set epp.resources.limits.cpu=2000m \
  --reuse-values

# Reduce scoring interval
helm upgrade llm-d-router ./charts/llm-d-router \
  --set epp.env.SCORING_INTERVAL_MS=200 \
  --reuse-values
```

## Performance Tuning

### Expected Performance Improvements

Based on llm-d project benchmarks:

| Metric | Without llm-d | With llm-d (MVP) | Improvement |
|--------|---------------|------------------|-------------|
| **TTFT P50** | 300ms | 180-220ms | **25-40% faster** |
| **TTFT P99** | 800ms | 450-600ms | **25-44% faster** |
| **Throughput** | 1500 tok/s | 2000-2500 tok/s | **33-67% higher** |
| **Cache Hit Rate** | 0% (no cache awareness) | 40-60% | **New capability** |

*Note: Actual improvements depend on workload characteristics (prefix overlap, request patterns).*

### Tuning Recommendations

**For High Throughput Workloads**:
- Increase `max-num-batched-tokens` to 32768
- Increase EPP replicas to 5+
- Set `epp.config.selector.strategy: top-n` with `topN: 5`

**For Low Latency Workloads**:
- Increase prefix-match scorer weight to 0.8
- Decrease `epp.env.SCORING_INTERVAL_MS` to 50ms
- Enable more aggressive circuit breaking

**For Diverse Workloads**:
- Balance scorer weights equally (0.33 each)
- Use `selector.strategy: weighted` with `topN: 3`
- Enable all filters

## Future Enhancements

### Planned (Not in MVP)

1. **KV-Cache Indexer** (Precise Cache-Hit Routing)
   - Chart: `charts/llm-d-kv-cache-indexer/`
   - Enable with: `epp.config.scorers[prefix-match].config.mode: precise`
   - Expected improvement: +15-20% cache hit rate

2. **Disaggregated Serving** (Prefill/Decode Separation)
   - Separate StatefulSets for prefill and decode pods
   - Requires RDMA/NIXL for KV-cache transfer
   - Expected improvement: +30-50% throughput

3. **SLO-Aware Autoscaling**
   - KEDA ScaledObject with custom llm-d metrics
   - Scale based on queue depth + cache pressure
   - Expected improvement: Better resource utilization

4. **Multi-Tenant Flow Control**
   - Priority-based request scheduling
   - Per-tenant rate limiting
   - InferenceObjective per tenant

### Enabling Future Features

**When KV-Cache Indexer is deployed**:
```yaml
# Uncomment in environments/{env}/values.yaml
engine:
  vllm:
    args:
      - --kv-cache-event-endpoint
      - "http://llm-d-kv-cache-indexer.llm-d-system.svc.cluster.local:8080/events"

# Update EPP config in environments/{env}/llm-d-router-values.yaml
epp:
  config:
    scorers:
      - name: prefix-match
        config:
          mode: precise  # Changed from heuristic
          indexerUrl: "http://llm-d-kv-cache-indexer:8080"
```

## References

- **llm-d Project**: https://github.com/kubernetes-sigs/llm-d
- **Gateway API**: https://gateway-api.sigs.k8s.io/
- **GAIE Spec**: https://github.com/kubernetes-sigs/gateway-api-inference-extension
- **vLLM Documentation**: https://docs.vllm.ai/
- **LMCache**: https://github.com/LMCache/LMCache
- **CNCF Sandbox Projects**: https://www.cncf.io/sandbox-projects/

## Support and Contributing

For issues, questions, or contributions:

- **GitHub Issues**: https://github.com/your-org/Custom-Ai-Ops/issues
- **Documentation**: `docs/explain/llm-d.md` (detailed llm-d explanation)
- **ADR**: `docs/adr/0004-llm-d-not-implemented.md` (historical context)

## Changelog

### 2026-07-15 - MVP Implementation (Option 1)

- ✅ Implemented llm-d Router (Proxy + EPP)
- ✅ Installed Gateway API + GAIE CRDs
- ✅ Created InferencePool resources for dev/staging/prod
- ✅ Configured Gateway + HTTPRoute for intelligent routing
- ✅ Updated vLLM configurations for cache-aware mode
- ✅ Created environment-specific value files
- ✅ Documented complete implementation guide

**Status**: Ready for deployment and testing
