# VRAM Budget Calculation: mistral-7b-instruct

## Inputs
- GPU: A100 (Ampere)
- Total VRAM: 40.0 GB
- Model size: 14.0 GB
- Quantisation: bf16
- Context length: 32768
- Layers: 32
- Heads: 32

## Calculation

```
Usable VRAM   = 40.0 * 0.90 = 36.00 GB
Model size    = 14.00 GB
Fixed OH      =  1.00 GB
KV cache      =  0.13 GB
Remaining     = 36.00 - 14.00 - 1.00 - 0.13 = 20.88 GB
```

## Result

**FITS**: 20.88 GB remaining after all allocations.

The model fits comfortably on a single A100 40 GB GPU with ample headroom for KV cache
growth under concurrent requests. No tensor parallelism required.