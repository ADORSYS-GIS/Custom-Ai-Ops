# Runbook: Latency Spike / Failover Triggered

## Symptoms
- Alert: `ModelServingHighLatency` (p95 > 2s)
- Alert: `ModelServingCriticalLatency` (p99 > 5s)
- Alert: `VLLMKVCacheUsageHigh` / `VLLMKVCacheUsageCritical` (KV cache > 85% / 100%)
- Alert: `VLLMRequestsWaitingHigh` (queue depth > 10)
- Alert: `VLLMSwapOutBlocksDetected` (KV cache pages swapped to CPU)
- Envoy Gateway may have activated SaaS fallback

## Steps

1. **Check current latency**:
   ```bash
   kubectl logs -n envoy-gateway-system deploy/envoy-gateway | grep "failover"
   ```

2. **Identify bottleneck**:
   - GPU utilisation: Grafana DCGM dashboard
   - **KV cache usage**: Grafana model-serving dashboard → "KV Cache Usage (%)" panel
   - **Request queue depth**: Grafana model-serving dashboard → "Request Queue Depth" panel
   - **Prefix cache hit rate**: Grafana model-serving dashboard → "Prefix Cache Hit Rate (%)" panel
   - **KV cache swap-out**: Grafana model-serving dashboard → "KV Cache Swap-Out Blocks" panel
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
   - If `VLLMPrefixCacheHitRateLow` (< 20%): sticky routing may not be working → verify `x-sticky-session-key` header in gateway logs
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
   - If traffic spike is predictable: pre-warm with KEDA cron scaler

6. **Verify recovery**:
   - P95 latency < 2s on Grafana dashboard
   - KV cache usage < 85%
   - Request queue depth < 5
   - No swap-out blocks detected
   - Fallback route deactivated (priority 0 receiving 100% traffic)

7. **Post-incident**:
   - Add capacity forecast recording rule if traffic pattern is new
   - Update model serving runbook with the specific cause
   - If KV cache was the root cause: review `--max-num-seqs` and `--gpu-memory-utilization` tuning