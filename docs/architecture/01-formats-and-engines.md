# Formats and Engines

## Decision Tree

```
Is the model in ONNX format?
├── Yes → ONNX Runtime GenAI
└── No
    ├── Is the model in Safetensors/BF16/FP16?
    │   └── Yes → vLLM
    ├── Is the model in AWQ/GPTQ?
    │   └── Yes → vLLM (native support)
    └── Otherwise → Unsupported format (convert to ONNX or Safetensors first)
```

## Format-Engine Mapping

| Format | Engine | Chart | Confidence |
|--------|--------|-------|------------|
| ONNX | ONNX Runtime GenAI | model-serving-onnx-rust | 95% |
| Safetensors (BF16/FP16) | vLLM | model-serving-vllm | 96% |
| AWQ | vLLM | model-serving-vllm | 94% |
| GPTQ | vLLM | model-serving-vllm | 93% |

This decision tree is codified in `tools/engine-selector` to prevent knowledge drift.