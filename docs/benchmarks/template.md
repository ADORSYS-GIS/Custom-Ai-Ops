# <YYYY-MM-DD> — <what was measured, in one line>

**Question.** The single question this benchmark answers.

## Setup

| | |
|---|---|
| Hardware | GPU model, VRAM, CPU, RAM, node count |
| Model | name, size, precision/quant, source repo |
| Engine | name + exact version + relevant flags (full command in appendix) |
| Serving stack | k8s? router? cache layers? versions |
| Workload | named workload + version, prompt/output length distribution, concurrency sweep range |
| Tool | GuideLLM / aiperf / inference-perf + version |
| SLO for goodput | e.g. TTFT < 2 s AND TPOT < 50 ms |

## Results

Table or chart per sweep point: TTFT p50/p90/p99, TPOT p50/p90/p99, throughput (tok/s), goodput. Raw data path/link.

## Interpretation

What the numbers mean, honestly, including anomalies and what this does NOT show.

## Reproduce

Exact commands, configs, and dataset references.
