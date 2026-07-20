# Contributing

## Code

Charts, tools, and manifests follow the existing conventions; commits are linted (`tools/commit-lint.sh`).

## Documentation — read this even if "just" writing docs

All knowledge in this repo follows the structure defined in [`docs/README.md`](docs/README.md) ([Diátaxis](https://diataxis.fr/) + [ADRs](https://adr.github.io/) + immutable benchmark reports). The short version:

1. **No work is done until its documentation is merged** — labs, deployments, incidents, decisions.
2. **Classify before writing:** tutorial, how-to, reference, or explanation — one quadrant per document. Unsure? Ask in the PR.
3. **Decisions get ADRs** (`docs/adr/template.md`); **numbers get benchmark reports** (`docs/benchmarks/template.md`); reports are immutable.
4. **New terms go in** [`docs/reference/glossary.md`](docs/reference/glossary.md).
5. Docs go through PR review like code. Tutorials must be executed end-to-end by a non-author before merge.
