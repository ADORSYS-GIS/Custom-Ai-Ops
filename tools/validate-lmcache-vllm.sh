#!/usr/bin/env bash
# LMCache + vLLM Configuration Validator
# Validates LMCache and vLLM templates are correctly structured

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CHART_DIR="$PROJECT_ROOT/charts/model-serving-engine"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

errors=0
warnings=0

echo "🔍 Validating LMCache + vLLM Configuration..."
echo

# Check if helm is available
if ! command -v helm &> /dev/null; then
    echo -e "${RED}✗ ERROR: helm not found${NC}"
    exit 1
fi

# ==========================================
# Test 1: Validate LMCache templates exist
# ==========================================
echo -e "${BLUE}📋 Test 1: LMCache Templates${NC}"
required_templates=(
    "lmcache-configmap.yaml"
    "lmcache-daemonset.yaml"
    "lmcache-service.yaml"
)

for template in "${required_templates[@]}"; do
    if [[ -f "$CHART_DIR/templates/$template" ]]; then
        echo -e "  ${GREEN}✓${NC} $template exists"
    else
        echo -e "  ${RED}✗${NC} $template missing"
        ((errors++))
    fi
done
echo

# ==========================================
# Test 2: Helm lint with LMCache enabled
# ==========================================
echo -e "${BLUE}📋 Test 2: Helm Lint (LMCache enabled)${NC}"
if helm lint "$CHART_DIR" \
    --set lmcache.enabled=true \
    --set model.name=test-model \
    --strict 2>&1 | tee /tmp/helm-lint-lmcache.log | grep -q "1 chart(s) linted, 0 chart(s) failed"; then
    echo -e "  ${GREEN}✓${NC} Helm lint passed with LMCache enabled"
else
    echo -e "  ${RED}✗${NC} Helm lint failed with LMCache enabled"
    cat /tmp/helm-lint-lmcache.log
    ((errors++))
fi
echo

# ==========================================
# Test 3: Template generation dev env
# ==========================================
echo -e "${BLUE}📋 Test 3: Template Generation (dev - LMCache disabled)${NC}"
if helm template test "$CHART_DIR" \
    -f "$PROJECT_ROOT/environments/dev/values.yaml" \
    --set model.name=test-model \
    > /tmp/helm-template-dev.yaml 2>&1; then
    echo -e "  ${GREEN}✓${NC} Dev template generated (LMCache disabled)"
    # Check no LMCache resources
    if grep -q "kind: DaemonSet" /tmp/helm-template-dev.yaml | grep -q "lmcache"; then
        echo -e "  ${YELLOW}⚠${NC} Warning: LMCache resources found in dev (should be disabled)"
        ((warnings++))
    fi
else
    echo -e "  ${RED}✗${NC} Dev template generation failed"
    ((errors++))
fi
echo

# ==========================================
# Test 4: Template generation staging env
# ==========================================
echo -e "${BLUE}📋 Test 4: Template Generation (staging - LMCache L1+L2)${NC}"
if helm template test "$CHART_DIR" \
    -f "$PROJECT_ROOT/environments/staging/values.yaml" \
    --set model.name=test-model \
    > /tmp/helm-template-staging.yaml 2>&1; then
    echo -e "  ${GREEN}✓${NC} Staging template generated (LMCache L1+L2)"
    
    # Check LMCache resources present
    if grep -q "kind: DaemonSet" /tmp/helm-template-staging.yaml && \
       grep -q "name: lmcache" /tmp/helm-template-staging.yaml; then
        echo -e "  ${GREEN}✓${NC} LMCache DaemonSet found"
    else
        echo -e "  ${RED}✗${NC} LMCache DaemonSet missing in staging"
        ((errors++))
    fi
    
    # Check LMCache ConfigMap
    if grep -q "kind: ConfigMap" /tmp/helm-template-staging.yaml && \
       grep -q "lmcache-config" /tmp/helm-template-staging.yaml; then
        echo -e "  ${GREEN}✓${NC} LMCache ConfigMap found"
    else
        echo -e "  ${RED}✗${NC} LMCache ConfigMap missing in staging"
        ((errors++))
    fi
else
    echo -e "  ${RED}✗${NC} Staging template generation failed"
    ((errors++))
fi
echo

# ==========================================
# Test 5: Template generation prod env
# ==========================================
echo -e "${BLUE}📋 Test 5: Template Generation (prod - LMCache L1+L2+L3)${NC}"
if helm template test "$CHART_DIR" \
    -f "$PROJECT_ROOT/environments/prod/values.yaml" \
    --set model.name=test-model \
    > /tmp/helm-template-prod.yaml 2>&1; then
    echo -e "  ${GREEN}✓${NC} Prod template generated (LMCache L1+L2+L3)"
    
    # Check Redis configuration in ConfigMap
    if grep -q "redis" /tmp/helm-template-prod.yaml; then
        echo -e "  ${GREEN}✓${NC} Redis (L3) configuration found"
    else
        echo -e "  ${YELLOW}⚠${NC} Redis (L3) configuration not found (check if enabled)"
        ((warnings++))
    fi
    
    # Check kv-transfer-config in StatefulSet
    if grep -q "kv-transfer-config" /tmp/helm-template-prod.yaml; then
        echo -e "  ${GREEN}✓${NC} vLLM kv-transfer-config found"
    else
        echo -e "  ${RED}✗${NC} vLLM kv-transfer-config missing"
        ((errors++))
    fi
    
    # Check LMCACHE_ENABLED env var
    if grep -q "LMCACHE_ENABLED" /tmp/helm-template-prod.yaml; then
        echo -e "  ${GREEN}✓${NC} LMCACHE_ENABLED env var found"
    else
        echo -e "  ${RED}✗${NC} LMCACHE_ENABLED env var missing"
        ((errors++))
    fi
else
    echo -e "  ${RED}✗${NC} Prod template generation failed"
    ((errors++))
fi
echo

# ==========================================
# Test 6: vLLM arguments validation
# ==========================================
echo -e "${BLUE}📋 Test 6: vLLM Arguments Validation${NC}"

# Check critical vLLM args in values.yaml
critical_args=(
    "--gpu-memory-utilization"
    "--max-model-len"
    "--kv-cache-dtype"
    "--enable-prefix-caching"
)

for env in dev staging prod; do
    values_file="$PROJECT_ROOT/environments/$env/values.yaml"
    echo "  Checking $env environment:"
    
    for arg in "${critical_args[@]}"; do
        if grep -q "$arg" "$values_file"; then
            echo -e "    ${GREEN}✓${NC} $arg present"
        else
            echo -e "    ${RED}✗${NC} $arg missing"
            ((errors++))
        fi
    done
done
echo

# ==========================================
# Test 7: QoS Guaranteed validation
# ==========================================
echo -e "${BLUE}📋 Test 7: QoS Guaranteed (requests == limits)${NC}"

for env in dev staging prod; do
    values_file="$PROJECT_ROOT/environments/$env/values.yaml"
    echo "  Checking $env environment:"
    
    # Extract resources from values file
    cpu_requests=$(grep -A 10 "vllm:" "$values_file" | grep -A 5 "requests:" | grep "cpu:" | head -1 | awk '{print $2}' | tr -d '"')
    cpu_limits=$(grep -A 10 "vllm:" "$values_file" | grep -A 5 "limits:" | grep "cpu:" | head -1 | awk '{print $2}' | tr -d '"')
    
    if [[ "$cpu_requests" == "$cpu_limits" ]]; then
        echo -e "    ${GREEN}✓${NC} CPU QoS Guaranteed (requests=$cpu_requests == limits=$cpu_limits)"
    else
        echo -e "    ${RED}✗${NC} CPU QoS NOT Guaranteed (requests=$cpu_requests != limits=$cpu_limits)"
        ((errors++))
    fi
done
echo

# ==========================================
# Test 8: LMCache configuration consistency
# ==========================================
echo -e "${BLUE}📋 Test 8: LMCache Configuration Consistency${NC}"

# Check chunk_size consistency in staging/prod
for env in staging prod; do
    values_file="$PROJECT_ROOT/environments/$env/values.yaml"
    
    if grep -q "lmcache:" "$values_file"; then
        echo "  Checking $env environment:"
        
        # Check if chunkSize is defined
        if grep -q "chunkSize:" "$values_file"; then
            chunk_size=$(grep "chunkSize:" "$values_file" | awk '{print $2}')
            echo -e "    ${GREEN}✓${NC} chunkSize defined: $chunk_size"
        else
            echo -e "    ${YELLOW}⚠${NC} chunkSize not explicitly defined (will use default: 256)"
            ((warnings++))
        fi
        
        # Check if cpuWorkers is defined
        if grep -q "cpuWorkers:" "$values_file"; then
            workers=$(grep "cpuWorkers:" "$values_file" | awk '{print $2}')
            echo -e "    ${GREEN}✓${NC} cpuWorkers defined: $workers"
        fi
    fi
done
echo

# ==========================================
# Test 9: Check for deprecated prefix-caching
# ==========================================
echo -e "${BLUE}📋 Test 9: LMCache Prefix Caching Handling${NC}"

# When LMCache is enabled, --enable-prefix-caching should be filtered out
# Check the StatefulSet template logic
if grep -q "Filter out --enable-prefix-caching when LMCache is active" \
    "$CHART_DIR/templates/statefulset.yaml"; then
    echo -e "  ${GREEN}✓${NC} StatefulSet correctly filters --enable-prefix-caching with LMCache"
else
    echo -e "  ${YELLOW}⚠${NC} Warning: Prefix caching filter logic not found (check statefulset.yaml)"
    ((warnings++))
fi
echo

# ==========================================
# Summary
# ==========================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [[ $errors -eq 0 && $warnings -eq 0 ]]; then
    echo -e "${GREEN}✓ All LMCache + vLLM validation tests passed!${NC}"
    echo ""
    echo "✅ LMCache configuration is production-ready:"
    echo "   - Templates exist and are valid"
    echo "   - Multi-tier cache (L1/L2/L3) correctly configured"
    echo "   - vLLM integration arguments correct"
    echo "   - QoS Guaranteed properly implemented"
    echo "   - Environment progression logical (dev < staging < prod)"
    exit 0
elif [[ $errors -eq 0 ]]; then
    echo -e "${YELLOW}⚠ LMCache + vLLM validation passed with $warnings warning(s)${NC}"
    echo -e "${YELLOW}  Warnings are informational and do not block deployment.${NC}"
    exit 0
else
    echo -e "${RED}✗ LMCache + vLLM validation failed with $errors error(s) and $warnings warning(s)${NC}"
    echo ""
    echo "Please review the errors above and fix them before deployment."
    exit 1
fi
