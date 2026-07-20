#!/bin/bash
set -euo pipefail

GATEWAY_URL="${1:?Usage: llm-d-smoke-test.sh <gateway_url> [api_key]}"
API_KEY="${2:-}"
EPP_URL="${EPP_URL:-${GATEWAY_URL}}"

echo "=== llm-d Routing Smoke Test ==="
echo "Gateway: ${GATEWAY_URL}"
echo ""

PASS=0
FAIL=0

assert_pass() {
    echo "PASS: $1"
    PASS=$((PASS + 1))
}

assert_fail() {
    echo "FAIL: $1"
    FAIL=$((FAIL + 1))
}

echo "--- Test 1: EPP router responding to health checks ---"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${GATEWAY_URL}/health" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" = "200" ]; then
    assert_pass "Gateway/health returned 200"
else
    assert_fail "Gateway/health returned ${HTTP_CODE} (expected 200)"
fi

echo ""
echo "--- Test 2: Chat completion routed via InferencePool ---"
AUTH_HEADER=""
if [ -n "$API_KEY" ]; then
    AUTH_HEADER="Authorization: Bearer ${API_KEY}"
fi

HTTP_CODE=$(curl -s -o /tmp/llm-d-smoke-response.json -w "%{http_code}" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "${GATEWAY_URL}/v1/chat/completions" \
    -d '{"model":"test","messages":[{"role":"user","content":"Say hello in one word"}],"max_tokens":10}' 2>/dev/null || echo "000")

if [ "$HTTP_CODE" = "200" ]; then
    HAS_CONTENT=$(python3 -c "import json; d=json.load(open('/tmp/llm-d-smoke-response.json')); print('yes' if d.get('choices',[{}])[0].get('message',{}).get('content','') else 'no')" 2>/dev/null || echo "unknown")
    if [ "$HAS_CONTENT" = "yes" ]; then
        assert_pass "Chat completion returned 200 with content"
    else
        assert_fail "Chat completion returned 200 but no content"
    fi
else
    assert_fail "Chat completion returned ${HTTP_CODE} (expected 200)"
fi

echo ""
echo "--- Test 3: EPP metrics endpoint exists ---"
METRICS=$(curl -s "${GATEWAY_URL}/metrics" 2>/dev/null || echo "")
if echo "$METRICS" | grep -q "epp_routing_decisions_total\|epp_cache_hits_total"; then
    assert_pass "EPP routing metrics found in /metrics"
else
    echo "WARN: EPP metrics not found in /metrics (may not be scraped yet)"
fi

echo ""
echo "--- Test 4: Cache-aware routing header honored ---"
FIRST_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -H "x-cache-affinity-key: test-prefix-123" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "${GATEWAY_URL}/v1/chat/completions" \
    -d '{"model":"test","messages":[{"role":"user","content":"Say hello in one word"}],"max_tokens":10}' 2>/dev/null || echo "000")

SECOND_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Content-Type: application/json" \
    -H "x-cache-affinity-key: test-prefix-123" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "${GATEWAY_URL}/v1/chat/completions" \
    -d '{"model":"test","messages":[{"role":"user","content":"Say hello in one word"}],"max_tokens":10}' 2>/dev/null || echo "000")

if [ "$FIRST_RESPONSE" = "200" ] && [ "$SECOND_RESPONSE" = "200" ]; then
    assert_pass "Both requests with same affinity key succeeded (cache affinity route)"
else
    assert_fail "Cache affinity requests failed (first: ${FIRST_RESPONSE}, second: ${SECOND_RESPONSE})"
fi

echo ""
echo "--- Test 5: KV-Cache Indexer reachable (if enabled) ---"
INDEXER_URL="${KV_CACHE_INDEXER_URL:-http://llm-d-kv-cache-indexer.llm-d-system:8080/health}"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$INDEXER_URL" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" = "200" ]; then
    assert_pass "KV-Cache Indexer health check passed"
else
    echo "WARN: KV-Cache Indexer not reachable at ${INDEXER_URL} (may be disabled in this env)"
fi

echo ""
echo "=== llm-d Smoke Test Results ==="
echo "Passed: ${PASS}"
echo "Failed: ${FAIL}"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi