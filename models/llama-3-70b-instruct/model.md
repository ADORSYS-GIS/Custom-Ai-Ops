# Model: llama-3-70b-instruct

## Metadata
- **Format**: safetensors
- **Engine**: vllm
- **Status**: STANDBY
- **GPU Pool**: gpu-h100-pool
- **GPU**: H100 (80 GB, Hopper)
- **Context Length**: 8192
- **Layers**: 80
- **Heads**: 64
- **Quantisation**: fp16
- **Model Size**: 140.0 GB

## VRAM Budget

| Component | Size |
|-----------|------|
| Model weights | 140.00 GB |
| Fixed overhead | 1.00 GB |
| KV cache budget | 0.16 GB |
| Usable VRAM (90%, single GPU) | 72.00 GB |
| **Remaining (single GPU)** | **-69.16 GB (OOM)** |

> **Note**: This model does not fit on a single H100 80 GB GPU. It requires
> tensor parallelism across at least 2x H100 80 GB GPUs (160 GB total, 144 GB
> usable) to accommodate the 140 GB model weights plus overhead and KV cache.

## llm-d Configuration
- **Enabled**: true
- **Routing Mode**: epp-with-indexer (Endpoint Picker + KV-Cache Indexer)
- **Serving Mode**: disaggregated (Prefill/Decode separation)
- **Inference Pool**: llama-3-70b-pool
- **Emit KV Events**: true
- **Disaggregated**: true
- **Prefill GPU Utilization**: 0.92
- **Decode GPU Utilization**: 0.85
- **Phase**: Phase 2 (KV-Cache Indexer) / Phase 4 (P/D disaggregation, requires RDMA)

## Gateway Configuration
- Backend: `llama-3-70b-local`
- Priority: 0 (primary)

## Deployment
- Chart: `model-serving-engine`
- Environment: `environments/prod/`
- Sync wave: 0 (workload)

## History
- 2025-07-14: Model on standby — large model requiring H100 with tensor parallelism. llm-d disaggregated P/D mode planned (Phase 4, requires RDMA).