# 01 — Anatomy of a model

You will download a real model, look inside every file it ships with, predict its memory footprint on paper, and verify your prediction against reality. At the end you can size any model without downloading it.

Theory lives in [how a transformer serves a request](../explanation/how-a-transformer-serves-a-request.md) — read it first.

**Starting point:** Linux/macOS, Python 3.11+, ~4 GB disk, internet. No GPU required.

## 1. Set up

```bash
mkdir -p ~/labs/01-anatomy && cd ~/labs/01-anatomy
python3 -m venv .venv && source .venv/bin/activate
pip install "huggingface_hub>=0.30" "safetensors>=0.4" "transformers>=4.51" "torch>=2.6" --extra-index-url https://download.pytorch.org/whl/cpu
```

## 2. Download the model

We use Qwen3-0.6B: small (~1.5 GB), ungated, modern architecture.

```bash
hf download Qwen/Qwen3-0.6B --local-dir ./qwen3-0.6b
ls -lh ./qwen3-0.6b
```

Expected output (sizes approximate):

```
config.json                     ~1 KB
generation_config.json          ~1 KB
merges.txt                      ~2 MB
model.safetensors               ~1.5 GB
tokenizer.json                  ~11 MB
tokenizer_config.json           ~10 KB
vocab.json                      ~3 MB
```

That's the whole model. One big tensor file; everything else is metadata. There is no code in here — the *architecture* code lives in the serving library (transformers, vLLM…); the files only tell it which architecture and supply the numbers.

## 3. Read the blueprint: `config.json`

```bash
python3 -c "import json; c=json.load(open('qwen3-0.6b/config.json')); [print(f'{k}: {c[k]}') for k in ('architectures','num_hidden_layers','hidden_size','num_attention_heads','num_key_value_heads','head_dim','vocab_size','max_position_embeddings','torch_dtype')]"
```

Expected output:

```
architectures: ['Qwen3ForCausalLM']
num_hidden_layers: 28
hidden_size: 1024
num_attention_heads: 16
num_key_value_heads: 8
head_dim: 128
vocab_size: 151936
max_position_embeddings: 40960
torch_dtype: bfloat16
```

These few numbers determine everything we compute next.

## 4. Predict the weight memory

Formula: **weight memory ≈ parameters × bytes per parameter**. The name says 0.6B parameters; BF16 means 2 bytes each:

```
0.6e9 params × 2 bytes ≈ 1.2 GB
```

Check against the actual tensors — this reads only the safetensors *header*, so it's instant:

```bash
python3 << 'EOF'
from safetensors import safe_open
total = 0
with safe_open("qwen3-0.6b/model.safetensors", framework="pt") as f:
    for name in f.keys():
        shape = f.get_slice(name).get_shape()
        n = 1
        for d in shape: n *= d
        total += n
print(f"parameters: {total/1e9:.3f} B")
print(f"at BF16 (2 bytes): {total*2/1e9:.2f} GB")
EOF
```

Expected output:

```
parameters: 0.752 B
at BF16 (2 bytes): 1.50 GB
```

Two lessons: the marketing name (0.6B) undercounts (embeddings for a 151k vocabulary are a big slice of a small model), and file size ≈ params × 2 held. Your prediction was within ~25%; on big models (8B, 70B) the formula lands within a few percent because embeddings stop mattering.

## 5. Predict the KV cache appetite

Formula: **KV bytes/token = 2 × layers × kv_heads × head_dim × 2 bytes** (first 2 = keys and values). With the config values:

```
2 × 28 × 8 × 128 × 2 = 114,688 bytes ≈ 112 KB per token
```

So one request at the full 40,960-token context: `40960 × 112 KB ≈ 4.7 GB` — **three times the weights**. Note the config's trick: 16 attention heads but only 8 KV heads. That's grouped-query attention (GQA) — the model was *designed* to halve its serving cache. Architecture choices are serving choices.

## 6. Tokens: what the model actually sees

```bash
python3 << 'EOF'
from transformers import AutoTokenizer
tok = AutoTokenizer.from_pretrained("./qwen3-0.6b")
text = "Serving models is an engineering discipline."
ids = tok(text).input_ids
print(f"{len(text)} chars -> {len(ids)} tokens")
print([tok.decode([i]) for i in ids])
msgs = [{"role": "user", "content": "hi"}]
print("--- what the chat template really sends: ---")
print(tok.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True))
EOF
```

Expected: the sentence becomes ~8 tokens (whole words with leading spaces, mostly), and the chat template output reveals the special tokens (`<|im_start|>` …) wrapped around your "hi" — the invisible overhead every request pays, and the shared prefix that cache layers will later exploit.

## 7. The payoff: size a model you never downloaded

Llama-3.1-70B-Instruct: 70e9 × 2 bytes = **140 GB** of weights — doesn't fit any single GPU (H100 = 80 GB); it *must* be split (part 4 of the [roadmap](../roadmap.md)). Its config (80 layers, 8 KV heads, head_dim 128) gives 2×80×8×128×2 ≈ **320 KB/token** — a single 128k-context request costs ~41 GB of KV. You now see, on paper, why long context is expensive and why the KV cache dominates serving economics.

**Done.** You can size any model from its config. Next: [02 — serve a model bare](02-serve-a-model-bare.md), where this model meets its first concurrent users.

---
*Validated against: (versions to be filled by the non-author validator — record `pip freeze | grep -E "transformers|torch|safetensors|huggingface"` here).*
