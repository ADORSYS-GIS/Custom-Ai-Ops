# Gateway Federation

## Architecture

All model-serving backends expose a uniform OpenAI-compatible API through Envoy AI Gateway. From the client perspective, self-hosted models and SaaS providers (OpenAI, Anthropic) are interchangeable.

## Priority Routing

- **Priority 0**: Self-hosted model (primary)
- **Priority 1**: SaaS fallback (activated when latency exceeds 2000ms or error rate > 5%)

## Health Checks

- **Active**: HTTP GET `/health` every 10s, timeout 2s, 3 failures → unhealthy
- **Passive**: Track response times; >2000ms triggers passive failover

## Rate Limiting & Load Shedding (Layer 1)

Protects the vLLM KV cache from request floods. Excess requests are shed at the edge before reaching the engine.

| Mechanism | Config | Failure Mode Prevented |
|---|---|---|
| Rate limiting | 50 req/s per `x-api-key` → HTTP 429 | Request floods overwhelming KV cache |
| Aggressive timeout | request 10s / backendRequest 8s | Queue thrashing from slow requests |

Configured in `charts/ai-gateway/templates/backend-traffic-policy.yaml` via `BackendTrafficPolicy.rateLimit` and `BackendTrafficPolicy.timeout`.

## Payload Validation (Layer 1)

Rejects oversized or malformed payloads before they reach vLLM, preventing KV cache pollution.

| Mechanism | Config | HTTP Response |
|---|---|---|
| Body size limit | `maxBodySize: 4MiB` | 413 |
| Required fields | `model`, `messages` | 400 |
| Max messages | 100 | 413 |

Configured in `charts/ai-gateway/templates/payload-validation.yaml` via `HTTPRouteFilter` (Envoy Gateway extension).

## Sticky Routing (Layer 1)

Routes requests with the same prompt prefix to the same backend replica, maximising vLLM's prefix cache hit rate.

- Header `x-sticky-session-key` added via `RequestHeaderModifier` filter
- Leverages vLLM's `--enable-prefix-caching` by ensuring repeated system prompts / few-shot examples land on the same replica

## Circuit Breaker

- Strategy: `Prioritized` (priority 0 → 1)
- Retry on 502/503/504 (2 attempts, 500ms backoff)
- Per-retry timeout: 5s

## Configuration

All backends and models are defined declaratively in `charts/ai-gateway/values.yaml`, enabling zero-code model additions and failover configuration changes.