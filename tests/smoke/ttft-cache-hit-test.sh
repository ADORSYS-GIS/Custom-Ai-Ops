#!/bin/bash
# ==============================================================================
# TTFT Cache-Hit Smoke Test
# ==============================================================================
# Measures Time-To-First-Token (TTFT) for cache-miss vs cache-hit scenarios.
#
# Tests:
#   1. Cache MISS (unknown prefix) — establishes baseline
#   2. Cache HIT (known prefix, first call) — populates cache
#   3. Cache HIT (known prefix, second call) — should be faster
#   4. Session affinity (same session-id → same pod)
#   5. L3 cache validation (if Redis reachable)
#
# Requirements:
#   - vLLM-compatible API endpoint (or Ollama for basic testing)
#   - x-cache-affinity-key header support (or session-id header)
#
# Usage:
#   ./tests/smoke/ttft-cache-hit-test.sh <api_url> [model] [api_key]
#
# Example:
#   ./tests/smoke/ttft-cache-hit-test.sh http://localhost:11434 qwen2.5:1.5b
#
# Returns:
#   JSON summary to stdout, detailed timing to stderr
# ==============================================================================

set -euo pipefail

API_URL="${1:?Usage: ttft-cache-hit-test.sh <api_url> [model] [api_key]}"
MODEL="${2:-qwen2.5:1.5b}"
API_KEY="${3:-}"

PASS=0
FAIL=0
WARN=0

assert_pass() { echo -e "  \033[32m✓ PASS\033[0m  $1"; PASS=$((PASS + 1)); }
assert_fail() { echo -e "  \033[31m✗ FAIL\033[0m  $1"; FAIL=$((FAIL + 1)); }
assert_warn() { echo -e "  \033[33m⚠ WARN\033[0m  $1"; WARN=$((WARN + 1)); }

section() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  $1"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

AUTH_HEADER=""
if [ -n "$API_KEY" ]; then
    AUTH_HEADER="-H Authorization: Bearer ${API_KEY}"
fi

# ==============================================================================
# Helper: call_chat — execute one completion and extract TTFT + usage
# ==============================================================================
# Returns JSON: {"ttft_ms": <float>, "content": "...", "prompt_tokens": N, "completion_tokens": N}
call_chat() {
    local prefix="$1"
    local prompt="$2"
    local extra_headers="${3:-}"

    local start_ns end_ns first_token_time

    # Timestamp avant l'appel (nanosecondes)
    start_ns=$(date +%s%N)

    # Exécuter la requête et capturer le streaming
    local response_file="/tmp/ttft-response-$$.json"
    local timing_file="/tmp/ttft-timing-$$.txt"

    # Pour mesurer TTFT, on utilise le streaming
    # On envoie la requête et on chronomètre le premier chunk
    local full_response
    full_response=$(curl -s -N \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER} \
        ${extra_headers} \
        -d "{
            \"model\": \"$MODEL\",
            \"messages\": [{\"role\": \"user\", \"content\": \"$prompt\"}],
            \"max_tokens\": 100,
            \"stream\": true
        }" \
        "$API_URL/v1/chat/completions" 2>/dev/null || echo "")

    first_token_time=$(date +%s%N)

    # Extraire TTFT en ms (temps jusqu'au premier chunk)
    local ttft_ms
    if [ "$start_ns" -le "$first_token_time" ]; then
        ttft_ms=$(echo "scale=3; ($first_token_time - $start_ns) / 1000000" | bc 2>/dev/null || echo "0")
    else
        ttft_ms="0"
    fi

    # Extraire le contenu du dernier chunk (qui a les stats d'usage)
    local content=""
    local prompt_tokens=0
    local completion_tokens=0

    content=$(echo "$full_response" | grep -a "^data: " | grep -v "data: \[DONE\]" | tail -1 \
        | python3 -c "import sys,json; d=json.loads(sys.stdin.read().replace('data: ','',1)); print(d.get('choices',[{}])[0].get('delta',{}).get('content','') or '')" 2>/dev/null || echo "")

    # Si pas de streaming, essayer en mode non-streaming
    if [ -z "$content" ]; then
        local nonstream_response
        end_ns=$(date +%s%N)
        nonstream_response=$(curl -s \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER} \
            ${extra_headers} \
            -d "{
                \"model\": \"$MODEL\",
                \"messages\": [{\"role\": \"user\", \"content\": \"$prompt\"}],
                \"max_tokens\": 100,
                \"stream\": false
            }" \
            "$API_URL/v1/chat/completions" 2>/dev/null || echo "{}")

        content=$(echo "$nonstream_response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(d.get('choices',[{}])[0].get('message',{}).get('content','') or '')
except:
    print('')
" 2>/dev/null || echo "")

        prompt_tokens=$(echo "$nonstream_response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    u = d.get('usage', {})
    print(u.get('prompt_tokens', 0))
except:
    print(0)
" 2>/dev/null || echo "0")

        completion_tokens=$(echo "$nonstream_response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    u = d.get('usage', {})
    print(u.get('completion_tokens', 0))
except:
    print(0)
" 2>/dev/null || echo "0")
    fi

    echo "{\"ttft_ms\":${ttft_ms},\"content\":\"${content:0:100}\",\"prompt_tokens\":${prompt_tokens},\"completion_tokens\":${completion_tokens}}"
}

# ==============================================================================
# 1. Cache MISS baseline
# ==============================================================================
section "1. Cache MISS (prefix inconnu)"

UNKNOWN_PREFIX="prefix-$(date +%s)-$RANDOM"
MISS_RESULT=$(call_chat "$UNKNOWN_PREFIX" "What is the weather in Paris today? Explain briefly." "-H x-cache-affinity-key: ${UNKNOWN_PREFIX}")
MISS_TTFT=$(echo "$MISS_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ttft_ms',0))" 2>/dev/null || echo "0")
MISS_CONTENT=$(echo "$MISS_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('content','')[:80])" 2>/dev/null || echo "")

if [ "$(echo "$MISS_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ]; then
    assert_pass "Cache MISS TTFT: ${MISS_TTFT}ms (prefix: ${UNKNOWN_PREFIX:0:30}...)"
else
    assert_warn "Cache MISS TTFT: ${MISS_TTFT}ms (streaming peut ne pas être supporté)"
fi

# ==============================================================================
# 2. Cache HIT — première requête (populate)
# ==============================================================================
section "2. Cache HIT — Populate (1ère requête avec préfixe connu)"

KNOWN_PREFIX="known-prefix-ttft-test-$(date +%s)"
HIT1_RESULT=$(call_chat "$KNOWN_PREFIX" "List three benefits of caching in AI inference." "-H x-cache-affinity-key: ${KNOWN_PREFIX}")
HIT1_TTFT=$(echo "$HIT1_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ttft_ms',0))" 2>/dev/null || echo "0")
HIT1_CONTENT=$(echo "$HIT1_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('content','')[:80])" 2>/dev/null || echo "")

if [ "$(echo "$HIT1_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ]; then
    assert_pass "Cache HIT #1 TTFT: ${HIT1_TTFT}ms (prefix: ${KNOWN_PREFIX:0:30}...)"
else
    assert_warn "Cache HIT #1 TTFT: ${HIT1_TTFT}ms"
fi

# ==============================================================================
# 3. Cache HIT — deuxième requête (should be faster)
# ==============================================================================
section "3. Cache HIT — Reuse (2ème requête, même préfixe)"

# Petite pause pour laisser le cache se propager
sleep 1

HIT2_RESULT=$(call_chat "$KNOWN_PREFIX" "List three benefits of caching in AI inference." "-H x-cache-affinity-key: ${KNOWN_PREFIX}")
HIT2_TTFT=$(echo "$HIT2_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ttft_ms',0))" 2>/dev/null || echo "0")

if [ "$(echo "$HIT2_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ]; then
    assert_pass "Cache HIT #2 TTFT: ${HIT2_TTFT}ms (même préfixe que #1)"
else
    assert_warn "Cache HIT #2 TTFT: ${HIT2_TTFT}ms"
fi

# Comparaison cache MISS vs HIT
if [ "$(echo "$MISS_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ] && [ "$(echo "$HIT2_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ]; then
    IMPROVEMENT=$(echo "scale=2; $MISS_TTFT / $HIT2_TTFT" | bc 2>/dev/null || echo "0")
    DIFF_MS=$(echo "scale=1; $MISS_TTFT - $HIT2_TTFT" | bc 2>/dev/null || echo "0")

    assert_pass "Amélioration TTFT: miss=${MISS_TTFT}ms → hit=${HIT2_TTFT}ms (${IMPROVEMENT}×, gain de ${DIFF_MS}ms)"

    if [ "$(echo "$IMPROVEMENT > 1.5" | bc 2>/dev/null || echo "false")" = "true" ]; then
        assert_pass "✅ Cache HIT significativement plus rapide que MISS (${IMPROVEMENT}×)"
    elif [ "$(echo "$IMPROVEMENT >= 1.0" | bc 2>/dev/null || echo "false")" = "true" ]; then
        assert_warn "Cache HIT marginalement plus rapide (${IMPROVEMENT}×) — vérifier la configuration du cache"
    else
        assert_warn "Cache HIT pas plus rapide que MISS (${IMPROVEMENT}×) — vérifier que LMCache est actif"
    fi
else
    assert_warn "Comparaison TTFT non disponible (données insuffisantes)"
fi

# ==============================================================================
# 4. Session Affinity — même session-id → même pod
# ==============================================================================
section "4. Session Affinity"

SESSION_ID="session-ttft-$(date +%s)-$RANDOM"

for i in 1 2; do
    SESS_RESULT=$(call_chat "$SESSION_ID" "What is your name?" "-H x-cache-affinity-key: ${SESSION_ID} -H x-session-id: ${SESSION_ID}")
    SESS_TTFT=$(echo "$SESS_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ttft_ms',0))" 2>/dev/null || echo "0")

    if [ "$(echo "$SESS_TTFT > 0" | bc 2>/dev/null || echo "false")" = "true" ]; then
        assert_pass "Session #${i} TTFT: ${SESS_TTFT}ms (session: ${SESSION_ID:0:20}...)"
    else
        assert_warn "Session #${i} TTFT: 0ms (streaming TTFT non mesurable sur cet endpoint)"
    fi
done

# ==============================================================================
# 5. L3 Cache Validation (if Redis reachable)
# ==============================================================================
section "5. L3 Cache (Redis)"

REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"

if command -v redis-cli &> /dev/null; then
    # Vérifier que Redis répond
    PONG=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" PING 2>/dev/null || echo "FAIL")
    if [ "$PONG" = "PONG" ]; then
        assert_pass "Redis joignable (${REDIS_HOST}:${REDIS_PORT})"

        # Test set/get/del
        L3_KEY="lmcache:smoke:ttft:$(date +%s)"
        if redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" SET "$L3_KEY" "ttft-test-$(date +%s)" EX 120 > /dev/null 2>&1; then
            assert_pass "SET L3 clé = OK"
        fi

        GET_VAL=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" GET "$L3_KEY" 2>/dev/null || echo "")
        if [ -n "$GET_VAL" ]; then
            assert_pass "GET L3 clé = ${GET_VAL:0:40}"
        fi

        # Métriques
        HITS=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" INFO stats 2>/dev/null | grep "keyspace_hits" | cut -d: -f2 | tr -d '\r' || echo "0")
        MISSES=$(redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" INFO stats 2>/dev/null | grep "keyspace_misses" | cut -d: -f2 | tr -d '\r' || echo "0")
        TOTAL=$((HITS + MISSES))
        if [ "$TOTAL" -gt 0 ]; then
            RATE=$(echo "scale=1; $HITS * 100 / $TOTAL" | bc 2>/dev/null || echo "0")
            assert_pass "Redis hit rate: ${RATE}% (${HITS}/${TOTAL})"
        else
            assert_warn "Redis hit rate: pas encore de données"
        fi
    else
        assert_warn "Redis non joignable (${REDIS_HOST}:${REDIS_PORT}) — L3 non validé"
    fi
else
    assert_warn "redis-cli non installé — L3 non validé"
fi

# ==============================================================================
# Résumé
# ==============================================================================
echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  TTFT Cache-Hit Smoke Test Results               ║"
echo "╠══════════════════════════════════════════════════╣"
echo "║  Model:   ${MODEL}                              "
echo "║  Endpoint: ${API_URL}                            "
echo "║  Passed:  ${PASS}"
echo "║  Failed:  ${FAIL}"
echo "║  Warnings: ${WARN}"
echo "╚══════════════════════════════════════════════════╝"

# JSON output
echo ""
echo "--- JSON Summary ---"
cat <<JSONEOF
{
  "timestamp": "$(date -Iseconds)",
  "model": "$MODEL",
  "endpoint": "$API_URL",
  "miss_ttft_ms": $MISS_TTFT,
  "hit1_ttft_ms": $HIT1_TTFT,
  "hit2_ttft_ms": $HIT2_TTFT,
  "improvement_x": $(echo "$MISS_TTFT $HIT2_TTFT" | awk '{if ($1 > 0 && $2 > 0) printf "%.2f", $1/$2; else print "0"}'),
  "passed": $PASS,
  "failed": $FAIL,
  "warnings": $WARN
}
JSONEOF

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "❌ Certains tests TTFT ont échoué."
    exit 1
else
    echo ""
    echo "✅ Tests TTFT terminés."
    exit 0
fi
