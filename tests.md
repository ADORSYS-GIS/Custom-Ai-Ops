# Certification Suite — Rigorous Validation of the Model Serving System

## Objective

Define the complete set of tests that the system must pass before being approved for production. Each test has a precise objective, an execution method, a strict pass/fail criterion (binary, unambiguous), and explains what it reinforces in the system. No test is cosmetic: every failure must block promotion to production.

Guiding principle: **a system is never "roughly ready"**. Each test below has a GO / NO-GO verdict. The system is approved only when 100% of the blocking tests pass.

---

## Category 1 — Packaging and Model Integrity Tests

### T1.1 — Cryptographic Integrity of Weights
**Objective**: guarantee that deployed weights are exactly those validated, without corruption or substitution.
**Method**: SHA-256 checksum of the weight file at promotion time into the registry, automatic comparison of the checksum at load time by the serving engine.
**Pass criterion**: 100% checksum match, otherwise immediate blocking failure of pod startup.
**Reinforces**: eliminates any possibility of silent drift between "what was validated" and "what actually runs".

### T1.2 — Format/Engine Consistency
**Objective**: verify that the selected engine actually matches the model's declared format.
**Method**: automated test running `engine-selector` on each registry entry and comparing the proposed engine to the engine actually configured in the chart.
**Pass criterion**: 0 discrepancies detected.
**Reinforces**: prevents a GGUF model from accidentally being configured on a vLLM chart (total incompatibility), the most frequent manual configuration error.

### T1.3 — Memory Budget Validation Before Deployment
**Objective**: guarantee that no model is deployed without mathematical proof that it fits in the target VRAM.
**Method**: `vram-budget-calc` executed in CI, calculation = Total_VRAM × 0.85 − weights − overhead, must be strictly positive and cover the maximum declared context.
**Pass criterion**: available KV-cache budget ≥ requirement for the declared `maxOutputTokens` and `contextLength` in the gateway.
**Reinforces**: eliminates the class of OOM incidents in production, the most frequent and most avoidable.

### T1.4 — Hardware Incompatibility Blocked
**Objective**: prevent deployment of checkpoints incompatible with the target GPU architecture (e.g., FP8 on a GPU without native support).
**Method**: hard-coded rule tested by case: attempting to deploy an FP8 model on an Ampere nodeSelector must be rejected by CI.
**Pass criterion**: 100% systematic rejection of tested cases (matrix of format × GPU architecture combinations).
**Reinforces**: transforms tribal knowledge ("we know not to do that") into an automatic safeguard.

---

## Category 2 — Declarative Infrastructure Tests (Helm/Kustomize/Git)

### T2.1 — Strict Lint on 100% of Charts
**Method**: `helm lint --strict` executed on each chart in the repository at every commit, not just modified charts.
**Pass criterion**: zero warnings, zero errors, across all charts (including shared dependent charts like `bjw-template`).
**Reinforces**: detects regressions in shared dependencies that a test targeting only the modified chart would miss.

### T2.2 — Complete Dry-Run Rendering
**Method**: `helm template --dry-run` on each chart × environment combination (dev/staging/prod), validation of generated YAML against the Kubernetes schema (`kubeconform` or equivalent).
**Pass criterion**: 100% of generated manifests are syntactically valid and conform to the targeted Kubernetes API schema.
**Reinforces**: prevents a broken manifest from reaching ArgoCD, where the failure would be detected later and more expensively.

### T2.3 — Declarative Registry Consistency
**Method**: test verifying that each entry in `models/registry.yaml` has: a corresponding chart, a corresponding gateway entry, a proven `budget.md` file, a valid status (LIVE/STAGED/STANDBY).
**Pass criterion**: 0 orphan entries in either direction (model without chart, or chart without registry entry).
**Reinforces**: prevents drift between documentation and deployment reality, a survival condition for a system operated for years.

### T2.4 — Helm Rendering Idempotency
**Method**: run `helm template` twice on the same commit and compare outputs.
**Pass criterion**: strictly identical bit-for-bit output between the two runs.
**Reinforces**: guarantees reproducibility — a non-idempotent system cannot be reliably audited or reproduced.

### T2.5 — Plaintext Secret Detection
**Method**: automated scan (`gitleaks` or equivalent) on each commit of the charts repo and values repo.
**Pass criterion**: 0 detections of API keys, tokens, or plaintext passwords.
**Reinforces**: prevents the most damaging and most frequent class of failure in GitOps repositories.

---

## Category 3 — ArgoCD Synchronization Tests

### T3.1 — Sync Wave Convergence
**Objective**: validate that the deployment order (-3 → -2 → -1 → 0 → 1 → 2+) is strictly respected and that each wave reaches "Healthy" state before the next starts.
**Method**: complete deployment on an ephemeral test cluster, capture state transition timestamps of each wave, verify chronological order.
**Pass criterion**: no resource from wave N+1 becomes "Progressing" before all resources from wave N are "Healthy".
**Reinforces**: prevents incidents of type "workload starts before storage is ready", already identified as costly in the existing pattern.

### T3.2 — Self-Healing After Manual Drift
**Objective**: verify that ArgoCD automatically detects and corrects any manual modification (`kubectl edit`) of a managed resource.
**Method**: manually modify a deployed resource (e.g., change replica count), measure delay before automatic correction.
**Pass criterion**: automatic correction in less than 3 minutes (default reconciliation interval), without human intervention.
**Reinforces**: guarantees that Git remains truly the single source of truth, not just in theory.

### T3.3 — Custom Health Check for ML CRDs
**Method**: deploy an `InferenceService` (KServe) or equivalent and verify that the custom `health.lua` correctly reports "Healthy" once the model is actually ready (not just the pod started).
**Pass criterion**: "Progressing" → "Healthy" transition in ArgoCD occurs at the same time as the container's `/health` returns 200, within 5 seconds.
**Reinforces**: eliminates the blind spot where ArgoCD displays a false indefinite "Progressing" on non-standard CRDs.

### T3.4 — One-Click Rollback, Verified
**Method**: deploy an intentionally broken version (e.g., bad image tag), trigger an ArgoCD rollback to the previous sync, measure time to return to healthy state.
**Pass criterion**: return to "Healthy" state in less than 5 minutes after rollback trigger.
**Reinforces**: validates that the last-resort safety net actually works, not just in theory on documentation.

### T3.5 — App-of-Apps Coherent at Scale
**Method**: with N managed applications (≥ 20, as in the current pattern at ~21 Applications), verify that a root-level change propagates correctly to all child applications without sync wave collision.
**Pass criterion**: 100% of child applications reach "Synced + Healthy" after propagation, without dependency deadlock.
**Reinforces**: validates the scalability of the GitOps structure itself as the number of models/services increases.

---

## Category 4 — Model Loading and Startup Tests

### T4.1 — Cold Start Measured and Bounded
**Method**: cold-start a pod (no cache), time between pod creation and first `200 OK` on `/health`.
**Pass criterion**: cold start time documented and below the threshold declared in the model datasheet (e.g., startup probe `failureThreshold` budget), with 20% margin.
**Reinforces**: prevents production surprises where actual cold start exceeds what probes tolerate, causing crash loops.

### T4.2 — Differentiated Probes Correctly Configured
**Method**: verify that the liveness probe never depends on model availability (otherwise long loading triggers a kill loop), and that the readiness probe actually checks a functional test inference.
**Pass criterion**: liveness stays "OK" throughout loading; readiness switches to "OK" only after a successful validation inference.
**Reinforces**: eliminates the class of crash-loop incidents caused by a poorly designed probe, already anticipated in the pattern (tcpSocket for liveness, httpGet long timeout for startup).

### T4.3 — Concurrent Loading Test (Shared RWX PVC)
**Method**: simultaneously start multiple replicas pointing to the same RWX weight volume, verify absence of corruption or blocking contention.
**Pass criterion**: all replicas start successfully in parallel, Nth replica startup time not significantly degraded compared to the first.
**Reinforces**: validates that the shared volume pattern scales beyond a single replica per model.

---

## Category 5 — Serving API Functional Tests

### T5.1 — Strict OpenAI-Compatible Conformance
**Method**: contract test suite (schema validation) on `/v1/chat/completions` covering all engines (llama.cpp, vLLM, ONNX Runtime GenAI, Triton), comparing response structure to the official OpenAI schema.
**Pass criterion**: 100% schema conformance, regardless of the underlying engine.
**Reinforces**: guarantees real engine interchangeability from the client's perspective, the fundamental principle of the architecture (section 4.1 of the architecture document).

### T5.2 — Robust SSE Streaming
**Method**: long connection test with simulated mid-stream network interruption, verification of resume or clean failure behavior on the client side.
**Pass criterion**: no silently truncated token without explicit error signal to the client; no connection leak on the server side after interruption.
**Reinforces**: avoids silently corrupted responses, particularly critical for LLMs in production.

### T5.3 — Native Authentication Verified
**Method**: request without API key (expected 401), request with invalid key (expected 401), request with valid key (expected 200), on each backend.
**Pass criterion**: exact 401/401/200 behavior on 100% of backends, including those using a sidecar (e.g., Caddy for vLLM).
**Reinforces**: non-negotiable baseline security before any public exposure.

### T5.4 — Real Maximum Context Test
**Method**: send a request reaching exactly the `contextLength` declared in the gateway, then a request exceeding it by one token.
**Pass criterion**: the boundary request succeeds; the exceeding request fails with an explicit error (not a crash, not silent truncation).
**Reinforces**: eliminates the gap between advertised capacity and actual capacity, a source of user confusion and incidents.

### T5.5 — Multi-Model Consistency Test (Model Registry)
**Method**: for each declared active model, execute a real completion request and verify a coherent response (non-empty, non-NaN, correct format).
**Pass criterion**: 100% of models declared "LIVE" respond correctly; any failure blocks deployment of the entire registry.
**Reinforces**: prevents a broken model from remaining invisible because attention is focused only on the last modified model.

---

## Category 6 — GPU Robustness and Scheduling Tests

### T6.1 — Correct Placement per nodeSelector/taints
**Method**: deploy each model chart and verify that the pod is actually placed on the expected node pool (verification of the real node label).
**Pass criterion**: 100% match between declared pool and actual execution pool.
**Reinforces**: prevents an expensive H100 GPU model from being placed on an L4 node by configuration error, or conversely a lightweight model wasting a high-end GPU.

### T6.2 — Behavior Under GPU Scarcity (Kueue/Volcano)
**Method**: simulate GPU demand exceeding available capacity, verify that higher-priority jobs pass before lower-priority jobs (correct preemption).
**Pass criterion**: passage order strictly conform to declared priorities, no low-priority job indefinitely blocks a high-priority job (no starvation).
**Reinforces**: validates that scheduling remains fair and predictable even at maximum load, an essential condition at scale.

### T6.3 — Failing GPU Node Detection and Eviction
**Method**: simulate an NVIDIA Xid error (via fault injection or dedicated test environment), verify detection by the GPU Operator and automatic cordon/drain of the node.
**Pass criterion**: node marked "unschedulable" in less than 2 minutes after error detection, existing pods migrated without loss of in-flight requests (if possible) or with clean error signaled to the client.
**Reinforces**: transforms a silent hardware failure into a detected and automatically managed incident.

### T6.4 — Memory Fragmentation Under Sustained Load
**Method**: sustained load test over several hours with variable batch sizes, monitoring real vs theoretical free GPU memory.
**Pass criterion**: no monotonic growth of used memory beyond what traffic explains (sign of leak/fragmentation) over a minimum 4-hour window.
**Reinforces**: detects memory leaks before they cause OOM in production after several days of uptime, a classic scenario not covered by short tests.

### T6.5 — MIG/Time-Slicing Validation
**Method**: on nodes configured for GPU partitioning, deploy multiple pods on the same physical GPU and verify effective isolation (memory and/or performance).
**Pass criterion**: one pod cannot observe or impact another pod's memory on the same card beyond the expected and documented performance degradation for the chosen mode.
**Reinforces**: validates that a misconfigured partition does not become an isolation flaw or a hidden bottleneck.

---

## Category 7 — Load and Performance Tests

### T7.1 — Nominal Load Test
**Method**: k6/Locust simulating average expected traffic, measure p50/p90/p99 latency, TTFT and TPOT for LLMs.
**Pass criterion**: all percentiles within SLAs declared in the model datasheet, no 5xx errors during the test.
**Reinforces**: establishes the baseline reference to detect any future regression.

### T7.2 — Peak Test (Stress Test)
**Method**: ramp up to 3x historical peak traffic, observe degradation behavior.
**Pass criterion**: graceful degradation (latency increases, but no cascading errors or crash); autoscaling (HPA/KEDA) reacts within the expected window (e.g., < 60 seconds to trigger scale-out).
**Reinforces**: validates that the system bends without breaking, the fundamental principle of robustness at scale.

### T7.3 — Sustainability Test (Endurance)
**Method**: sustained nominal traffic over 24 to 72 continuous hours.
**Pass criterion**: no progressive degradation of latency or error rate over the duration, stable memory (linked to T6.4).
**Reinforces**: detects problems that only appear after long uptime (leaks, fragmentation, connection accumulation).

### T7.4 — Burst Cold Start Test
**Method**: simultaneously trigger cold start of N replicas (sudden massive scale-out), measure time before all become "Ready".
**Pass criterion**: convergence time of the entire group documented and acceptable for the peak SLA, without excessive contention on the image registry or shared weight storage.
**Reinforces**: validates actual behavior during a sudden and unforeseen peak, the most critical scenario for serving millions of users.

### T7.5 — Scale-to-Zero and Wake-Up Validation
**Method**: for low-traffic models configured for scale-to-zero (KEDA), measure delay between first request and effective response after wake-up.
**Pass criterion**: wake-up delay documented and conforming to the declared SLA for this model (different and more tolerant than always-on models).
**Reinforces**: validates that cost optimization does not break the user experience beyond what is acceptable.

---

## Category 8 — Resilience and Chaos Engineering Tests

### T8.1 — Random Pod Kill (Basic Chaos)
**Method**: forced and random deletion of serving pods during active traffic (Chaos Mesh or Litmus type).
**Pass criterion**: no in-flight request silently lost (clean error returned to client if interrupted), new pod operational and absorbing traffic in less than the documented cold start time.
**Reinforces**: validates basic resilience without waiting for a real incident to discover it.

### T8.2 — Simulated Zone/Region Failure
**Method**: artificially cut access to an entire worker cluster, verify automatic traffic failover by the gateway to a fallback backend (other region, or SaaS fallback).
**Pass criterion**: effective failover in less than the delay announced in the continuity SLA, minimized and measured request loss.
**Reinforces**: validates the multi-region/anti-lock-in strategy defined in the architecture, which otherwise would remain a theoretical intention never verified.

### T8.3 — Model Registry Degradation (MLflow Unavailable)
**Method**: cut access to the model registry during a deployment operation.
**Pass criterion**: models already in production continue serving without interruption (the registry is not a single point of failure for the runtime, only for new promotions).
**Reinforces**: validates the correct separation between control plane (registry, CI/CD) and data plane (actual serving).

### T8.4 — Gateway Failure (Envoy AI Gateway)
**Method**: simulate total gateway unavailability.
**Pass criterion**: defined and documented behavior (e.g., direct DNS fallback to a secondary backend, or controlled degradation with clear error message) rather than a total silent outage.
**Reinforces**: the gateway being the single entry point of the entire system, this test reveals whether it constitutes an unmanaged SPOF (single point of failure).

### T8.5 — Data Corruption in Transit
**Method**: inject malformed packets or artificial network latency (Toxiproxy) between the gateway and backends.
**Pass criterion**: no silently corrupted response delivered to the client; timeout and retry applied correctly per the declared configuration.
**Reinforces**: validates internal network robustness, often neglected because only tested in ideal laboratory conditions.

### T8.6 — Simulated Data Drift Test
**Method**: artificially inject a distribution change in test traffic (e.g., out-of-domain requests), verify detection by the drift monitoring tool (Evidently AI).
**Pass criterion**: drift alert triggered within the expected detection window (e.g., < 1 hour), with effective triggering of the application-level circuit breaker if configured.
**Reinforces**: validates that silent quality degradation (the most dangerous blind spot because invisible in infrastructure metrics) is actually detected, not just assumed to be.

---

## Category 9 — Security Tests

### T9.1 — Image Vulnerability Scan
**Method**: Harbor/Trivy scan of each image before promotion.
**Pass criterion**: zero unpatched CRITICAL vulnerabilities; HIGH vulnerabilities documented and explicitly accepted if not immediately fixable.
**Reinforces**: prevents a known vulnerable CUDA/Python dependency from reaching production.

### T9.2 — Network Isolation (NetworkPolicy)
**Method**: verify that a serving pod cannot initiate unauthorized outbound connections (e.g., directly to the Internet, outside what is strictly necessary).
**Pass criterion**: any connection attempt not listed in the egress policy is blocked and logged.
**Reinforces**: limits the attack surface in case of serving container compromise (data or model weight exfiltration).

### T9.3 — Prompt Injection Test (LLM)
**Method**: suite of known adversarial prompts testing the model/application layer's resistance to system instruction extraction or guardrail bypass.
**Pass criterion**: behavior conforming to the defined security policy (no system prompt leakage, no out-of-policy content generation), with success rate measured and tracked over time (no silent regression at each model change).
**Reinforces**: LLM-specific security, absent from classical infrastructure tests.

### T9.4 — Image Signature and Provenance
**Method**: cosign verification of each image signature before `argocd-image-updater` proposes an update.
**Pass criterion**: automatic rejection of any unsigned image or signed by an unauthorized identity.
**Reinforces**: prevents injection of a malicious image into the automated deployment chain (supply chain attack).

### T9.5 — Secret Rotation and Expiration
**Method**: verify that API keys and secrets managed via External Secrets Operator are correctly refreshed after rotation on the AWS Secrets Manager/Vault side, without manual restart required.
**Pass criterion**: new secret value active in the system in less than the declared sync delay, old value revoked without service interruption.
**Reinforces**: validates that a security policy (regular rotation) does not break availability, a frequent reason teams disable rotation in practice.

---

## Category 10 — Cost and Economic Governance Tests

### T10.1 — Correct Cost Metric Emission
**Method**: for each completion request, verify that the pricing CEL rule emits a non-zero value consistent with the volume of tokens actually consumed.
**Pass criterion**: difference between calculated cost and expected cost (manual reference calculation) less than 1%.
**Reinforces**: guarantees that the cost-recovery pricing model (ADR-0028 in the existing pattern) reflects reality, a condition of the system's economic sustainability.

### T10.2 — Cost Drift Alert
**Method**: simulate abnormally high traffic on an expensive model, verify triggering of a budget alert before cost becomes uncontrolled.
**Pass criterion**: alert triggered before cumulative cost exceeds a defined threshold (e.g., 150% of planned daily budget).
**Reinforces**: protects against uncontrolled billing incidents, particularly critical at the scale of millions of users.

---

## Category 11 — End-to-End Tests (Final Synthesis)

### T11.1 — Complete User Journey, Multi-Engine
**Method**: scenario simulating a real user sending successive requests routed to models of different engines (llama.cpp, vLLM, ONNX), verifying experience consistency (comparable perceived latency, identical response format).
**Pass criterion**: no perceptible difference on the client side between engines, per the abstraction principle in section 4 of the architecture.
**Reinforces**: final validation that the modularity promise is kept in practice, not just in design theory.

### T11.2 — Complete Reconstruction from Scratch (Total Disaster Recovery)
**Method**: on an empty cluster, execute only `argocd app sync` from the root Git repo, without any manual intervention, and measure time until the entire system (all models, gateway, observability) is "Healthy".
**Pass criterion**: complete reconstruction succeeds without manual intervention, within a documented and acceptable delay (this delay becomes the system's official RTO — Recovery Time Objective).
**Reinforces**: this is the ultimate test of the GitOps philosophy — if this test fails, Git is not truly the source of truth, regardless of what the documentation says.

### T11.3 — Complete Traceability Audit
**Method**: for a simulated incident (automatically triggered rollback), verify that it is possible to fully reconstruct the timeline (which commit, which test failed, which corrective action, at what time) solely from Git and logs, without tribal knowledge.
**Pass criterion**: complete and unambiguous reconstruction of the timeline by a person who did not participate in the incident.
**Reinforces**: survival condition of the system over several years with team turnover — a system that can only be understood by its original creators is not durable.

---

## Summary Table — Global Approval Criterion (GO/NO-GO)

| Category | Number of Tests | Blocking for Production |
|---|---|---|
| 1. Packaging and model integrity | 4 | Yes — without exception |
| 2. Declarative infrastructure | 5 | Yes — without exception |
| 3. ArgoCD synchronization | 5 | Yes — without exception |
| 4. Loading and startup | 3 | Yes — without exception |
| 5. Serving API | 5 | Yes — without exception |
| 6. GPU robustness and scheduling | 5 | Yes — without exception |
| 7. Load and performance | 5 | Yes for T7.1/T7.2; T7.3/T7.4/T7.5 required before major ramp-up |
| 8. Resilience and chaos engineering | 6 | Yes for T8.1/T8.2/T8.6; others required before broad public exposure |
| 9. Security | 5 | Yes — without exception |
| 10. Cost and governance | 2 | Yes before opening to billed traffic |
| 11. End-to-end | 3 | Yes — without exception, conditions final approval |

**Final approval rule**: the system is certified ready for large-scale production only when all blocking tests above pass simultaneously on the same commit, in a single reproducible CI/CD pipeline run. Any exception must be documented as an explicit ADR with a committed remediation date — never as a silent omission.