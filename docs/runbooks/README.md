# Runbooks

Operational procedures for a paged human at 03:00. Existing runbooks (`latency-spike.md`, `pod-crashloop.md`, `gpu-node-failure.md`) set the pattern.

Format per runbook: **Symptom** (alert/observation) → **Impact** → **Diagnosis** (ordered checks with commands) → **Mitigation** (fastest safe action first) → **Root-cause follow-up**.

Rules: copy-pasteable commands; no theory (link `../explanation/`); update the runbook in the same PR as any incident postmortem that invalidated it.
