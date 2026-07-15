# Architecture Overview

This platform serves ML models of multiple formats (Safetensors, AWQ, GPTQ) through a unified OpenAI-compatible API, using vLLM as the sole inference engine.

## Three-Plane Architecture

1. **Model Plane** — Model weights + format (Safetensors, AWQ, GPTQ)
2. **Engine Plane** — Runtime that executes a given format (vLLM)
3. **Exposure Plane** — OpenAI-compatible endpoint (uniform API regardless of engine)

## Key Decisions

- See [ADR index](../adr/) for architectural decision records
- See [01-formats-and-engines.md](01-formats-and-engines.md) for format-to-engine mapping
- See [04-gitops-deployment.md](04-gitops-deployment.md) for deployment chain