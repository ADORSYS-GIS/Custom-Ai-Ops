# Architecture Decision Records

An ADR is a one-page, numbered, dated record of a significant technical decision: the context, the options considered, the decision, and its consequences. The format follows [Michael Nygard's original](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) via [adr.github.io](https://adr.github.io/).

Rules:

- **When required:** any choice of engine, framework, format, platform component, or process that a future engineer might reasonably ask "why?" about. If it was argued about, it gets an ADR.
- **Numbering:** sequential, zero-padded (`0005-...`), never reused.
- **Statuses:** `Proposed` → `Accepted` | `Rejected`; later `Superseded by ADR-NNNN` (never deleted, never edited into a different decision).
- **Evidence:** performance-motivated ADRs must cite reports in [`../benchmarks/`](../benchmarks/).
- Use [`template.md`](template.md).

Existing ADRs 0001–0003 predate this structure and already conform — good precedent.
