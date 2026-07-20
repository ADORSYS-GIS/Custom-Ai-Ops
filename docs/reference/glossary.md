# Glossary

One-sentence definitions, alphabetical. Link here from every other doc; add terms the week they first appear.

| Term | Definition |
|---|---|
| **ADR** | Architecture Decision Record — a one-page, dated record of a technology decision, its context, and its consequences. |
| **Chunked prefill** | Splitting a long prompt's prefill into chunks interleaved with decode steps so it doesn't stall other requests' token streams. |
| **Continuous batching** | Scheduling that admits/evicts requests at every decode step rather than at batch boundaries; the single biggest throughput win in LLM serving. |
| **Decode** | The token-by-token generation phase; memory-bandwidth-bound because the whole model is read from VRAM per token. |
| **Disaggregation (P/D)** | Running prefill and decode on separate GPU pools, transferring KV cache between them, so each phase scales independently. |
| **DRA** | Dynamic Resource Allocation — the Kubernetes-native GPU/device allocation model (GA since k8s 1.34) replacing device plugins. |
| **Goodput** | Throughput counting only requests that met their SLO (TTFT + TPOT targets); the honest headline metric. |
| **GGUF** | llama.cpp's single-file quantized model format, standard for CPU/edge serving. |
| **ITL** | Inter-token latency; see TPOT. |
| **KV cache** | Per-token key/value tensors cached to avoid recomputing attention at every decode step; grows with context × batch and is the central resource problem of LLM serving. |
| **LMCache** | The KV-cache layer for vLLM: offloads/shares cache across CPU RAM, disk, and remote tiers (CacheGen compression, CacheBlend non-prefix reuse). |
| **llm-d** | CNCF Kubernetes-native distributed inference framework: vLLM + Gateway API Inference Extension + inference scheduler, with cache-aware routing and P/D disaggregation. |
| **NIXL** | The KV/data transfer library (from NVIDIA Dynamo) adopted across engines and orchestrators for GPU-to-GPU cache movement. |
| **PagedAttention** | vLLM's virtual-memory-style block management for KV cache; eliminates fragmentation. |
| **Prefill** | The parallel processing of the entire prompt in one pass; compute-bound. |
| **Prefix caching** | Reusing KV blocks for shared prompt prefixes (system prompts, chat history); generalized by SGLang's RadixAttention. |
| **Quantization** | Serving weights at reduced precision (FP8, INT4/AWQ/GPTQ) for 2–4× less VRAM and bandwidth at near-equal quality. |
| **Speculative decoding** | A cheap drafter proposes tokens verified by the target model in one pass; a latency optimization for low-concurrency serving. |
| **TPOT** | Time per output token during decode; >~50 ms/token feels sluggish when streaming. |
| **TTFT** | Time to first token — queueing plus prefill; what users perceive as responsiveness. |
