# ADR-0004: llm-d Integration for Cache-Aware Routing and Disaggregated Serving

## Status: Accepted

## Context

The platform uses a consistent-hash heuristic at the Envoy AI Gateway layer to route requests to vLLM replicas that hold relevant KV-cache blocks (Layer 8 — Cache-aware routing in the KV Cache Management architecture). While this heuristic improves cache locality, it has limitations:

1. **Imprecise routing**: The FNV-1a hash of the first 512 bytes of the request body is a proxy for cache affinity — it does not know which replica actually holds the cache blocks.
2. **No cluster-wide cache state**: Each vLLM pod only knows its own cache. There is no mechanism to query the global cache state across all replicas.
3. **No SLO-aware autoscaling**: The KEDA ScaledObject scales on queue depth and cache usage, but not on TTFT/TPOT SLO targets.
4. **No prefill/decode disaggregation**: Large models (e.g., 70B+) benefit from splitting prefill (compute-bound) and decode (memory-bound) onto separate GPU pools, which requires KV-cache transfer between pods.

llm-d (CNCF Sandbox, March 2026) provides:
- **EPP (Endpoint Picker)**: Routes each request to the replica holding the relevant KV-cache blocks via a 4-stage pipeline (Discover→Filter→Score→Select).
- **KV-Cache Indexer**: Maintains a cluster-wide near-real-time map of cache blocks.
- **SLO-aware autoscaling**: Scales based on TTFT/TPOT/cache-hit vs targets.
- **Disaggregated serving (P/D)**: Splits prefill and decode onto independently scalable pods with NIXL-based KV-cache transfer.

## Decision

Integrate llm-d into the platform in 5 phases:

### Phase 1: EPP Cache-Aware Routing (No RDMA required)
- Deploy the llm-d chart (`charts/llm-d/`) with the Router (Envoy + EPP) enabled.
- EPP replaces the consistent-hash heuristic in the ai-gateway BackendTrafficPolicy.
- HTTPRoute targets the InferencePool CRD instead of direct backendRefs.
- No KV-Cache Indexer, no disaggregation.
- **Environment**: Staging

### Phase 2: KV-Cache Indexer for Exact Routing
- Enable the KV-Cache Indexer deployment.
- vLLM pods emit KV-cache events (`LLM_D_KV_EVENTS=true`).
- EPP queries the indexer for precise routing decisions.
- **Environment**: Production

### Phase 3: SLO-Aware Autoscaling
- EPP metrics (TTFT, TPOT, cache-hit) feed into KEDA ScaledObject triggers.
- Augments (not replaces) the existing KEDA triggers.
- **Environment**: Production

### Phase 4: Prefill/Decode Disaggregation (Requires RDMA)
- Two separate StatefulSets/Deployments: prefill (compute-bound) and decode (memory-bound).
- KV-cache transferred via NIXL (RDMA/NVLink).
- Controlled by `disaggregation.enabled` in model-serving-engine chart.
- **Environment**: Production (A100/H100 with RDMA only)

### Phase 5: Wide Expert Parallel for MoE
- LeaderWorkerSet for MoE expert parallelism.
- **Environment**: Production (future, when MoE models are onboarded)

## Updated Components

| Component | Change |
|-----------|--------|
| `engine-selector` | Added `RoutingMode` (ConsistentHash/Epp/EppWithIndexer), `ServingMode` (Unified/Disaggregated/WideExpertParallel), `routing_mode_for()`, `detect_serving_mode()`, `should_disaggregate()` |
| `vram-budget-calc` | Added `--disaggregated` mode with separate prefill/decode budgets |
| `cache-roi-calc` | Added `--precise-hit-rate` for EPP vs heuristic comparison |
| `charts/llm-d/` | New chart: Router (Envoy+EPP), KV-Cache Indexer, InferencePool CRD, ServiceMonitor, PDB |
| `charts/model-serving-engine/` | Added `llmD` and `disaggregation` values sections, llm-d labels and env vars on StatefulSet |
| `charts/ai-gateway/` | HTTPRoute targets InferencePool when llm-d enabled; BackendTrafficPolicy uses RoundRobin (EPP handles routing) |
| `environments/` | dev (disabled), staging (Phase 1 EPP), prod (Phase 2 + indexer) |
| `apps/` | ApplicationSets for llm-d in dev/staging/prod |
| `observability/` | EPP routing alerts, KV-Cache Indexer alerts, disaggregation alerts, Grafana dashboard |

## SSM/Mamba Consideration

SSM/Mamba models use fixed-size recurrent state, not paginable KV-cache. The engine-selector returns `RoutingMode::ConsistentHash` for SSM models regardless of llm-d configuration. llm-d EPP routing is only applied to Transformer and Hybrid model families. MoE models use `RoutingMode::EppWithIndexer` for expert-weight cache locality.

## Risks

1. **CNCF Sandbox instability**: llm-d APIs (InferencePool CRD, EPP plugin interface) may have breaking changes. Mitigated by isolating llm-d behind feature flags (`llmD.enabled: false` by default in dev).
2. **RDMA requirement**: Phases 4-5 require InfiniBand/NVLink. Ensure GPU node pool labels distinguish RDMA-capable nodes.
3. **KV-Cache Indexer SPOF**: The indexer is a new critical component. Mitigated by PDB, multiple replicas in prod, and alerts on availability.
4. **Increased operational complexity**: Additional components to monitor and debug. Mitigated by comprehensive observability (14 new alerts, dedicated Grafana dashboard).

## Consequences

- The platform gains precise cache-aware routing, improving cache hit rates and reducing TTFT.
- Disaggregated serving enables independent scaling of prefill and decode, optimizing GPU utilization for large models.
- SSM/Mamba models are explicitly excluded from llm-d routing, preventing misconfiguration.
- The consistent-hash heuristic remains as a fallback when llm-d is disabled.
- Four Rust tools, five Helm charts, and three environments now fully support both legacy and llm-d modes.