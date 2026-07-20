# Evaluation Report: mistral-7b-instruct

## Status: PENDING

| Benchmark | Score | Baseline | Pass? |
|-----------|-------|----------|-------|
| MMLU (5-shot) | - | 0.650 | PENDING |
| HellaSwag | - | 0.750 | PENDING |
| ARC-Challenge | - | 0.580 | PENDING |

## Latency (PENDING)

| Metric | Value | Threshold | Pass? |
|--------|-------|-----------|-------|
| TTFT (p50) | - | <500ms | PENDING |
| TTFT (p95) | - | <1000ms | PENDING |
| TPS | - | >20 | PENDING |
| E2E latency (p95) | - | <2000ms | PENDING |

## Recommendation
PENDING — awaiting evaluation results. Model is staged but not yet deployed.
Once GPU quota is allocated, run the smoke test suite (`tests/smoke/vllm-smoke-test.sh`)
and the load test (`tests/load/load-test.js`) to validate latency and throughput
before promoting to LIVE.