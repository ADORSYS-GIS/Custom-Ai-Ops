# Benchmark reports

The evidence layer. Every performance claim made anywhere in this repo — in an ADR, an explanation, a meeting — should trace to a report here.

Rules:

- **Immutable.** A report is never edited after merge; new results = new dated report, optionally marking the old one "Superseded by".
- **Named** `YYYY-MM-DD-short-slug.md`, e.g. `2026-08-14-vllm-prefix-caching-ab.md`.
- **Complete or invalid.** A report missing hardware, versions, or workload definition cannot be cited. Use [`template.md`](template.md).
- **Workloads are named and versioned** (e.g. `chat-multiturn-shared-prefix@v1`). Cache-friendly vs cache-hostile workloads differ up to 10× on identical hardware; an unnamed workload makes a report meaningless.
- Report percentiles (p50/p90/p99), never just means; state the SLO used for goodput.

Methodology reference: [NVIDIA — LLM Inference Benchmarking: Fundamental Concepts](https://developer.nvidia.com/blog/llm-benchmarking-fundamental-concepts/). Tools of record: [GuideLLM](https://github.com/vllm-project/guidellm), NVIDIA aiperf, [inference-perf](https://github.com/kubernetes-sigs/inference-perf).
