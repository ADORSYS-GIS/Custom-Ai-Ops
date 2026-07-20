# ADR-0004: Adopt Diátaxis-based documentation structure

## Status

Accepted — 2026-07-20

## Context

This repository accumulated ~10,000 lines of documentation organically: loose root-level notes (`impl.md`, `namage.md`, `solve.md`), monolithic reports (`docs/integration-report.md`, `docs/external-tools.md`, `docs/env.md`), and an unclassified `docs/explain/` folder with overlapping documents (three variants of vLLM+LMCache alone). There is no defined home for benchmarks, no glossary, and no documented rule for what goes where. The team has been mandated to make this knowledge transferable to the rest of the company; findability and consistency are the blockers.

## Options considered

- **Keep organic growth** — zero effort now; cost compounds with every document and every new reader.
- **Single handbook document** — easy to start, but 10k lines already prove it doesn't scale, and mixed audiences (learner vs operator vs decision-reviewer) fight each other in one page.
- **Diátaxis + ADRs + immutable benchmark reports** — the [Diátaxis](https://diataxis.fr/) framework separates docs by reader need (tutorial / how-to / reference / explanation); [ADRs](https://adr.github.io/) capture decisions; dated benchmark reports form the evidence layer. Industry-standard, tool-agnostic, incremental to adopt.

## Decision

Adopt Diátaxis as the documentation structure, with three additions: `docs/adr/` (already present), `docs/benchmarks/` (immutable dated reports), and `docs/runbooks/` (already present). Enforce via the rules in [`docs/README.md`](../README.md), most importantly: *no work is done until its documentation is merged*, and *one quadrant per document*.

## Consequences

- Every existing document gets a target home; consolidation happens in follow-up PRs per [`docs/MIGRATION.md`](../MIGRATION.md) — this ADR does not move content.
- Authors must classify before writing; PR review enforces it.
- Benchmark claims become citable and immutable, which ADRs then rely on.
- Cost: slightly more ceremony per document; revisit if the structure demonstrably slows the team rather than the docs' readers.
