# Benchmark methodology

The standard behind every report in this directory. A number produced outside this methodology is an anecdote, not a result. Reports themselves are immutable; this methodology is living and versioned — each report states which version it followed (`methodology@v1`).

**Current version: v1 (2026-07-20)**

## 1. Metrics

| Metric | Definition | Notes |
|---|---|---|
| TTFT | request sent → first token received, client-side, streaming on | queueing + prefill |
| TPOT / ITL | mean time between subsequent tokens | decode + batch pressure |
| E2E latency | request sent → last token | ≈ TTFT + TPOT × output_tokens |
| Throughput | output tok/s across the system | raw capacity |
| Goodput | throughput counting only requests meeting the stated SLO | the honest headline |
| $/Mtok | (GPU-$/hr × hrs) / Mtok served at SLO | the final scoreboard |

Always p50/p90/p99, never just means. Server-side context captured alongside every run (for explanation, not headlines): queue depth, KV-cache utilization %, preemption count, prefix-cache hit rate, GPU util/memory.

## 2. Named workload profiles

Traffic shape changes every conclusion; a report without a named workload is invalid. Profiles are versioned (bump on any change to distributions/datasets):

| Profile | ISL/OSL (approx tokens) | Traits |
|---|---|---|
| `uniform-synthetic@v1` | fixed 1k/1k | control profile for isolating variables |
| `chat-multiturn@v1` | 1–4k in / 100–400 out | growing shared prefixes; KV-reuse-friendly |
| `rag@v1` | 4–16k in / 200–600 out | long prefill, shared document chunks |
| `summarize@v1` | 8–32k in / 100–300 out | prefill-dominated |
| `codegen@v1` | 1–8k in / 500–2k out | decode-heavy |
| `agentic@v1` | growing context, tool loops | extreme prefix reuse, bursty |

Each profile's dataset + generator config + seed lives in `experiments/workloads/`. Replayed sanitized production traffic supersedes synthetic profiles once available.

## 3. Load patterns

- **Sweep, don't spot-check.** Concurrency or arrival-rate sweep producing the full saturation curve; a single point hides the knee.
- Poisson arrivals for realism headlines; constant concurrency for controlled comparisons; a burst pattern when testing autoscaling/routing.

## 4. Validity rules

1. One variable per experiment; both arms pinned to image digests.
2. Warmup ≥1 discarded run (unless cold start is the subject).
3. ≥3 repetitions; report median run + spread. Wild disagreement between runs is a finding, not noise to average away.
4. Measure client-side, streaming on, from a separate machine/pod — never the GPU node.
5. Environment manifest recorded (see [`../../experiments/README.md`](../../experiments/README.md)).
6. Know whether you're measuring the engine or the queue: state offered load relative to the saturation knee.
7. Model-side changes (quantization, spec decoding, model swap) ship with a quality delta (lm-eval-harness + internal prompt set) or are disqualified.
8. No benchmarketing: both sides of a comparison get equal tuning effort, or the report states that they didn't.

## 5. Tools of record

| Purpose | Tool |
|---|---|
| Primary load generator | [GuideLLM](https://github.com/vllm-project/guidellm) (profiles, sweeps, goodput, JSON output; engine-agnostic) |
| Cross-checks | NVIDIA aiperf, [inference-perf](https://github.com/kubernetes-sigs/inference-perf) (quarterly, for tool bias) |
| Quality gate | lm-eval-harness + internal prompt set |
| Server metrics | Prometheus/Grafana (scraped during every run) |

One primary generator keeps months of results comparable.

## 6. Citation format

> p95 TTFT 412 ms → 187 ms (−55%), `chat-multiturn@v1` @ 32 concurrent, 3 runs — [2026-08-02-prefix-caching-ab](2026-08-02-prefix-caching-ab.md)
