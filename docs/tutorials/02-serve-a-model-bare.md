# 02 — Serve a model bare

You will build the most naive possible model server — load weights, run `generate()` behind HTTP — then watch it collapse under four concurrent users, with numbers. Every runtime feature we study later is a fix for something you measure today.

Prereqs: [tutorial 01](01-anatomy-of-a-model.md) completed (model downloaded, venv active). Works on CPU; an NVIDIA GPU (any, ≥4 GB) makes it more realistic. Why it fails the way it does: [explanation](../explanation/how-a-transformer-serves-a-request.md).

## 1. Install the server deps

```bash
cd ~/labs/01-anatomy && source .venv/bin/activate
pip install "fastapi>=0.115" "uvicorn>=0.34" "httpx>=0.28" "accelerate>=1.0"
mkdir -p ~/labs/02-bare && cd ~/labs/02-bare
```

## 2. The naive server

Save as `server.py`:

```python
import threading, time
from fastapi import FastAPI
from fastapi.responses import StreamingResponse
from pydantic import BaseModel
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, TextIteratorStreamer

MODEL_DIR = "../01-anatomy/qwen3-0.6b"
device = "cuda" if torch.cuda.is_available() else "cpu"
dtype = torch.bfloat16 if device == "cuda" else torch.float32

print(f"loading weights onto {device}...")
t0 = time.time()
tok = AutoTokenizer.from_pretrained(MODEL_DIR)
model = AutoModelForCausalLM.from_pretrained(MODEL_DIR, torch_dtype=dtype).to(device).eval()
print(f"loaded in {time.time()-t0:.1f}s")

app = FastAPI()
lock = threading.Lock()          # one request at a time — the naive "scheduler"

class Req(BaseModel):
    prompt: str
    max_new_tokens: int = 128

@app.post("/generate")
def generate(req: Req):
    msgs = [{"role": "user", "content": req.prompt}]
    ids = tok.apply_chat_template(
        msgs, add_generation_prompt=True, enable_thinking=False,
        return_tensors="pt",
    ).to(device)
    streamer = TextIteratorStreamer(tok, skip_prompt=True, skip_special_tokens=True)

    def run():
        with lock, torch.no_grad():
            model.generate(ids, max_new_tokens=req.max_new_tokens,
                           do_sample=False, streamer=streamer)

    threading.Thread(target=run).start()
    return StreamingResponse((piece for piece in streamer), media_type="text/plain")
```

That lock is not a bug — it's honesty. Without it, concurrent `generate()` calls interleave on the GPU and slow each other anyway; the lock just makes the queueing visible. This is exactly what "no batching, no scheduling" means.

Run it:

```bash
uvicorn server:app --port 8000
```

Smoke test from a second terminal — you should see tokens trickle in:

```bash
curl -N -X POST localhost:8000/generate -H 'content-type: application/json' \
  -d '{"prompt": "Say hello in five words."}'
```

## 3. The measurement client

Save as `client.py`. It measures, client-side with streaming — the way our [methodology](../benchmarks/methodology.md) demands — TTFT and total latency per request, at increasing concurrency:

```python
import asyncio, sys, time
import httpx

URL = "http://localhost:8000/generate"
PAYLOAD = {"prompt": "Explain what a KV cache is in about 100 words.", "max_new_tokens": 120}

async def one(client):
    t0 = time.perf_counter()
    ttft = None
    async with client.stream("POST", URL, json=PAYLOAD) as r:
        async for _ in r.aiter_bytes():
            if ttft is None:
                ttft = time.perf_counter() - t0
    return ttft, time.perf_counter() - t0

async def bench(conc):
    async with httpx.AsyncClient(timeout=600) as client:
        await one(client)                                   # warmup, discarded
        results = await asyncio.gather(*[one(client) for _ in range(conc)])
    ttfts = sorted(r[0] for r in results)
    e2es = sorted(r[1] for r in results)
    print(f"concurrency {conc:>2}: "
          f"TTFT p50={ttfts[len(ttfts)//2]:6.2f}s  max={ttfts[-1]:6.2f}s | "
          f"E2E p50={e2es[len(e2es)//2]:6.2f}s  max={e2es[-1]:6.2f}s")

for c in [int(x) for x in (sys.argv[1:] or ["1", "2", "4", "8"])]:
    asyncio.run(bench(c))
```

Run the sweep:

```bash
python3 client.py 1 2 4 8
```

## 4. What you will see

Illustrative shape (a single RTX-class GPU; your absolute numbers will differ, the *shape* will not):

```
concurrency  1: TTFT p50=  0.15s  max=  0.15s | E2E p50=  2.1s  max=  2.1s
concurrency  2: TTFT p50=  1.2s   max=  2.3s  | E2E p50=  3.3s  max=  4.4s
concurrency  4: TTFT p50=  3.4s   max=  6.8s  | E2E p50=  5.5s  max=  8.9s
concurrency  8: TTFT p50=  7.9s   max= 15.7s  | E2E p50= 10.1s  max= 17.8s
```

Read it against the three predicted failures:

1. **TTFT grows ~linearly with concurrency** — requests are served strictly one after another; the last user at concurrency 8 waits through seven strangers' full generations. Meanwhile run `watch -n1 nvidia-smi`: utilization spikes and idles; the GPU is bored while users queue.
2. **Worst-case memory behavior** — raise `max_new_tokens` to 2048 and concurrency to 16 on a small GPU and the process eventually dies with CUDA OOM: memory is allocated naively per request, nothing reclaims or pages it.
3. **No fairness** — change one request's `max_new_tokens` to 1024 and watch every request behind it absorb its full duration. No preemption, no interleaving.

## 5. Freeze the baseline

These numbers are the denominator for every speedup we will ever claim. Record your run properly: create `experiments/EXP-<date>-bare-baseline/` per the [experiment protocol](../../experiments/README.md) (env manifest, both scripts, raw output) and file the dated report in [`../benchmarks/`](../benchmarks/README.md).

**Done.** You have built inference serving's "before" picture. Part 2 of the [roadmap](../roadmap.md) replaces this file's 40 lines with a runtime — and now you know exactly which three problems it must solve, and by how much.

---
*Validated against: (to be filled by the non-author validator — `pip freeze | grep -E "transformers|torch|fastapi|uvicorn|httpx"`, GPU model, driver).*
