# Learning roadmap: cost- and performance-optimal model serving

A living page. Ten parts, weights-upward. Each part produces documents in this repo's structure: theory → `explanation/`, guided labs → `tutorials/` or `how-to/`, numbers → `benchmarks/` reports, choices → `adr/`, operations → `runbooks/`. **A part is done when its documents are merged** (see [CONTRIBUTING](../CONTRIBUTING.md)).

Note: this repo already contains substantial theory (`explain/` bibles, `architecture/` series). The roadmap's job is to convert that reading into *verified* understanding — every part pairs theory with an experiment that could prove us wrong.

## The operating loop (every part, same loop)

1. **Study** → write/refine the `explanation/` doc in our own words.
2. **Hypothesize** → falsifiable, numeric prediction, written *before* touching hardware (recorded in the experiment, see [`../experiments/README.md`](../experiments/README.md)).
3. **Experiment** → per [`benchmarks/methodology.md`](benchmarks/methodology.md); results become an immutable dated report.
4. **Decide** → if it informs a real choice, an ADR.
5. **Teach** → 30-min internal session. Can't explain it → don't own it yet.

Wrong predictions are the most valuable outcome; document them.

## The optimization surface (memorize this table)

| Layer | Levers |
|---|---|
| Workload | prompt/context design, streaming, request shaping, semantic caching, model routing |
| Orchestration | autoscaling signals, KV-aware routing, P/D disaggregation, multi-node |
| KV cache | prefix reuse, offload tiers (GPU→CPU→NVMe→remote), cross-instance sharing |
| Runtime | continuous batching, paged attention, chunked prefill, scheduling, speculative decoding |
| Model | quantization (FP8/INT4/NVFP4), smaller/distilled models, multi-LoRA |
| Parallelism | tensor / pipeline / expert / data |
| Hardware & cost | GPU selection, MIG/fractional, spot, utilization economics |

## Parts

| # | Part | Core question | Key deliverables | Exit criterion |
|---|---|---|---|---|
| 1 | **Foundations & bare serving** | What is a model, physically, and what breaks when you serve it naively? | `explanation/how-a-transformer-serves-a-request.md` · tutorials 01–02 · baseline benchmark report | Anyone can whiteboard prefill/decode/KV and size a model on paper within 10%; naive-serving failure measured, not recited |
| 2 | **The runtime layer** | What does vLLM do, component by component, and what is each worth? | One explanation doc + one on/off benchmark report per component (continuous batching, PagedAttention, prefix caching, chunked prefill, scheduling knobs) · SGLang & TensorRT-LLM comparison · ADR: default engine | Given a latency complaint, the team names the knob and predicts the direction of the effect |
| 3 | **Model-level optimization** | How many bits do we actually need? | BF16/FP8/INT4 quality+perf matrix (lm-eval gate mandatory) · speculative-decoding crossover report · ADR: precision policy | Precision picked and defended with quality *and* perf numbers |
| 4 | **Parallelism** | How do models span GPUs? | TP scaling-efficiency report · explanation of TP/PP/EP/DP and MoE serving | Given model + fleet, a parallelism layout proposed without googling |
| 5 | **The KV cache layer** | When does reuse beat recompute? | LMCache on/off per workload profile · aggregated vs disaggregated at equal GPU count · KV-hierarchy explanation (consolidating the existing bibles per [MIGRATION](MIGRATION.md)) | From a workload's prefix profile, cache payoff predicted *before* the test |
| 6 | **Kubernetes-native serving** | How does a fleet route, scale, and share cache? | Routing report (random vs prefix-aware, Gateway API Inference Extension) · llm-d deep dive · comparison: llm-d vs Dynamo vs AIBrix vs KServe · KEDA autoscaling-signal report · ADR: fleet architecture · deploy/scale/upgrade how-tos | Multi-replica KV-aware autoscaling stack running; every routing decision explainable from logs |
| 7 | **Cost engineering** | What does a token cost, and how do we plan capacity? | The team cost model (goodput-based, validated against one real deployment) · capacity-planning how-to · builds on `architecture/07-capacity-forecasting.md` | Hypothetical product → defensible fleet design + monthly cost in under a day |
| 8 | **Production operations** | What happens at 03:00? | Golden dashboards · chaos-drill-derived runbooks · benchmark regression gate in CI · ADR: SLOs | Simulated incident diagnosed from dashboards in <15 min |
| 9 | **The frontier** (continuous) | What's next beyond llm-d? | Watchlist triage → lab trials: model routing/cascades, semantic caching, agentic serving, long-context KV stores (Mooncake-style), alt silicon, multi-cluster | Monthly digest filed; one frontier trial per quarter |

## Sequencing rules

- **Same model family parts 1–6** so every number stays comparable to the part-1 baseline.
- **One variable per experiment.** The methodology doc is law.
- Parts 1–2 done together as a team (shared vocabulary is the point); parts 3+ split into owner-pairs.
- Hardware: parts 1–3 need one ~24 GB GPU (rented is fine); parts 4+ need 2–4 GPUs intermittently. Never buy before part 7 teaches you what to buy.
