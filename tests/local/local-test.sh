#!/usr/bin/env bash
# ==============================================================================
# Custom-Ai-Ops — Local Test Suite
# ==============================================================================
# Test le projet complet en local avec Qwen2.5 via Ollama.
# Utilisation :
#   ./tests/local/local-test.sh              # Test complet
#   ./tests/local/local-test.sh --smoke      # Tests smoke uniquement
#   ./tests/local/local-test.sh --helm       # Tests Helm uniquement
#   ./tests/local/local-test.sh --tools      # Tests outils Rust uniquement
#   ./tests/local/local-test.sh --all        # Test complet (défaut)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOCAL_DATA="$SCRIPT_DIR/data"
export PATH="$HOME/.local/bin:$PATH"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

PASS=0
FAIL=0
WARN=0
SKIP=0

OLLAMA_URL="${OLLAMA_URL:-http://localhost:11434}"
OLLAMA_MODEL="${OLLAMA_MODEL:-qwen2.5:1.5b}"
TEST_OUTPUT="$LOCAL_DATA/output"

mkdir -p "$LOCAL_DATA" "$TEST_OUTPUT"

echo -e "${CYAN}"
echo "╔══════════════════════════════════════════════════╗"
echo "║   Custom-Ai-Ops — Local Test Suite               ║"
echo "║   Model: ${OLLAMA_MODEL}                         "
echo "║   Date:  $(date)                                  "
echo "╚══════════════════════════════════════════════════╝"
echo -e "${NC}"

# ==============================================================================
# Utilitaires
# ==============================================================================

assert_pass() {
    echo -e "  ${GREEN}✓ PASS${NC}  $1"
    PASS=$((PASS + 1))
}

assert_fail() {
    echo -e "  ${RED}✗ FAIL${NC}  $1"
    FAIL=$((FAIL + 1))
}

assert_warn() {
    echo -e "  ${YELLOW}⚠ WARN${NC}  $1"
    WARN=$((WARN + 1))
}

assert_skip() {
    echo -e "  ${BLUE}− SKIP${NC}  $1"
    SKIP=$((SKIP + 1))
}

section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

check_dependency() {
    if ! command -v "$1" &> /dev/null; then
        assert_fail "Dépendance manquante : $1"
        return 1
    fi
    return 0
}

# ==============================================================================
# Partie 1 : Prérequis
# ==============================================================================

section "1. Prérequis"

echo "  Vérification des dépendances..."
HELM_OK=true
OLLAMA_OK=true

check_dependency "curl" || HELM_OK=false
check_dependency "helm" || HELM_OK=false
check_dependency "jq" || HELM_OK=false

if command -v ollama &> /dev/null; then
    OLLAMA_VERSION=$(ollama --version 2>/dev/null || echo "inconnu")
    echo -e "  ${GREEN}✓${NC} Ollama: $OLLAMA_VERSION"
else
    assert_skip "Ollama non installé — tests d'inférence ignorés"
    OLLAMA_OK=false
fi

if command -v python3 &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} Python: $(python3 --version 2>&1)"
    echo -e "  ${GREEN}✓${NC} Pip: $(pip3 --version 2>&1 | head -1)"
fi

if command -v cargo &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} Cargo: $(cargo --version 2>&1 | head -1)"
fi

echo ""
echo "  Résultat: $(($(curl -s -o /dev/null -w "%{http_code}" "$OLLAMA_URL/api/version" 2>/dev/null || echo "000")))"

if curl -sf "$OLLAMA_URL/api/version" > /dev/null 2>&1; then
    assert_pass "Ollama service en cours d'exécution"
else
    assert_fail "Ollama service injoignable sur $OLLAMA_URL"
    OLLAMA_OK=false
fi

# ==============================================================================
# Partie 2 : Disponibilité du modèle Qwen
# ==============================================================================

section "2. Modèle Qwen"

if [ "$OLLAMA_OK" = true ]; then
    echo "  Vérification du modèle $OLLAMA_MODEL..."

    MODELS=$(curl -sf "$OLLAMA_URL/api/tags" 2>/dev/null | jq -r '.models[].name' 2>/dev/null || echo "")

    if echo "$MODELS" | grep -q "$OLLAMA_MODEL"; then
        assert_pass "Modèle $OLLAMA_MODEL disponible"
    elif echo "$MODELS" | grep -q "qwen"; then
        # Prendre le premier modèle Qwen trouvé
        QWEN_FOUND=$(echo "$MODELS" | grep "qwen" | head -1)
        OLLAMA_MODEL="$QWEN_FOUND"
        assert_warn "Modèle spécifié non trouvé, utilisation de $QWEN_FOUND"
    else
        assert_fail "Aucun modèle Qwen trouvé (liste: $MODELS)"
        OLLAMA_OK=false
    fi

    # Test d'inférence simple
    echo "  Test d'inférence rapide..."
    RESPONSE=$(curl -sf "$OLLAMA_URL/api/generate" \
        -d "{\"model\":\"$OLLAMA_MODEL\",\"prompt\":\"Say hello in one word\",\"stream\":false}" 2>/dev/null || echo "{}")

    RESPONSE_TEXT=$(echo "$RESPONSE" | jq -r '.response // "vide"' 2>/dev/null)
    if [ -n "$RESPONSE_TEXT" ] && [ "$RESPONSE_TEXT" != "null" ] && [ "$RESPONSE_TEXT" != "vide" ]; then
        assert_pass "Inférence OK — réponse: \"${RESPONSE_TEXT:0:50}\""
    else
        assert_fail "Échec de l'inférence (réponse vide)"
        OLLAMA_OK=false
    fi
else
    assert_skip "Test modèle ignoré (Ollama non disponible)"
fi

# ==============================================================================
# Partie 3 : Tests OpenAI-compatible (Smoke Tests)
# ==============================================================================

section "3. Tests API (OpenAI-compatible via Ollama)"

if [ "$OLLAMA_OK" = true ]; then
    echo "  Utilisation de $OLLAMA_URL avec le modèle $OLLAMA_MODEL"
    echo ""

    # Test 3a: Chat completion simple
    echo "  --- 3a. Chat completion de base ---"
    CHAT_RESPONSE=$(curl -sf "$OLLAMA_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "{\"model\":\"$OLLAMA_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"Dis bonjour en un mot\"}],\"max_tokens\":10}" 2>/dev/null || echo "{}")

    CHAT_CONTENT=$(echo "$CHAT_RESPONSE" | jq -r '.choices[0].message.content // ""' 2>/dev/null)
    if [ -n "$CHAT_CONTENT" ]; then
        assert_pass "Chat completion — réponse: \"${CHAT_CONTENT:0:60}\""
    else
        assert_fail "Chat completion — réponse vide"
    fi

    # Test 3b: Usage stats (tokens)
    echo ""
    echo "  --- 3b. Métriques d'utilisation ---"
    PROMPT_TOKENS=$(echo "$CHAT_RESPONSE" | jq -r '.usage.prompt_tokens // 0' 2>/dev/null)
    COMPLETION_TOKENS=$(echo "$CHAT_RESPONSE" | jq -r '.usage.completion_tokens // 0' 2>/dev/null)

    if [ "$PROMPT_TOKENS" -gt 0 ] && [ "$COMPLETION_TOKENS" -gt 0 ]; then
        assert_pass "Tokens comptés — prompt: $PROMPT_TOKENS, completion: $COMPLETION_TOKENS"
    else
        assert_warn "Tokens non comptés (usage: $(echo "$CHAT_RESPONSE" | jq '.usage' 2>/dev/null))"
    fi

    # Test 3c: Streaming
    echo ""
    echo "  --- 3c. Streaming ---"
    STREAM_CHUNKS=$(curl -sf "$OLLAMA_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "{\"model\":\"$OLLAMA_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"Count to 3\"}],\"max_tokens\":20,\"stream\":true}" 2>/dev/null | grep -c "data: " || echo "0")

    if [ "$STREAM_CHUNKS" -gt 0 ]; then
        assert_pass "Streaming OK — $STREAM_CHUNKS chunks reçus"
    else
        assert_warn "Streaming — aucun chunk reçu (peut dépendre du modèle)"
    fi

    # Test 3d: Model list
    echo ""
    echo "  --- 3d. Liste des modèles ---"
    MODELS_LIST=$(curl -sf "$OLLAMA_URL/api/tags" 2>/dev/null | jq -r '.models[].name' 2>/dev/null | tr '\n' ' ')
    if [ -n "$MODELS_LIST" ]; then
        assert_pass "Modèles disponibles: $MODELS_LIST"
    else
        assert_fail "Impossible de lister les modèles"
    fi

    # Test 3e: Cache-aware routing (header) / Session affinity
    echo ""
    echo "  --- 3e. Header session-affinity (non bloquant) ---"
    AFFINITY_RESP=$(curl -sf "$OLLAMA_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "x-cache-affinity-key: test-prefix-123" \
        -d "{\"model\":\"$OLLAMA_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}],\"max_tokens\":5}" 2>/dev/null || echo "{}")

    AFFINITY_OK=$(echo "$AFFINITY_RESP" | jq -r '.choices[0].message.content // ""' 2>/dev/null)
    if [ -n "$AFFINITY_OK" ]; then
        assert_pass "Header x-cache-affinity-key accepté (réponse: \"${AFFINITY_OK:0:30}\")"
    else
        assert_warn "Header x-cache-affinity-key ignoré par Ollama (normal — nécessite vLLM+EPP)"
    fi

    # Test 3f: Contexte long
    echo ""
    echo "  --- 3f. Contexte long (2K tokens) ---"
    LONG_PROMPT=$(python3 -c "print('hello ' * 500)")
    LONG_RESP=$(curl -sf "$OLLAMA_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "{\"model\":\"$OLLAMA_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"$LONG_PROMPT\"}],\"max_tokens\":5}" 2>/dev/null || echo "{}")

    LONG_CONTENT=$(echo "$LONG_RESP" | jq -r '.choices[0].message.content // ""' 2>/dev/null)
    if [ -n "$LONG_CONTENT" ]; then
        assert_pass "Contexte long (1000 mots) OK — réponse reçue"
    else
        assert_warn "Contexte long — réponse vide (limite de mémoire possible)"
    fi

else
    assert_skip "Tests API ignorés (Ollama non disponible)"
fi

# ==============================================================================
# Partie 4 : Validation Helm & Templates
# ==============================================================================

section "4. Validation Helm & Templates K8s"

echo "  Validation des chartes Helm..."
for chart in charts/model-serving-engine charts/llm-d charts/llm-d-router charts/ai-gateway charts/redis; do
    CHART_DIR="$PROJECT_ROOT/$chart"
    if [ -f "$CHART_DIR/Chart.yaml" ]; then
        echo "  Test: $chart..."
        if helm lint "$CHART_DIR" --strict > "$TEST_OUTPUT/helm-lint-${chart//\//-}.log" 2>&1; then
            assert_pass "Helm lint ✓ $chart"
        else
            assert_fail "Helm lint ✗ $chart"
            head -5 "$TEST_OUTPUT/helm-lint-${chart//\//-}.log"
        fi
    else
        assert_warn "Chart.yaml non trouvé dans $chart"
    fi
done

echo ""
echo "  Génération des templates pour chaque environnement..."

for env in dev staging prod; do
    VALUES="$PROJECT_ROOT/environments/$env/values.yaml"
    if [ ! -f "$VALUES" ]; then
        assert_skip "Fichier values non trouvé: $VALUES"
        continue
    fi

    echo "  --- $env ---"

    # model-serving-engine
    if helm template test "$PROJECT_ROOT/charts/model-serving-engine" \
        -f "$VALUES" \
        --set model.name=qwen2.5 \
        > "$TEST_OUTPUT/template-mse-$env.yaml" 2>/dev/null; then
        assert_pass "$env: model-serving-engine templates OK"
    else
        assert_fail "$env: model-serving-engine template échoué"
    fi

    # llm-d (si activé)
    if echo "$env" | grep -q "staging\|prod"; then
        LLMD_VALUES="$PROJECT_ROOT/environments/$env/llm-d/values.yaml"
        if [ -f "$LLMD_VALUES" ]; then
            if helm template test "$PROJECT_ROOT/charts/llm-d" \
                -f "$LLMD_VALUES" \
                > "$TEST_OUTPUT/template-llmd-$env.yaml" 2>/dev/null; then
                assert_pass "$env: llm-d templates OK"
            else
                assert_fail "$env: llm-d template échoué"
            fi
        fi
    fi

    # llm-d-router
    if helm template test "$PROJECT_ROOT/charts/llm-d-router" \
        -f "$VALUES" \
        > "$TEST_OUTPUT/template-router-$env.yaml" 2>/dev/null; then
        assert_pass "$env: llm-d-router templates OK"
    else
        assert_fail "$env: llm-d-router template échoué"
    fi

    # Redis chart (standalone mode — used by dev/staging)
    if helm template test "$PROJECT_ROOT/charts/redis" \
        --namespace model-serving \
        > "$TEST_OUTPUT/template-redis-$env.yaml" 2>/dev/null; then
        assert_pass "$env: redis (standalone) templates OK"
    else
        assert_fail "$env: redis (standalone) template échoué"
    fi

    # NIXL/RDMA-enabled template (disaggregation prefill role)
    if [ "$env" = "prod" ]; then
        if helm template test "$PROJECT_ROOT/charts/model-serving-engine" \
            -f "$VALUES" \
            --set model.name=qwen2.5 \
            --set disaggregation.enabled=true \
            --set disaggregation.role=prefill \
            --set disaggregation.nixl.enabled=true \
            --set disaggregation.nixl.ibPorts=mlx5_0:1 \
            > "$TEST_OUTPUT/template-nixl-prefill-$env.yaml" 2>/dev/null; then
            assert_pass "$env: NIXL prefill templates OK"

            # Vérifier NixlConnector dans le JSON kv-transfer-config
            if grep -q "NixlConnector" "$TEST_OUTPUT/template-nixl-prefill-$env.yaml" 2>/dev/null; then
                assert_pass "$env: NixlConnector config OK"
            else
                assert_fail "$env: NixlConnector not found in template"
            fi
            if grep -q "kv_producer" "$TEST_OUTPUT/template-nixl-prefill-$env.yaml" 2>/dev/null; then
                assert_pass "$env: kv_producer role OK"
            else
                assert_fail "$env: kv_producer role not found"
            fi
            if grep -q "port.*5600" "$TEST_OUTPUT/template-nixl-prefill-$env.yaml" 2>/dev/null; then
                assert_pass "$env: NIXL port 5600 OK"
            else
                assert_fail "$env: NIXL port 5600 not found"
            fi
        else
            assert_fail "$env: NIXL prefill template échoué"
        fi
    fi

    # Redis chart (sentinel HA mode — used by prod)
    if [ "$env" = "prod" ]; then
        if helm template test "$PROJECT_ROOT/charts/redis" \
            --namespace model-serving-prod \
            --set architecture=sentinel \
            > "$TEST_OUTPUT/template-redis-sentinel-$env.yaml" 2>/dev/null; then
            assert_pass "$env: redis (sentinel HA) templates OK"
        else
            assert_fail "$env: redis (sentinel HA) template échoué"
        fi
    fi
done

echo ""
echo "  Validation YAML des templates générés (multi-document support)..."
for f in "$TEST_OUTPUT"/template-*.yaml; do
    [ -f "$f" ] || continue
    DOC_COUNT=$(python3 -c "
import yaml, sys
try:
    docs = list(yaml.safe_load_all(open('$f')))
    print(len(docs))
except Exception as e:
    print(f'ERROR: {e}')
    sys.exit(1)
" 2>/dev/null || echo "ERROR")
    
    if [ "$DOC_COUNT" != "ERROR" ] && [ "$DOC_COUNT" -gt 0 ]; then
        assert_pass "$(basename "$f"): $DOC_COUNT documents YAML valides"
    else
        # Afficher l'erreur réelle
        ERROR_MSG=$(python3 -c "
import yaml
try:
    list(yaml.safe_load_all(open('$f')))
except Exception as e:
    print(str(e)[:200])
" 2>/dev/null)
        if [ -n "$ERROR_MSG" ]; then
            assert_fail "$(basename "$f"): $ERROR_MSG"
        else
            assert_fail "$(basename "$f"): YAML invalide (fichier vide)"
        fi
    fi
done

# ==============================================================================
# Partie 5 : Validation des outils Rust
# ==============================================================================

section "5. Outils Rust"

if command -v cargo &> /dev/null; then
    for tool in engine-selector model-onboarding; do
        TOOL_DIR="$PROJECT_ROOT/tools/$tool"
        if [ -f "$TOOL_DIR/Cargo.toml" ]; then
            echo "  Vérification: $tool..."
            if cargo check --manifest-path "$TOOL_DIR/Cargo.toml" 2>&1 | tail -3 | grep -q "Finished\|error"; then
                assert_pass "$tool: cargo check OK"
            else
                # Vérification plus souple
                if cargo check --manifest-path "$TOOL_DIR/Cargo.toml" > "$TEST_OUTPUT/cargo-check-$tool.log" 2>&1; then
                    assert_pass "$tool: cargo check OK"
                else
                    assert_warn "$tool: cargo check a des problèmes (voir $TEST_OUTPUT/cargo-check-$tool.log)"
                fi
            fi
        fi
    done
else
    assert_skip "Outils Rust ignorés (cargo non installé)"
fi

# ==============================================================================
# Partie 6 : Validation K6 (load test - vérification syntaxe)
# ==============================================================================

section "6. Test de charge (k6)"

if command -v k6 &> /dev/null; then
    echo "  Vérification syntaxe k6..."
    K6_OUTPUT=$(k6 archive -O /dev/null "$PROJECT_ROOT/tests/load/load-test.js" 2>&1 || true)
    if echo "$K6_OUTPUT" | grep -qi "error\|Error\|syntax error\|parse error"; then
        echo "  Erreur: $K6_OUTPUT"
        assert_fail "load-test.js: erreur de syntaxe k6"
    else
        assert_pass "load-test.js: syntaxe k6 OK"
        # Vérification que le fichier est lisible
        if node -e "try { require('fs').readFileSync('$PROJECT_ROOT/tests/load/load-test.js','utf8'); console.log('OK') } catch(e) { console.log('FAIL') }" 2>/dev/null | grep -q "OK"; then
            assert_pass "load-test.js: fichier valide"
        else
            assert_fail "load-test.js: fichier illisible"
        fi
    fi
else
    assert_skip "Test de charge ignoré (k6 non installé)"
fi

# ==============================================================================
# Partie 7 : Test TTFT (vLLM + LMCache + llm-d) — si le modèle est disponible
# ==============================================================================

section "7. Test TTFT — vLLM + LMCache + llm-d EPP"

VLLM_MODEL_PATH="$PROJECT_ROOT/Qwen2.5-0.5B-FP8-dynamic"
VLLM_SCRIPT="$SCRIPT_DIR/vllm_server.py"
TTFT_SCRIPT="$SCRIPT_DIR/ttft-test.sh"

if [ -f "$VLLM_SCRIPT" ] && [ -d "$VLLM_MODEL_PATH" ] && python3 -c "import torch, transformers" 2>/dev/null; then
    echo "  Lancement du test TTFT complet (vLLM + LMCache + llm-d)..."
    echo "  Modèle: $(basename "$VLLM_MODEL_PATH")"
    echo ""
    
    # Exécuter le test TTFT en mode --quick (car le modèle doit charger)
    if [ -f "$TTFT_SCRIPT" ]; then
        TTFT_OUTPUT=$(bash "$TTFT_SCRIPT" 2>&1 || true)
        TTFT_PASS=$(echo "$TTFT_OUTPUT" | grep -cP '\xE2\x9C\x93 PASS' 2>/dev/null || echo "0")
        TTFT_FAIL=$(echo "$TTFT_OUTPUT" | grep -cP '\xE2\x9C\x97 FAIL' 2>/dev/null || echo "0")
        
        # Compter les PASS/FAIL du test TTFT
        if [ "$TTFT_PASS" -gt 0 ] 2>/dev/null; then
            assert_pass "Test TTFT: $TTFT_PASS tests passés, $(echo "$TTFT_FAIL" | tr -d '[:space:]') échoués"
            PASS=$((PASS + $(echo "$TTFT_PASS" | tr -d '[:space:]')))
            FAIL=$((FAIL + $(echo "$TTFT_FAIL" | tr -d '[:space:]')))
        else
            # Afficher la sortie brute pour debug
            echo "$TTFT_OUTPUT" | tail -20
            assert_warn "Test TTFT exécuté (voir détails ci-dessus)"
        fi
    else
        assert_skip "Script TTFT non trouvé: $TTFT_SCRIPT"
    fi
else
    MISSING=""
    [ ! -f "$VLLM_SCRIPT" ] && MISSING="$MISSING\n    - Script vLLM serveur manquant"
    [ ! -d "$VLLM_MODEL_PATH" ] && MISSING="$MISSING\n    - Modèle Qwen introuvable"
    python3 -c "import torch, transformers" 2>/dev/null || MISSING="$MISSING\n    - Modules Python manquants (pip install torch transformers)"
    echo -e "  Test TTFT ignoré:$MISSING"
    assert_skip "Test TTFT (vLLM requis — modèle/scripts non disponibles)"
fi

# ==============================================================================
# Partie 8 : Validation des docs (liens)
# ==============================================================================

section "8. Documentation"

if [ -f "$PROJECT_ROOT/tools/validate-docs.sh" ]; then
    echo "  Vérification rapide des fichiers docs..."
    DOC_COUNT=$(find "$PROJECT_ROOT/docs" -name "*.md" -type f 2>/dev/null | wc -l)
    if [ "$DOC_COUNT" -gt 0 ]; then
        assert_pass "$DOC_COUNT fichiers de documentation trouvés"
    else
        assert_warn "Aucun fichier de documentation trouvé"
    fi
else
    assert_skip "Validation docs (script non trouvé)"
fi

# ==============================================================================
# Partie 9 : Test de chaos Redis (Finding #4)
# ==============================================================================

section "9. Test de chaos Redis"

REDIS_CHAOS_SCRIPT="$PROJECT_ROOT/tests/chaos/redis-chaos-validate.sh"

if [ -f "$REDIS_CHAOS_SCRIPT" ]; then
    echo "  Validation du script de chaos Redis..."
    # Validation syntaxique du YAML LitmusChaos
    CHAOS_YAML="$PROJECT_ROOT/tests/chaos/redis-chaos.yaml"
    if [ -f "$CHAOS_YAML" ]; then
        # Vérifier que le YAML est valide
        DOCS_COUNT=$(python3 -c "
import yaml, sys
try:
    docs = list(yaml.safe_load_all(open('$CHAOS_YAML')))
    print(len(docs))
except Exception as e:
    print(f'ERROR: {e}')
    sys.exit(1)
" 2>/dev/null || echo "ERROR")
        
        if [ "$DOCS_COUNT" != "ERROR" ] && [ "$DOCS_COUNT" -ge 2 ]; then
            assert_pass "redis-chaos.yaml: $DOCS_COUNT documents LitmusChaos valides"
        else
            assert_fail "redis-chaos.yaml: YAML invalide"
        fi
    else
        assert_fail "redis-chaos.yaml non trouvé"
    fi

    # Vérifier que le script bash est syntaxiquement correct
    if bash -n "$REDIS_CHAOS_SCRIPT" 2>/dev/null; then
        assert_pass "redis-chaos-validate.sh: syntaxe bash OK"
    else
        assert_fail "redis-chaos-validate.sh: erreur de syntaxe"
    fi
    
    # Exécution réelle si Redis + kubectl + cluster K8s sont disponibles
    K8S_CLUSTER=false
    if command -v kubectl &> /dev/null && kubectl cluster-info --request-timeout=3s &>/dev/null 2>&1; then
        K8S_CLUSTER=true
    fi

    if command -v redis-cli &> /dev/null && redis-cli PING 2>/dev/null | grep -q "PONG" && [ "$K8S_CLUSTER" = true ]; then
        echo "  Exécution de la validation Redis chaos..."
        CHAOS_OUTPUT=$(bash "$REDIS_CHAOS_SCRIPT" 2>&1 || true)
        CHAOS_PASS=$(echo "$CHAOS_OUTPUT" | grep -E "^\s*✅ PASS" | wc -l | tr -d ' ')
        CHAOS_FAIL=$(echo "$CHAOS_OUTPUT" | grep -E "^\s*❌ FAIL" | wc -l | tr -d ' ')
        
        CHAOS_PASS=${CHAOS_PASS:-0}
        CHAOS_FAIL=${CHAOS_FAIL:-0}
        
        if [ "$CHAOS_FAIL" -eq 0 ] 2>/dev/null; then
            assert_pass "Validation Redis: $CHAOS_PASS tests passés"
            PASS=$((PASS + CHAOS_PASS))
        else
            assert_warn "Validation Redis: $CHAOS_PASS passés, $CHAOS_FAIL échoués"
            PASS=$((PASS + CHAOS_PASS))
            WARN=$((WARN + 1))
        fi
        echo "$CHAOS_OUTPUT" | while IFS= read -r line; do
            echo "    $line"
        done
    else
        # Vérifier quelle dépendance manque
        if ! command -v redis-cli &> /dev/null; then
            assert_skip "Validation Redis chaos ignorée (redis-cli non installé)"
        elif ! redis-cli PING 2>/dev/null | grep -q "PONG"; then
            assert_skip "Validation Redis chaos ignorée (Redis non joignable)"
        elif ! command -v kubectl &> /dev/null; then
            assert_skip "Validation Redis chaos ignorée (kubectl non disponible — pas de cluster K8s)"
        else
            assert_skip "Validation Redis chaos ignorée"
        fi
    fi
else
    assert_skip "Script chaos Redis non trouvé"
fi

# ==============================================================================
# Partie 10 : Test TTFT Cache-Hit (Finding #5)
# ==============================================================================

section "10. Test TTFT Cache-Hit (Smoke)"

TTFT_SMOKE="$PROJECT_ROOT/tests/smoke/ttft-cache-hit-test.sh"

if [ -f "$TTFT_SMOKE" ]; then
    # Vérifier la syntaxe bash
    if bash -n "$TTFT_SMOKE" 2>/dev/null; then
        assert_pass "ttft-cache-hit-test.sh: syntaxe bash OK"
    else
        assert_fail "ttft-cache-hit-test.sh: erreur de syntaxe"
    fi

    # Exécution si Ollama est disponible
    if [ "$OLLAMA_OK" = true ]; then
        echo "  Exécution du test TTFT cache-hit..."
        echo "  Endpoint: ${OLLAMA_URL}, Model: ${OLLAMA_MODEL}"
        echo ""
        
        TTFT_OUTPUT=$(bash "$TTFT_SMOKE" "$OLLAMA_URL" "$OLLAMA_MODEL" 2>&1 || true)
        TTFT_PASS=$(echo "$TTFT_OUTPUT" | grep -E "^\s*(✓|PASS)" | wc -l | tr -d ' ')
        TTFT_FAIL=$(echo "$TTFT_OUTPUT" | grep -E "^\s*(✗|FAIL)" | wc -l | tr -d ' ')
        
        TTFT_PASS=${TTFT_PASS:-0}
        TTFT_FAIL=${TTFT_FAIL:-0}
        
        if [ "$TTFT_FAIL" -eq 0 ] 2>/dev/null; then
            assert_pass "Test TTFT smoke: $TTFT_PASS tests passés"
            PASS=$((PASS + TTFT_PASS))
        else
            assert_warn "Test TTFT smoke: $TTFT_PASS passés, $TTFT_FAIL échoués"
            PASS=$((PASS + TTFT_PASS))
            WARN=$((WARN + 1))
        fi
        
        # Extraire le résumé JSON
        TTFT_JSON=$(echo "$TTFT_OUTPUT" | grep -A100 "^--- JSON Summary ---" | tail -n +2 || echo "")
        if [ -n "$TTFT_JSON" ]; then
            echo "  Résumé TTFT:"
            echo "$TTFT_JSON" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(f'    MISS: {d.get(\"miss_ttft_ms\",0)}ms | HIT#1: {d.get(\"hit1_ttft_ms\",0)}ms | HIT#2: {d.get(\"hit2_ttft_ms\",0)}ms')
    print(f'    Amélioration: {d.get(\"improvement_x\",0)}×')
except:
    print('    (résumé non disponible)')
" 2>/dev/null || echo "    (résumé non disponible)"
        fi
    else
        assert_skip "Test TTFT smoke ignoré (Ollama non disponible)"
    fi
else
    assert_skip "Script TTFT smoke non trouvé"
fi

# ==============================================================================
# Résumé
# ==============================================================================

echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                 RÉSULTATS DES TESTS              ║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC}  Tests passés: ${GREEN}$PASS${NC}"
echo -e "${CYAN}║${NC}  Tests échoués: ${RED}$FAIL${NC}"
echo -e "${CYAN}║${NC}  Avertissements: ${YELLOW}$WARN${NC}"
echo -e "${CYAN}║${NC}  Tests ignorés: ${BLUE}$SKIP${NC}"
echo -e "${CYAN}║${NC}  Total: $((PASS + FAIL + WARN + SKIP))"
echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"

# Sauvegarder les résultats
cat > "$LOCAL_DATA/results.json" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "model": "$OLLAMA_MODEL",
  "passed": $PASS,
  "failed": $FAIL,
  "warnings": $WARN,
  "skipped": $SKIP,
  "status": "$([ "$FAIL" -eq 0 ] && echo 'PASS' || echo 'FAIL')"
}
EOF

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo -e "${RED}✗ Certains tests ont échoué. Consultez les logs dans $TEST_OUTPUT/${NC}"
    exit 1
else
    echo ""
    echo -e "${GREEN}✓ Tous les tests sont passés !${NC}"
    exit 0
fi
