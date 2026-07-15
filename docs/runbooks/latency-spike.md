# Runbook: Latency Spike / Failover Triggered

## Symptoms
- Alert: `ModelServingHighLatency` (p95 > 2s)
- Alert: `ModelServingCriticalLatency` (p99 > 5s)
- Alert: `VLLMKVCacheUsageHigh` / `VLLMKVCacheUsageCritical` (KV cache > 85% / 100%)
- Alert: `VLLMRequestsWaitingHigh` (queue depth > 10)
- Alert: `VLLMSwapOutBlocksDetected` (KV cache pages swapped to CPU)
- Alert: `LMCacheL1HitRateLow` / `LMCacheL2HitRateLow` / `LMCacheL3HitRateLow` (distributed cache tiers underperforming)
- Alert: `VLLMPrefillSkipRateLow` (prefill not skipped despite cache hit opportunity)
- Alert: `CacheRoutingHeaderAbsent` (`x-cache-affinity-key` header missing — cache-aware routing degraded)
- Alert: `SSMModelPagedAttentionMisconfigured` (SSM/Mamba model deployed with PagedAttention — anti-pattern, see Bible §14)

## Steps

1. **Check current latency**:
   ```bash
   kubectl get pods -n model-serving-prod -o wide
   kubectl logs -n model-serving-prod -l app.kubernetes.io/name=model-serving-engine | tail -50
   ```

2. **Identify bottleneck**:
   - GPU utilisation: `nvidia-smi` on GPU nodes
   - **KV cache usage**: `kubectl logs` vLLM pod → check `gpu_cache_usage_perc`
   - **Request queue depth**: `kubectl logs` vLLM pod → check `num_requests_waiting`
   - **Prefix cache hit rate**: `kubectl logs` vLLM pod → check prefix cache metrics
   - **KV cache swap-out**: `kubectl logs` vLLM pod → check `swap_out_blocks`
   - Network: Check for pod network latency in chaos test results

3. **Check if GPU is throttled**:
   ```bash
   nvidia-smi -q -d PERFORMANCE
   kubectl get events -n model-serving-prod --sort-by='.lastTimestamp'
   ```

4. **Check if KV cache is the bottleneck**:
   - If `VLLMKVCacheUsageCritical` (cache 100%): vLLM is evicting blocks → reduce `--max-num-seqs` or increase `--gpu-memory-utilization`
   - If `VLLMSwapOutBlocksDetected`: host swap is active → verify swapoff DaemonSet is running:
     ```bash
     kubectl get ds -n model-serving-prod -l app.kubernetes.io/name=model-serving-engine
     kubectl logs -n model-serving-prod ds/<swapoff-daemonset> | grep "swap"
     ```
   - If `VLLMPrefixCacheHitRateLow` (< 20%): prefix caching may not be effective → verify `--enable-prefix-caching` is set
   - If `LMCacheL1HitRateLow` (< 30%): CPU tier cache not effective → verify LMCache daemon health and CPU workers:
     ```bash
     kubectl get pods -n model-serving-prod -l app.kubernetes.io/name=lmcache
     kubectl logs -n model-serving-prod ds/lmcache -c lmcache | grep "L1"
     ```
   - If `LMCacheL2HitRateLow` (< 20%): NVMe tier not effective → check disk usage and path
   - If `LMCacheL3HitRateLow` (< 10%): Redis/S3 tier not effective → verify Redis connectivity (prod only):
     ```bash
     kubectl exec -n model-serving-prod ds/lmcache -- redis-cli -h redis-cache.model-serving-prod.svc.cluster.local ping
     ```
   - If `VLLMPrefillSkipRateLow` (< 10%): cache hits not being leveraged for prefill skip → verify vLLM has `--enable-prefix-caching`
   - If `CacheRoutingHeaderAbsent`: cache-aware routing not configured → verify cache-routing-policy ConfigMap is mounted
   - If `SSMModelPagedAttentionMisconfigured`: SSM/Mamba model deployed with PagedAttention args (anti-pattern, see Bible §14) → remove `--enable-prefix-caching`, `--kv-cache-dtype`, `--block-size` for SSM models (use engine-selector to detect family)
   - If `VLLMRequestsWaitingHigh` (> 10): KEDA should be scaling out → check ScaledObject status:
     ```bash
     kubectl get scaledobject -n model-serving-prod
     kubectl describe scaledobject <name> -n model-serving-prod
     ```

5. **Immediate actions**:
   - If KV cache full: reduce `--max-num-seqs` in values.yaml and redeploy via GitOps
   - If GPU overloaded: scale up replicas (KEDA should auto-scale, but manual override if needed)
     ```bash
     kubectl scale statefulset/<name> --replicas=<n+1> -n model-serving-prod
     ```
   - If model is too large: consider lower quantisation or `--kv-cache-dtype fp8`
   - If swap detected: ensure swapoff DaemonSet is scheduled on all GPU nodes
   - If LMCache daemon down: restart LMCache DaemonSet (cache will rebuild across hierarchy):
     ```bash
     kubectl rollout restart ds/lmcache -n model-serving-prod
     ```
   - If cache-aware routing broken: re-apply cache-routing-policy ConfigMap
   - If SSM model misconfigured: remove PagedAttention args and run `cargo run --bin engine-selector -- <model-path>` to verify family detection
   - If traffic spike is predictable: pre-warm with KEDA cron scaler

6. **Verify recovery**:
   - P95 latency < 2s
   - KV cache usage < 85%
   - Request queue depth < 5
   - No swap-out blocks detected
   - LMCache L1 hit rate > 30% (prod/staging)
   - LMCache L2 hit rate > 20% (prod only)
   - LMCache L3 hit rate > 10% (prod only)
   - Prefill skip rate > 10% under load
   - Cache ROI estimate positive
   - Traffic restored to normal levels

7. **Post-incident**:
   - Add capacity forecast tracking if traffic pattern is new
   - Update model serving runbook with the specific cause
   - If KV cache was the root cause: review `--max-num-seqs` and `--gpu-memory-utilization` tuning
   - If LMCache tier was the root cause: review `tools/cache-roi-calc` to verify the storage tier cost is justified by GPU savings (Bible §9 ROI formula)
   - If SSM model was misconfigured: update model registry entry with correct family metadata and re-run `cargo run --bin engine-selector`