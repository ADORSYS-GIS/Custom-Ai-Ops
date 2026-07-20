# How a transformer serves a request: prefill, decode, and the KV cache

> The entry point to everything else in this knowledge base. One idea to remember: **an LLM generates one token per full pass over its weights, so generation speed is limited by how fast the GPU can *read memory*, not by how fast it can compute.** Nearly every technology we study — vLLM, LMCache, llm-d, quantization, disaggregation — is a response to that single fact.

This is the deliberately-short primer. The deep dives ([kv-cache](../explain/kv-cache.md), [gpu](../explain/gpu.md) — being consolidated per [MIGRATION](../MIGRATION.md)) expand every section.

## What a model physically is

A set of weight tensors (billions of learned numbers) plus a small amount of metadata: `config.json` (architecture shape), tokenizer files (text ↔ token IDs), and a chat template (how conversations are formatted into one token sequence). Weights ship as [safetensors](../reference/glossary.md) files, typically 2 bytes per parameter at BF16.

First napkin formula — **weight memory ≈ parameters × bytes per parameter**: an 8B model at BF16 needs ~16 GB of VRAM before serving a single request. [Tutorial 01](../tutorials/01-anatomy-of-a-model.md) verifies this against real files.

## The two phases of every request

Serving one request has two radically different phases:

**Prefill.** The whole prompt is processed in one parallel pass. Thousands of tokens at once means huge matrix multiplications — this phase is **compute-bound**: the GPU's arithmetic units are the bottleneck. Its duration is what the user feels as time-to-first-token (TTFT), and it grows with prompt length.

**Decode.** Tokens are generated *one at a time* — each new token requires a full pass through all the weights, and the model cannot produce token N+1 before N exists. For every single token, the GPU streams tens of gigabytes of weights from VRAM to do a relatively tiny amount of math per byte moved. This phase is **memory-bandwidth-bound**: the bottleneck is VRAM read speed, and the arithmetic units sit mostly idle. Its pace is the inter-token latency (TPOT) the user feels while text streams.

This asymmetry — one prompt-crunching compute burst, then a long bandwidth-starved trickle — is the central engineering fact of inference. It's why prefill and decode are measured separately ([methodology](../benchmarks/methodology.md)), scheduled separately (chunked prefill), and eventually run on separate hardware pools (P/D disaggregation). See NVIDIA's [benchmarking concepts](https://developer.nvidia.com/blog/llm-benchmarking-fundamental-concepts/) for the standard treatment.

## Why the KV cache exists

Attention makes each new token look at every previous token. Done naively, generating token 1,000 would reprocess all 999 predecessors — the whole generation becomes quadratic and unusable. The fix: cache each token's attention keys and values (its **KV**) the first time it's computed, so each decode step only computes the *new* token and reads the rest from cache.

The cache is the classic space-for-time trade, and the space is significant. Second napkin formula:

**KV bytes per token ≈ 2 × layers × kv_heads × head_dim × bytes-per-value**

Multiply by context length and by concurrent requests, and the KV cache — not the weights — becomes what limits how many users fit on a GPU. A handful of long-context requests can consume more VRAM than the model itself. This is why we call the KV cache *the* scarce resource of inference, and why an entire layer of the stack (part 5 of the [roadmap](../roadmap.md)) exists just to manage it.

## Batching: the lever hiding inside the bottleneck

Decode wastes the GPU's compute — so serve many requests at once. The weights are read from VRAM once per step *regardless of batch size*; each extra request in the batch reuses that same expensive memory traffic. Batching therefore raises throughput dramatically at first, almost free. The costs arrive as the batch grows: each request's KV cache occupies VRAM, and per-token latency creeps up. **Latency ↔ throughput via batch size** is the fundamental dial of serving economics; the saturation curves in our benchmark reports are pictures of this trade.

## What this predicts (and part 1 verifies)

If the model above is right, a naive server — load weights, run `generate()` per HTTP request — must fail in three specific ways:

1. **No batching:** requests are processed one at a time; GPU utilization stays pathetic while users queue. TTFT explodes roughly linearly with concurrency.
2. **Rigid memory handling:** KV memory management is naive (contiguous, worst-case allocations), so VRAM runs out or fragments long before the hardware is actually exhausted.
3. **No scheduling:** a long request blocks short ones behind it; there is no preemption, no fairness, no admission control.

[Tutorial 02](../tutorials/02-serve-a-model-bare.md) builds that server and measures all three. Continuous batching, PagedAttention, and the scheduler — the heart of every modern runtime, and part 2 of the roadmap — are precisely the fixes for failures 1, 2, and 3.

## Sources

- NVIDIA, [LLM Inference Benchmarking: Fundamental Concepts](https://developer.nvidia.com/blog/llm-benchmarking-fundamental-concepts/) (2025)
- Kwon et al., [Efficient Memory Management for LLM Serving with PagedAttention](https://arxiv.org/abs/2309.06180) (vLLM paper, 2023)
- BentoML, [LLM Inference Handbook — metrics](https://bentoml.com/llm/llm-inference-basics/llm-inference-metrics) (2025)
