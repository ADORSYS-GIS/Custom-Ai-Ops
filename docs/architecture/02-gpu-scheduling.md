# GPU Scheduling

## Node Pools

| Pool | Hardware | Use Case |
|------|----------|----------|
| `gpu-h100-pool` | NVIDIA H100 | vLLM high-performance |
| `gpu-a100-pool` | NVIDIA A100 | vLLM standard LLM |
| `gpu-l4-pool` | NVIDIA L4 | ONNX lightweight inference |
| `gpu-edge-pool` | GPU modest (RTX A2000) | ONNX small models, PoC |
| `cpu-pool` | CPU only | Preprocessing, gateway, auxiliary services |

## Node Isolation

vLLM pods are isolated on GPU-only nodes via `nodeSelector`:

```yaml
nodeSelector:
  nvidia.com/gpu.present: "true"
```

This prevents CPU-only workloads from competing for GPU node RAM, which could trigger the host OOM killer and evict vLLM pods (destroying the KV cache).

## QoS Guaranteed

All vLLM pods use **QoS Guaranteed** — `requests` strictly equal `limits` for CPU, memory, and GPU:

| Environment | CPU | Memory | GPU |
|---|---|---|---|
| Prod | 8 | 32Gi | 1 |
| Staging | 4 | 24Gi | 1 |
| Dev | 2 | 16Gi | 1 |

This ensures vLLM pods are **never** evicted by the host OOM killer in favour of Burstable pods.

## Swap Disable (swapoff DaemonSet)

Swap on GPU nodes is disabled via a DaemonSet (`charts/model-serving-engine/templates/swapoff-daemonset.yaml`):

- Runs `nsenter -t 1 -m -u -i -n -p -- swapoff -a` on the host
- `hostPID: true`, `hostNetwork: true`
- `nodeSelector: nvidia.com/gpu.present: "true"`
- Tolerates GPU taints (`nvidia.com/gpu`, `node-role.kubernetes.io/gpu`)
- Capabilities: `SYS_ADMIN`, `SYS_PTRACE`
- Sync-wave: `-2` (runs before model pods)

**Why**: if the host swaps KV cache pages to CPU RAM, inference latency spikes by 10-100x.

## VRAM Budget Formula

```
Usable VRAM = Total VRAM × 0.90
Available   = Usable VRAM − Model Size − 1 GB Fixed Overhead − KV Cache
KV Cache    = 2 × Batch × Context × Layers × Heads × Bytes-per-weight / 1024³
```

If `Available < 0`, deployment is **blocked** by `vram-budget-calc`.

## Hardware Constraints

- **FP8 rejected on Ampere** (RTX A2000, A100 lack FP8 Tensor Cores)
- Minimum quantisation enforced per GPU pool

## Tooling

- `tools/vram-budget-calc` — CI gate, validates budget before merge
- DCGM Exporter — GPU metrics (utilisation, memory, temperature, ECC)
- Kueue — quota and queue management
- Karpenter — on-demand node provisioning