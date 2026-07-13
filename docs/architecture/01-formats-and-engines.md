# Formats and Engines

## Decision Tree

```
Is the model in Safetensors/BF16/FP16?
├── Yes → vLLM
└── No
    ├── Is the model in AWQ/GPTQ?
    │   └── Yes → vLLM (native support)
    └── Otherwise → Unsupported format (convert to Safetensors first)
```

## Format-Engine Mapping

| Format | Engine | Chart | Confidence |
|--------|--------|-------|------------|
| Safetensors (BF16/FP16) | vLLM | model-serving-engine | 96% |
| AWQ | vLLM | model-serving-engine | 94% |
| GPTQ | vLLM | model-serving-engine | 93% |

This decision tree is codified in `tools/engine-selector` to prevent knowledge drift.