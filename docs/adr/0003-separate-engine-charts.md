# ADR-0003: Unified engine Helm chart

## Status: Accepted

## Context

Previously, separate Helm charts were maintained per engine type (e.g. `model-serving-vllm`, `model-serving-onnxruntime-genai`). With the consolidation to vLLM as the sole inference engine, maintaining multiple engine-specific charts is no longer justified.

## Decision

Consolidate into a single Helm chart, `model-serving-engine`, which serves all vLLM-compatible formats (Safetensors, AWQ, GPTQ). The deprecated `model-serving-vllm` and deleted `model-serving-onnxruntime-genai` charts are replaced by `model-serving-engine`.

All charts depend on `bjw-template` library chart for common StatefulSet/PVC/probe patterns.

## Consequences

- A single chart focuses on vLLM's configuration.
- Shared patterns are maintained once in `bjw-template`.
- Adding a new vLLM-compatible format only requires updating the engine-selector decision tree and `model-serving-engine` values.