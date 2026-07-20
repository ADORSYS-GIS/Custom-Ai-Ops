# Migration map

This PR installs structure only; it moves nothing. Each pre-existing document below gets consolidated into its target home in small follow-up PRs (one area per PR, so review stays meaningful). Delete this file when the table is empty.

| Current location | Target | Notes |
|---|---|---|
| `impl.md` (root) | `docs/explanation/` + `docs/how-to/` | Split: rationale vs steps |
| `namage.md` (root) | `docs/explanation/` or delete | Review content; name is a typo artifact |
| `solve.md` (root) | `docs/runbooks/` or `docs/explanation/` | Problem/solution notes → classify per item |
| `docs/env.md` | `docs/reference/` | Environment facts → reference tables |
| `docs/integration-report.md` (1.3k lines) | split across `docs/explanation/` + `docs/reference/` + `docs/adr/` | Monolith; extract any decisions into ADRs |
| `docs/external-tools.md` (1.5k lines) | `docs/reference/` | Tool inventory → reference |
| `docs/explain/kv-cache.md`, `bible-kv-cache.md` | `docs/explanation/kv-cache.md` | Merge two overlapping docs into one |
| `docs/explain/vllm+lmcache*.md` (3 files) | `docs/explanation/` + `docs/how-to/` | Theory → one explanation; practice → how-to guide |
| `docs/explain/gpu.md`, `onnx.md`, `llm-d.md` | `docs/explanation/` | Rename move, review for quadrant purity |
| `docs/explain/` (directory) | remove after empty | Superseded by `docs/explanation/` |
| `docs/architecture/00–07-*.md` | keep as `docs/architecture/` for now | Coherent series; long-term candidates for `explanation/` |
| `docs/adr/0001–0003` | stay | Already conform |
| `docs/runbooks/*` | stay | Already conform |

Follow-up PR order (suggested): (1) root loose files, (2) explain→explanation merge, (3) the two monolith reports.
