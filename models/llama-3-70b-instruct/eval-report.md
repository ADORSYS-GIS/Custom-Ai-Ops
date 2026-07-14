# Evaluation Report: llama-3-70b-instruct

## Status: PENDING

| Benchmark | Score | Baseline | Pass? |
|-----------|-------|----------|-------|
| MMLU (5-shot) | - | 0.790 | PENDING |
| HellaSwag | - | 0.850 | PENDING |
| ARC-Challenge | - | 0.700 | PENDING |

## Latency (PENDING)

| Metric | Value | Threshold | Pass? |
|--------|-------|-----------|-------|
| TTFT (p50) | - | <500ms | PENDING |
| TTFT (p95) | - | <1000ms | PENDING |
| TPS | - | >20 | PENDING |
| E2E latency (p95) | - | <2000ms | PENDING |

## Recommendation
PENDING — awaiting evaluation results. Model is on standby and not yet deployed.
Requires multi-GPU tensor parallelism (2x H100 80 GB) and RDMA fabric for
disaggregated P/D mode. Once infrastructure is provisioned, run the smoke test
suite (`tests/smoke/smoke-test.sh`), the llm-d smoke test (`tests/smoke/llm-d-smoke-test.sh`),
and the load test (`tests/load/load-test.js`) to validate latency, throughput,
and KV cache affinity before promoting to LIVE.