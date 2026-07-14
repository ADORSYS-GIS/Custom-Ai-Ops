# VRAM Budget Calculation: llama-3-70b-instruct

## Inputs
- GPU: H100 (Hopper)
- Total VRAM: 80.0 GB (per GPU)
- Model size: 140.0 GB
- Quantisation: fp16
- Context length: 8192
- Layers: 80
- Heads: 64

## Calculation (single GPU)

```
Usable VRAM   = 80.0 * 0.90 = 72.00 GB
Model size    = 140.00 GB
Fixed OH      =   1.00 GB
KV cache      =   0.16 GB
Remaining     = 72.00 - 140.00 - 1.00 - 0.16 = -69.16 GB  (OOM)
```

## Result (single GPU)

**OOM RISK**: -69.16 GB remaining. The model does not fit on a single H100 80 GB GPU.

## Multi-GPU Tensor Parallelism (2x H100 80 GB)

```
Total VRAM    = 80.0 * 2 = 160.0 GB
Usable VRAM   = 160.0 * 0.90 = 144.00 GB
Model size    = 140.00 GB
Fixed OH      =   1.00 GB
KV cache      =   0.16 GB
Remaining     = 144.00 - 140.00 - 1.00 - 0.16 = 2.84 GB  (FITS)
```

## Disaggregated P/D Mode Budget

When deployed in disaggregated Prefill/Decode mode (llm-d Phase 4):

| Role | GPU Utilisation | KV Cache Multiplier | Usable VRAM |
|------|----------------|---------------------|-------------|
| Prefill | 0.92 | 0.3x | 73.60 GB per GPU |
| Decode | 0.85 | 1.5x | 68.00 GB per GPU |

> Requires RDMA fabric for KV cache transfer between prefill and decode workers.

## Recommendation

Deploy with tensor parallelism = 2 across 2x H100 80 GB GPUs. Enable disaggregated
P/D mode once RDMA infrastructure is provisioned. Model remains on STANDBY until
H100 quota and RDMA networking are available.