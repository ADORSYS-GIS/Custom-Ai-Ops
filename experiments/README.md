# Experiments

Raw, reproducible experiment assets. The **immutable report** distilled from an experiment lives in [`../docs/benchmarks/`](../docs/benchmarks/README.md); everything needed to re-run it lives here, in one directory per experiment. If it's not in the directory, it didn't happen.

## Layout

```
experiments/
├── workloads/                      # named workload profiles (dataset refs, generator configs, seeds)
│   └── chat-multiturn@v1/
└── EXP-2026-08-02-prefix-caching-ab/
    ├── HYPOTHESIS.md               # written BEFORE any run (see protocol)
    ├── manifest/
    │   ├── env.md                  # hardware, GPU/driver/CUDA, k8s + node info, image digests
    │   ├── engine-args.txt         # exact flags/env per engine instance, both arms
    │   └── *.yaml                  # k8s manifests / helm values / compose files used
    ├── scripts/
    │   └── run.sh                  # ONE command reproduces the whole run (incl. teardown)
    ├── results/
    │   ├── raw/                    # untouched tool output, one subdir per run: a1/ a2/ a3/ b1/…
    │   └── summary.csv             # consolidated table the report reads from
    └── analysis/
        ├── plots/                  # saturation curves, latency CDFs + the script that made them
        └── notes.md                # observations, anomalies, dead ends (dead ends are data)
```

## Protocol

1. **Hypothesis first.** `HYPOTHESIS.md` before any run: falsifiable and numeric ("enabling X improves p95 TTFT ≥30% on `chat-multiturn@v1` at 32 concurrent, because <mechanism>"). Copy it verbatim into the final report — no editing after the fact.
2. **Freeze the environment.** Pin digests, not tags; two arms differ by exactly one variable; fill `manifest/` completely (a report missing it cannot be cited).
3. **Run** per [`../docs/benchmarks/methodology.md`](../docs/benchmarks/methodology.md). `run.sh` is the only entry point — ad-hoc commands get folded into it and rerun.
4. **Capture server-side metrics** for the run window into `results/raw/`.
5. **Analyze**; every plot regenerable from `analysis/` scripts + `results/`.
6. **Report**: write the dated report in `docs/benchmarks/` with an explicit verdict — hypothesis confirmed / rejected / inconclusive. Rejected gets a "why we were wrong" paragraph; those are the highest-value documents in the repo.
7. **Review**: a non-author reviews method and spot-checks one rerun before the report PR merges.
8. **Cost line**: GPU-hours consumed recorded in the report; `run.sh` tears down rented capacity.

## Hygiene

- Raw JSON results are small — commit them. Beyond ~50 MB, park raw data in object storage and link it from the report; `summary.csv` + plots stay in git.
- Containers only; no bare-metal pip installs.
- After ~5 experiments by hand, automate the boilerplate (`tools/labctl` candidate) — not before feeling the friction.
