# <Technology or concept>

> One-paragraph abstract: what this is and the one thing to remember about it.

## The problem it solves

What breaks or costs money without it — concrete, failure mode named.

## How it works

Mechanism in our own words (if we can only paraphrase the docs, we don't understand it yet). Mermaid where a picture beats prose.

## Where it sits in the stack

Layer (see the [optimization surface](../roadmap.md)) and what it interacts with above/below.

## The configuration that matters

The 3–7 knobs worth knowing and the direction each trades (latency ↔ throughput ↔ memory ↔ quality). Exhaustive flag tables go to [`../reference/`](../reference/README.md), not here.

## When it pays / when it doesn't

Which [workload profiles](../benchmarks/methodology.md) it wins on; where it's neutral or harmful.

## Evidence

Links to `../benchmarks/` reports backing every claim above.

## Sources

External docs/papers/talks, with dates — this field rots fast.

---

*Comparison documents ("X vs Y for <context>") use this same file plus: a criteria-and-weights table agreed **before** testing, a qualitative matrix (note feature* maturity*, not just presence — headline features converged by 2026), and a per-workload recommendation with revisit triggers. The decision itself still goes to an ADR.*
