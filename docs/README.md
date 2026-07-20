# Documentation

This folder is the team's knowledge base for **inference operations** — the science of deploying models for inference, from the weights upward. It is structured so that anyone in the company can find what they need without knowing where to look first.

## Structure: Diátaxis

We follow [Diátaxis](https://diataxis.fr/), the de facto standard for technical documentation. Its core insight: documentation serves four distinct needs, and mixing them in one page is what makes docs feel bad. Every document lives in exactly one quadrant:

| Directory | Type | Job | Reader's question |
|---|---|---|---|
| [`tutorials/`](tutorials/) | Tutorial | Learning by doing, guaranteed success | "Teach me the basics" |
| [`how-to/`](how-to/) | How-to guide | Task recipe for someone who knows the basics | "How do I do X?" |
| [`reference/`](reference/) | Reference | Facts — exhaustive, dry, lookup-oriented | "What are the exact details?" |
| [`explanation/`](explanation/) | Explanation | Understanding, context, the *why* | "Why does it work this way?" |

Plus three first-class citizens Diátaxis doesn't name:

| Directory | Contains |
|---|---|
| [`adr/`](adr/) | Architecture Decision Records — one page per technology decision ([what's an ADR?](adr/README.md)) |
| [`benchmarks/`](benchmarks/) | Dated, immutable benchmark reports — the evidence layer under every ADR |
| [`runbooks/`](runbooks/) | Operational procedures: symptom → diagnosis → mitigation |

And two living pages:

- [`reference/glossary.md`](reference/glossary.md) — one-sentence definitions, linked from everywhere. Our shared vocabulary.
- [`watchlist.md`](watchlist.md) — dated triage of ecosystem news (vLLM, llm-d, LMCache, …). The field moves monthly; this is how we keep up.

## The rules

1. **A lab or piece of work is not done until its documentation is merged.** No exceptions; this is the mechanism that keeps this repo alive.
2. **One quadrant per document.** If a how-to starts explaining theory, extract the theory to `explanation/` and link it.
3. **Benchmark reports are immutable.** Never edited — superseded by a new dated report.
4. **Every technology decision gets an ADR.** If we argued about it in a meeting, it gets an ADR.
5. **New terms go in the glossary** the week they are first used.
6. **All docs go through PR review**, like code.

## Where do I start?

- New to inference serving entirely → `explanation/`, then `tutorials/`.
- Need to operate the platform → `how-to/` and `runbooks/`.
- Wondering why we chose something → `adr/`, then the `benchmarks/` reports it cites.

See [`MIGRATION.md`](MIGRATION.md) for how pre-existing documents map into this structure.
