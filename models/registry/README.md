# Model Registry Directory

This directory is reserved for future use as a per-model documentation directory structure.

## Current Structure

The declarative model registry is maintained in `models/registry.yaml`, with individual model documentation stored in `models/<model-name>/`:

```
models/
├── registry.yaml              # Central declarative registry
├── <model-name>/             # Per-model documentation
│   ├── model.md              # Model datasheet
│   ├── budget.md             # VRAM budget calculation
│   └── eval-report.md        # Quality validation results
└── registry/                 # Reserved for future use
```

## Future Use Cases

This directory may be used for:
- Centralized model metadata archives
- Historical model version tracking
- Cross-model comparison reports
- Automated registry generation outputs

## Current Status

**Empty** - Not currently used by the deployment pipeline.
