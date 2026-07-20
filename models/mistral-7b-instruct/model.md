# Model: mistral-7b-instruct

## Metadata
- **Format**: safetensors
- **Engine**: vllm
- **Status**: STAGED
- **GPU Pool**: gpu-a100-pool
- **GPU**: A100 (40 GB, Ampere)
- **Context Length**: 32768
- **Layers**: 32
- **Heads**: 32
- **Quantisation**: bf16
- **Model Size**: 14.0 GB

## VRAM Budget

| Component | Size |
|-----------|------|
| Model weights | 14.00 GB |
| Fixed overhead | 1.00 GB |
| KV cache budget | 0.13 GB |
| Usable VRAM (90%) | 36.00 GB |
| **Remaining** | **20.88 GB** |

## llm-d Configuration
- **Enabled**: true
- **Routing Mode**: epp (Endpoint Picker)
- **Serving Mode**: unified
- **Inference Pool**: mistral-7b-pool
- **Emit KV Events**: true
- **Disaggregated**: false
- **Phase**: Phase 1 (EPP routing)

## Gateway Configuration
- Backend: `mistral-7b-local`
- Priority: 0 (primary)

## Deployment
- Chart: `model-serving-engine`
- Environment: `environments/prod/`
- Sync wave: 0 (workload)

## History
- 2025-07-14: Initial onboarding — model staged for A100 (40 GB) deployment, awaiting quota. llm-d EPP routing enabled (Phase 1).