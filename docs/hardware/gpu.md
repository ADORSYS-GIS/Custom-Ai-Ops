# In-Depth Guide: Choosing, Understanding, and Exploiting GPUs for AI (2026)

> Technical reference document for deciding **which GPU to choose, why, for which use case, and how to operate it concretely** in production.

---

## Table of Contents

1. [The 3 GPU Families and Their Design Logic](#1)
2. [Fundamental Concepts to Master Before Choosing](#2)
3. [Detailed Per-GPU Datasheet: Why, Use Case, How to Operate](#3)
4. [Quantitative Microarchitecture Comparison](#4)
5. [Inference Runtimes: Which to Use with Which GPU](#5)
6. [Infrastructure Constraints (Power, Cooling, Network)](#6)
7. [Decision Tree and Summary Table](#7)

---

<a name="1"></a>
## 1. The 3 GPU Families and Their Design Logic

Choosing a GPU is never about the "best GPU in the abstract" but about the **dominant constraint**: budget, required reliability, data volume, tolerated latency. Three families address three different logics.

### 1.1 Consumer GPUs (RTX 3090 / 4090 / 5090)
**Why they exist:** NVIDIA reuses the same silicon as professional cards but removes reliability guarantees (ECC, certified drivers) to lower cost. These are chips optimized for graphics rendering **repurposed** for AI compute — hence an excellent raw performance/price ratio, but no data integrity guarantee over time.

**What structurally limits them:**
- No ECC memory → a silent bit-flip error can corrupt a long training run without any alert triggering.
- No NVLink → impossible to merge memory from multiple cards into a coherent pool; each card remains an isolated memory island connected only via PCIe (64 GB/s), very slow compared to NVLink (900 GB/s+).
- High TDP in desktop form factor (up to 600 W over 3-4 slots) → hard to densify in a rack.

### 1.2 Professional / Workstation GPUs (RTX 6000 Ada)
**Why they exist:** bridge the gap between consumer and datacenter for teams that need reliability (ECC) and rack density without paying the $30-40k price of an SXM accelerator.

**What distinguishes them:**
- ECC on GDDR6 → memory corruption detected and automatically corrected, essential as soon as a job runs more than a few hours.
- Double-slot blower form factor → designed to be stacked densely in a rack, unlike the 3-4 slot form factor of gaming cards.
- Certified enterprise drivers → long-term stability, vendor support.

### 1.3 Datacenter GPUs (H100/H200/B200, MI300X/325X/300A)
**Why they exist:** to address two needs that the previous categories do not cover at all: (1) training a model that fits on no single card (hundreds of billions of parameters), and (2) serving thousands of concurrent requests with guaranteed latency.

**What fundamentally distinguishes them:** HBM memory mounted *on-package* (directly on the die, not on the card) and proprietary interconnects (NVLink, Infinity Fabric) that allow multiple physical GPUs to behave as **a single logical GPU** with a unified memory space.

---

<a name="2"></a>
## 2. Fundamental Concepts to Master Before Choosing

These concepts explain *why* two GPUs with similar specs can have very different real-world performance. You must understand them before reading the product datasheets, otherwise the choice is based on marketing rather than actual workload.

### 2.1 Prefill vs Decode — The Most Important Concept for Choosing an Inference GPU

An LLM request goes through two phases with **opposing** hardware requirements:

- **Prefill** (reading and encoding the prompt): the GPU multiplies large dense matrices over the entire prompt at once. This is massively parallel compute → **limited by TFLOPS** (raw compute power of Tensor Cores).
- **Decode** (token-by-token generation): to produce *each* new token, the GPU must reload the entire model weights + accumulated KV Cache from HBM to its registers. The computation itself is trivial; what is expensive is the **memory transfer**. → **limited by HBM bandwidth**, not TFLOPS.

**Direct practical consequence:** if your workload is decode-dominated (chatbots, long generation), buying a GPU with more TFLOPS but the same memory bandwidth brings almost nothing. You need to buy **bandwidth**.

Quantified proof — Llama 2 70B (140 GB in FP16), single stream:

```
Read time = Model weights / Memory bandwidth

H100 SXM (3,350 GB/s): 140 / 3350 ≈ 41.8 ms → ~24 tokens/s
H200 SXM (4,800 GB/s): 140 / 4800 ≈ 29.2 ms → ~34 tokens/s
```

Same compute die, same TFLOPS: the only change (memory) gives **+42% throughput**. This demonstrates that for decode, bandwidth trumps everything else.

### 2.2 The "CUDA Gap" — Why Paper Specs Lie

On paper, the AMD MI300X shows **1,307 TFLOPS** in dense FP16/BF16 vs **990 TFLOPS** for the H100 — a theoretical advantage of +32.1%. In production, the opposite happens, and the gap widens with scale:

| Context | Real NVIDIA vs AMD Throughput |
|---|---|
| 2 GPUs | H100 +29.4% |
| 4 GPUs | H100 +38.9% |
| 8 GPUs (full node) | H100 +46%, latency -31.9% |
| 16 concurrent users | H100 +30.8% / B200 +76.5% |
| 128 concurrent users | H100 +38.7% / B200 +105.3% |
| 512 concurrent users | H100 +67% / B200 +77.9% |

**Why this gap exists:** real performance depends on the compiler and low-level libraries' (cuBLAS, cuDNN) ability to *actually fill* the compute units without dead time. NVIDIA has 15 years of accumulated CUDA optimization; ROCm (AMD) is a younger ecosystem catching up but has not yet reached the same maturity, particularly under high concurrency where request scheduling becomes critical.

**What this implies for selection:** never size a cluster based on announced TFLOPS alone. Always weight by the CUDA Gap score at the target concurrency level (16, 128, 512 users depending on your real traffic profile).

### 2.3 Internal NUMA — The Hidden Trap of Chiplet GPUs (AMD MI300X)

The MI300X is not a monolithic GPU: it is **8 compute chiplets (XCDs)** connected to distributed HBM memory. This creates NUMA (Non-Uniform Memory Access) behavior *within the GPU itself*:

```
Access to a XCD's local HBM    : ~0.66 TB/s, latency ~50 cycles
Access to a neighboring XCD's HBM: ~0.30 TB/s, latency ~100 cycles (via Infinity Fabric)
```

**Why this matters:** if the compiler or runtime does not intelligently place tensors on the right XCDs, a significant portion of memory accesses traverse the internal interconnect at half speed. This is a frequent source of undocumented underperformance — verify explicitly if evaluating MI300X for low-latency inference.

### 2.4 Memory Sizing Formula (VRAM)

```
Minimum VRAM = Model weights + Activity headroom

Model weights = Parameters (billions) × Precision (bytes/parameter)
   FP16/BF16 = 2 bytes  |  FP8 = 1 byte  |  FP4 = 0.5 bytes

Activity headroom = KV Cache + Activations
   (grows with context length and batch size)
```

Example: Llama 70B in FP16 = 70 × 2 = **140 GB** of weights alone, before even reserving space for the KV Cache — hence the impossibility of fitting it on an 80 GB card (H100) without quantizing or distributing across multiple GPUs.

---

<a name="3"></a>
## 3. Detailed Per-GPU Datasheet: Why, Use Case, How to Operate

### RTX 4090 / RTX 5090 — Prototyping and Lightweight Models

**Why choose them:** unbeatable performance/price ratio for raw compute at low batch. The RTX 5090 brings GDDR7 (1,792 GB/s, +78% vs 4090) which directly benefits decode.

**Concrete use cases:**
- Local development of an inference pipeline before deployment.
- Serving models ≤8B (Llama 3.1 8B) at very high throughput for minimal operating cost (>90 tok/s observed).
- Running quantized models (Q4/Q8) for individual research needs.

**How to operate them:**
- Always quantize to Q4/Q8 to free VRAM for larger activations/KV Cache, rather than staying in FP16 by default.
- Use vLLM rather than proprietary runtimes — the consumer CUDA ecosystem is better covered by open-source tools.
- Do not attempt dense multi-GPU: without NVLink, memory aggregation between cards goes through PCIe, which cancels much of the benefit.

**What NOT to expect:** ECC reliability, 24/7 production-critical service, efficient multi-card memory aggregation.

---

### RTX 6000 Ada — Local Fine-Tuning and Intermediate Models

**Why choose it:** the only entry point with ECC and rack form factor below the SXM accelerator threshold. 48 GB allows fitting a ~32B model quantized to Q4 (≈16 GB of weights) while keeping over 30 GB for KV Cache and concurrency.

**Concrete use cases:**
- Local fine-tuning of intermediate models (Qwen 3 32B and equivalents) without depending on the cloud.
- Internal inference server for a team (tens of simultaneous users, not thousands).
- Test environment before migrating to an H100/H200 production cluster.

**How to operate it:**
- Systematically quantize models >16B to Q4 to stay comfortably under the 48 GB limit and keep margin for dynamic batching.
- The 300 W TDP and double-slot blower form factor allow densification to 4-8 cards per server — consider for a small internal cluster before investing in SXM.
- Prefer this GPU as soon as reliability (ECC) becomes a non-negotiable criterion, even without massive scaling needs.

---

### NVIDIA H100 SXM — The Established Enterprise Standard

**Why choose it:** most mature software ecosystem (CUDA, cuBLAS optimized for years), NVLink 4.0 to aggregate multiple cards into a coherent memory pool, broad market availability (cloud and secondary market).

**Concrete use cases:**
- Enterprise inference with guaranteed latency (SLA) on models up to ~70B with tensor parallelism (TP2 minimum, since 80 GB < 140 GB required in FP16).
- Distributed training of medium-to-large models on clusters of several hundred GPUs.
- Any workload where software maturity trumps raw memory capacity.

**How to operate it:**
- For 70B+, plan tensor parallelism (TP2 or more) from the design stage — a single H100 card is not enough.
- Leverage the Transformer Engine (dynamic FP8) to halve the memory footprint without rewriting the model.

---

### NVIDIA H200 SXM — The Default Choice for Production Inference in 2026

**Why choose it:** same compute die as the H100 (thus same software maturity and same TFLOPS), but 141 GB of HBM3e at 4.8 TB/s instead of 80 GB at 3.35 TB/s. Since decode is bandwidth-limited (see §2.1), this is a direct throughput gain without changing a line of application code.

**Concrete use cases:**
- Serving 70B models in FP8 (~70 GB) or FP16 on a **single card**, without TP required — greatly simplifies the deployment architecture.
- Very high concurrency inference where the freed memory space serves to enlarge the KV Cache and thus the number of requests processed in parallel.
- Long contexts (RAG, multi-turn agents) where the KV Cache becomes the limiting factor rather than model weights.

**How to operate it:**
- Size dynamic batching (continuous batching) by leveraging the freed memory space rather than keeping the same settings as on H100.
- Consider the NVL variant (4 cards, 1.8 TB/s, 564 GB unified) if the need exceeds a single card without wanting to build a full SXM cluster.

---

### NVIDIA B200 (Blackwell) — Maximum Throughput and Precision Flexibility

**Why choose it:** dual-die architecture with 192 GB unified memory space @ 8 TB/s, 5th-gen Tensor Cores with asynchronous thread execution (eliminates warp-synchronous wait times of previous generations), and above all native **FP4/FP6** support that drastically reduces the weight memory footprint (70B in FP4 ≈ 35-40 GB instead of 140 GB).

**Concrete use cases:**
- Very high concurrency inference (128-512 users) where the gap with AMD reaches +77 to +105% — the most profitable choice as soon as traffic is high and sustained.
- Giant models requiring FP4 to fit on a reasonable number of cards.
- Exascale training via the unified GB200 NVL72 rack (72 GPUs in a single NVLink domain at 130 TB/s).

**How to operate it:**
- Migrate weights to FP4/FP6 as soon as the model's output precision allows — this is the main lever to fully exploit this generation's memory capacity.
- The autonomous RAS engine can detect thermal drift or silent failures before service interruption — integrate it into cluster monitoring rather than leaving it as an ignored background task.
- Anticipate the TDP (up to 1,200 W/card): liquid cooling is mandatory, traditional forced air is no longer sufficient (see §6).

---

### AMD Instinct MI300X — Maximum Raw Memory Capacity

**Why choose it:** 192 GB of HBM3 on a single card, more than the H200 (141 GB), at a TCO often lower than the NVIDIA equivalent. Relevant when the dominant constraint is *available memory capacity*, not peak latency under high concurrency.

**Concrete use cases:**
- Loading full 70B FP16 models on a single card, without quantization or parallelism, when maximum precision is required.
- Environments where hardware budget trumps peak latency (moderate traffic, no aggressive SLA).
- Organizations committed to the open ROCm ecosystem for strategic reasons (independence from NVIDIA).

**How to operate it:**
- Use vLLM with the `ROCM_AITER_FA` backend (enterprise-optimized) rather than `ROCM_ATTN` as soon as the card is an MI300X — the latter falls back to slow emulation kernels for asymmetric attention heads.
- Account for internal NUMA behavior (§2.3) when placing tensors: poorly managed, the raw memory advantage can be canceled by inter-XCD access latency.
- Avoid very high concurrency deployments (512+ users) where the CUDA Gap with NVIDIA is widest — reserve this GPU for moderate-concurrency workloads.

---

### AMD Instinct MI300A — Scientific Computing and HPC

**Why choose it:** it is an APU, not a pure GPU — CPU (Zen 4, 24 cores) and GPU share the same coherent HBM3 memory (128 GB, 5.3 TB/s) via the Infinity Cache. This completely eliminates data copies via PCIe between host and device.

**Concrete use cases:**
- Double-precision (FP64) scientific computing where CPU↔GPU round-trips are frequent (physics simulations, molecular dynamics).
- Classic HPC workloads migrated to AI where existing code assumes a unified memory space.
- Any pipeline where host-device PCIe transfer latency is the identified bottleneck.

**How to operate it:**
- Design code to leverage native memory coherence rather than replicating an explicit copy pattern inherited from classical discrete GPU architectures.
- Reserve this choice for workloads where FP64 is truly necessary — for standard LLM inference (FP16/FP8/FP4), this is not the right tool.

---

<a name="4"></a>
## 4. Quantitative Microarchitecture Comparison

| GPU | Microarchitecture | Memory | Bandwidth | TDP | Interconnect | Form Factor |
|---|---|---|---|---|---|---|
| RTX 4090 | Ada Lovelace | 24 GB GDDR6X | 1,008 GB/s | 450–600 W | PCIe4 x16 (64 GB/s) | Desktop |
| RTX 5090 | Ada Lovelace (AD102) | 32 GB GDDR7 | 1,792 GB/s | 575 W | PCIe4 x16 (64 GB/s) | Desktop |
| RTX 6000 Ada | Ada Lovelace | 48 GB GDDR6 ECC | 960 GB/s | 300 W | PCIe4 x16 (64 GB/s) | Rack (blower) |
| A100 SXM | Ampere | 80 GB HBM2e | 2,039 GB/s | 400 W | NVLink 3.0 (600 GB/s) | SXM |
| H100 SXM | Hopper | 80 GB HBM3 | 3.35 TB/s | 700 W | NVLink 4.0 (900 GB/s) | SXM |
| H200 SXM | Hopper | 141 GB HBM3e | 4.8 TB/s | 700 W | NVLink 4.0 (900 GB/s) | SXM |
| B200 | Blackwell | 192 GB HBM3e | 8.0 TB/s | 1,000–1,200 W | NVLink 5.0 (1.8 TB/s) | SXM / HGX |
| MI300X | CDNA 3 | 192 GB HBM3 | 5.3 TB/s | 750 W | Infinity Fabric 3 | OAM |
| MI325X | CDNA 3 | 256 GB HBM3e | 6.0 TB/s | 750 W | Infinity Fabric 3 | OAM |
| MI300A | CDNA 3 (APU) | 128 GB shared HBM3 | 5.3 TB/s | 550–760 W | Internal Infinity Fabric | Socket SH5 |

---

<a name="5"></a>
## 5. Inference Runtimes: Which to Use with Which GPU

### vLLM — Agility and Portability
**Why:** implements **PagedAttention**, which eliminates KV Cache fragmentation (memory management by pages rather than contiguous blocks, like an OS manages RAM).

**How to operate it:**
- vLLM V1 (2025+) replaced HIP kernels with custom GPU kernels, developed in 3 optimization phases (vectorized loads for prefill, specialized kernel for decode at sequence=1, fusion into a single kernel) — use this version rather than V0, now obsolete.
- On AMD, explicitly choose `ROCM_AITER_FA` on MI300X+ rather than the generic `ROCM_ATTN` backend.
- Ideal for rapid prototyping and mixed NVIDIA/AMD environments thanks to its portability.

### vLLM — Summary

| Dimension | vLLM |
|---|---|
| Hardware | NVIDIA + AMD ROCm 6.2+ |
| Complexity | Low |
| Multi-model | Limited |
| Use case | Prototyping, mixed NVIDIA/AMD, production inference |

---

<a name="6"></a>
## 6. Infrastructure Constraints: Why They Condition GPU Choice

Choosing a GPU without verifying its energy and thermal feasibility leads to projects blocked at the deployment phase — the bottleneck in 2026 is no longer silicon but electricity and cooling.

### Power
- 1 node 8× H100 ≈ **10.1 kW**; 1 node 8× B200 ≈ **14.3 kW**.
- 100 GPU cluster ≈ 176 kW (standard commercial connection) · 1,000 GPU ≈ 1.76 MW (industrial transformers) · 10,000 GPU ≈ 17.6 MW (dedicated substation).
- High-voltage connection delay: often **24 to 36 months** in major technology hubs → a criterion to verify *before* validating a large-scale GPU choice, not after.
- **Mitigation strategy:** distribute load across multiple 100-GPU sites rather than concentrating a 1,000-GPU cluster on a single site — each site stays below grid alert thresholds.

### Cooling
- Physical limit of forced air: **~20 kW/rack**.
- An H100 rack (~40 kW) already exceeds this limit; a Blackwell rack (>120 kW) makes it absolute → **liquid cooling mandatory from the dense Hopper generation onward**.
- Direct-to-Chip (copper cold plates): ~3,500× more efficient than air.
- Two-phase immersion for extreme densities (passive boil/condensation cycle).

**Direct implication on GPU choice:** if the existing infrastructure has no liquid cooling, a dense deployment of B200 or even H100 in a full rack is not feasible without preliminary work — a factor often overlooked when selecting hardware.

### Inter-GPU Network
- **InfiniBand** (400-800 Gbps): native RDMA, minimum latency, reference for massive training.
- **Optimized Ethernet** (Spectrum-X + BlueField-3 + RoCEv2): performance close to InfiniBand with standard Ethernet flexibility — relevant if the network team already masters Ethernet rather than InfiniBand.
- **Rail-Optimized** topology: each GPU has its own dedicated NIC to a Leaf switch → All-Reduce without packet collision.

---

<a name="7"></a>
## 7. Decision Tree and Summary Table

```
What is the dominant constraint?
│
├─ Minimal budget, prototyping only
│   → RTX 4090/5090 + vLLM, Q4 quantized models
│
├─ Reliability (ECC) + local fine-tuning, no massive scale
│   → RTX 6000 Ada + vLLM
│
├─ Production 70B with strict SLA, mature ecosystem required
│   → H200 SXM + vLLM
│
├─ Raw memory capacity prioritized over peak latency
│   → MI300X + vLLM (ROCM_AITER_FA backend)
│
├─ Very high concurrency (128-512+ simultaneous users)
│   → B200 + vLLM (widest performance gap)
│
├─ FP64 scientific computing / HPC
│   → MI300A
│
└─ Exascale training, multi-thousand GPU
    → GB200 NVL72 + dedicated liquid infrastructure
```

| Profile | Recommended GPU | Runtime | Why |
|---|---|---|---|
| Cloud provider / giant training | GB200 NVL72 | — | Only option for rack-scale interconnect without network bottleneck |
| Enterprise / production LLM, strict SLA | H200 SXM | vLLM | Best throughput/latency tradeoff, most mature ecosystem |
| Memory budget priority | MI300X | vLLM (AITER_FA) | 192 GB on a single card, competitive TCO |
| FP64 scientific HPC | MI300A | ROCm | Coherent CPU/GPU memory, zero PCIe copy |
| SME / research / local fine-tuning | RTX 6000 Ada | vLLM | Controlled cost, ECC, rapid iteration |
| Individual prototyping | RTX 4090/5090 | vLLM | Performance/price ratio, no reliability constraint |

---