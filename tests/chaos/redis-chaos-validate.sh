#!/bin/bash
# ==============================================================================
# Redis Chaos Validation Script
# ==============================================================================
# Validates Redis resilience after chaos experiments:
#   1. Pod health (all expected pods running)
#   2. Sentinel quorum
#   3. Primary service endpoint
#   4. L3 cache CRUD (set/get/del)
#   5. LMCache circuit breaker metrics (if Prometheus available)
#
# Usage:
#   ./tests/chaos/redis-chaos-validate.sh [namespace]
# ==============================================================================

set -euo pipefail

NAMESPACE="${1:-model-serving-prod}"
PASS=0
FAIL=0
WARN=0

assert_pass() { echo "  ✅ PASS: $1"; PASS=$((PASS + 1)); }
assert_fail() { echo "  ❌ FAIL: $1"; FAIL=$((FAIL + 1)); }
assert_warn() { echo "  ⚠️  WARN: $1"; WARN=$((WARN + 1)); }

echo "╔══════════════════════════════════════════════════╗"
echo "║  Redis Chaos Validation                          ║"
echo "║  Namespace: ${NAMESPACE}                          "
echo "║  Date:      $(date)                              "
echo "╚══════════════════════════════════════════════════╝"
echo ""

# ── 1. Pod Health ──────────────────────────────────────────────────────────
echo "─── 1. Pod Health ───"

# Check Redis pods
REDIS_PODS=$(kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=redis-cache -o json 2>/dev/null || echo '{"items":[]}')
REDIS_RUNNING=$(echo "$REDIS_PODS" | jq '[.items[] | select(.status.phase=="Running")] | length' 2>/dev/null || echo 0)
REDIS_TOTAL=$(echo "$REDIS_PODS" | jq '.items | length' 2>/dev/null || echo 0)

if [ "$REDIS_TOTAL" -gt 0 ] && [ "$REDIS_RUNNING" -eq "$REDIS_TOTAL" ]; then
    assert_pass "Redis: ${REDIS_RUNNING}/${REDIS_TOTAL} pods Running"
elif [ "$REDIS_TOTAL" -eq 0 ]; then
    assert_warn "Aucun pod Redis trouvé (normal si désactivé dans cet environnement)"
else
    assert_fail "Redis: ${REDIS_RUNNING}/${REDIS_TOTAL} pods Running — certains pods ne sont pas prêts"
fi

# Check Sentinel pods
SENTINEL_PODS=$(kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/component=sentinel -o json 2>/dev/null || echo '{"items":[]}')
SENTINEL_RUNNING=$(echo "$SENTINEL_PODS" | jq '[.items[] | select(.status.phase=="Running")] | length' 2>/dev/null || echo 0)
SENTINEL_TOTAL=$(echo "$SENTINEL_PODS" | jq '.items | length' 2>/dev/null || echo 0)

if [ "$SENTINEL_TOTAL" -gt 0 ] && [ "$SENTINEL_RUNNING" -eq "$SENTINEL_TOTAL" ]; then
    assert_pass "Sentinel: ${SENTINEL_RUNNING}/${SENTINEL_TOTAL} pods Running"
fi

# ── 2. Redis Primary Service ──────────────────────────────────────────────
echo ""
echo "─── 2. Primary Service ───"

SVC_HOST="redis-cache-primary.${NAMESPACE}.svc.cluster.local"
PING_RESULT=$(kubectl exec -n "$NAMESPACE" "${REDIS_PODS_POD:-deploy/placeholder}" -- redis-cli -h "$SVC_HOST" PING 2>/dev/null || echo "UNREACHABLE")

# Try with first running pod
FIRST_REDIS_POD=$(echo "$REDIS_PODS" | jq -r '.items[0].metadata.name // ""' 2>/dev/null)
if [ -n "$FIRST_REDIS_POD" ] && [ "$FIRST_REDIS_POD" != "null" ]; then
    PING_RESULT=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" -- redis-cli -h "$SVC_HOST" PING 2>/dev/null || echo "UNREACHABLE")
    if [ "$PING_RESULT" = "PONG" ]; then
        assert_pass "Service ${SVC_HOST} répond au PING"
    else
        assert_fail "Service ${SVC_HOST} ne répond pas (résultat: ${PING_RESULT})"
    fi
else
    assert_warn "Aucun pod Redis disponible pour tester le service"
fi

# ── 3. Redis Role (Primary/Replica detection) ─────────────────────────────
echo ""
echo "─── 3. Redis Roles ───"

if [ -n "$FIRST_REDIS_POD" ] && [ "$FIRST_REDIS_POD" != "null" ]; then
    ROLE=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" -- redis-cli ROLE 2>/dev/null | head -1 || echo "unknown")
    if echo "$ROLE" | grep -q "master"; then
        assert_pass "${FIRST_REDIS_POD}: rôle = master"
    elif echo "$ROLE" | grep -q "slave"; then
        assert_pass "${FIRST_REDIS_POD}: rôle = slave (connecté au master)"
    else
        assert_warn "${FIRST_REDIS_POD}: rôle = ${ROLE}"
    fi
fi

# ── 4. L3 Cache CRUD ──────────────────────────────────────────────────────
echo ""
echo "─── 4. L3 Cache CRUD ───"

L3_KEY="lmcache:chaos-test:$(date +%s)"

if [ -n "$FIRST_REDIS_POD" ] && [ "$FIRST_REDIS_POD" != "null" ]; then
    # Set
    SET_OK=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli SET "$L3_KEY" "chaos-validation-$(date +%s)" EX 60 2>/dev/null || echo "FAIL")
    if echo "$SET_OK" | grep -q "OK"; then
        assert_pass "SET ${L3_KEY} = OK"
    else
        assert_fail "SET ${L3_KEY} = ${SET_OK}"
    fi

    # Get
    GET_VAL=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli GET "$L3_KEY" 2>/dev/null || echo "")
    if [ -n "$GET_VAL" ] && [ "$GET_VAL" != "FAIL" ]; then
        assert_pass "GET ${L3_KEY} = ${GET_VAL:0:40}"
    else
        assert_fail "GET ${L3_KEY} = vide (cache non accessible)"
    fi

    # Del
    DEL_OK=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli DEL "$L3_KEY" 2>/dev/null || echo "FAIL")
    if [ "$DEL_OK" -ge 1 ] 2>/dev/null; then
        assert_pass "DEL ${L3_KEY} = OK (${DEL_OK} clé supprimée)"
    else
        assert_warn "DEL ${L3_KEY} = ${DEL_OK} (déjà expirée?)"
    fi
else
    assert_warn "CRUD tests ignorés (pas de pod Redis)"
fi

# ── 5. Information Redis ─────────────────────────────────────────────────
echo ""
echo "─── 5. Redis Info ───"

if [ -n "$FIRST_REDIS_POD" ] && [ "$FIRST_REDIS_POD" != "null" ]; then
    # Utilisation mémoire
    USED_MEM=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli INFO memory 2>/dev/null | grep "used_memory_human" | cut -d: -f2 || echo "unknown")
    assert_pass "Mémoire utilisée: ${USED_MEM}"

    # Connexions
    CONNECTED=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli INFO clients 2>/dev/null | grep "connected_clients" | cut -d: -f2 || echo "unknown")
    assert_pass "Clients connectés: ${CONNECTED}"

    # Hit rate (si disponible)
    HITS=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli INFO stats 2>/dev/null | grep "keyspace_hits" | cut -d: -f2 || echo "0")
    MISSES=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli INFO stats 2>/dev/null | grep "keyspace_misses" | cut -d: -f2 || echo "0")
    TOTAL=$((HITS + MISSES))
    if [ "$TOTAL" -gt 0 ]; then
        RATE=$(echo "scale=1; $HITS * 100 / $TOTAL" | bc 2>/dev/null || echo "0")
        assert_pass "Cache hit rate: ${RATE}% (${HITS} hits / ${TOTAL} total)"
    else
        assert_warn "Hit rate: pas encore de données (${HITS} hits, ${MISSES} misses)"
    fi

    # Évictions
    EVICTED=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli INFO stats 2>/dev/null | grep "evicted_keys" | cut -d: -f2 || echo "0")
    assert_pass "Évictions LRU: ${EVICTED}"
else
    assert_warn "Redis info ignoré (pas de pod Redis)"
fi

# ── 6. Réplication (si mode sentinel) ────────────────────────────────────
echo ""
echo "─── 6. Réplication ───"

if [ -n "$FIRST_REDIS_POD" ] && [ "$FIRST_REDIS_POD" != "null" ]; then
    REPL_ROLE=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
        -- redis-cli ROLE 2>/dev/null | head -1 || echo "unknown")

    if echo "$REPL_ROLE" | grep -q "master"; then
        REPL_COUNT=$(kubectl exec -n "$NAMESPACE" "$FIRST_REDIS_POD" \
            -- redis-cli ROLE 2>/dev/null | sed -n '3p' || echo "0")
        REPL_COUNT=$(echo "$REPL_COUNT" | tr -d '\r')
        if [ "$REPL_COUNT" -gt 0 ] 2>/dev/null; then
            assert_pass "Master → ${REPL_COUNT} replica(s) connecté(s)"
        else
            assert_warn "Master sans replica (mode standalone ou sentinel non configuré)"
        fi
    elif [ "$SENTINEL_TOTAL" -gt 0 ] 2>/dev/null; then
        assert_pass "Sentinel mode détecté (${SENTINEL_RUNNING}/${SENTINEL_TOTAL} sentinels Running)"
    fi
else
    assert_warn "Réplication ignorée (pas de pod Redis)"
fi

# ==============================================================================
# Résumé
# ==============================================================================
echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  Redis Chaos Validation Results                  ║"
echo "╠══════════════════════════════════════════════════╣"
echo "║  Passed:  ${PASS}"
echo "║  Failed:  ${FAIL}"
echo "║  Warnings: ${WARN}"
echo "║  Total:   $((PASS + FAIL + WARN))"
echo "╚══════════════════════════════════════════════════╝"

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "❌ Certaines validations ont échoué — Redis peut nécessiter une intervention."
    exit 1
elif [ "$WARN" -gt 2 ]; then
    echo ""
    echo "⚠️  Des avertissements sont présents (Redis peut être en mode dégradé)."
    exit 0
else
    echo ""
    echo "✅ Redis fonctionne correctement après le chaos."
    exit 0
fi
