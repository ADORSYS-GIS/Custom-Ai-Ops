#!/usr/bin/env bash
# Documentation Link Validator
# Checks that all referenced .md files exist

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

errors=0
warnings=0
checked=0

echo "🔍 Validating documentation links..."
echo

cd "$PROJECT_ROOT"

# List of referenced files from grep search
references=(
    "docs/integration-report.md"
    "docs/explain/kv-cache.md"
    "docs/explain/bible-kv-cache.md"
    "docs/architecture/04-gitops-deployment.md"
    "docs/runbooks/latency-spike.md"
    "docs/architecture/01-formats-and-engines.md"
    "docs/architecture/00-overview.md"
    "docs/explain/vllm+lmcache-theory.md"
    "docs/explain/vllm+lmcache-practice.md"
    "docs/runbooks/pod-crashloop.md"
)

# Check main documentation files
echo "📄 Checking referenced documentation files..."
for ref in "${references[@]}"; do
    ((checked++))
    if [[ -f "$ref" ]]; then
        echo -e "  ${GREEN}✓${NC} $ref"
    else
        echo -e "  ${RED}✗${NC} $ref - MISSING"
        ((errors++))
    fi
done
echo

# Check top-level docs
echo "📚 Checking top-level documentation..."
top_level_docs=(
    "README.md"
    "impl.md"
    "solve.md"
    "namage.md"
)

for doc in "${top_level_docs[@]}"; do
    ((checked++))
    if [[ -f "$doc" ]]; then
        echo -e "  ${GREEN}✓${NC} $doc"
    else
        echo -e "  ${RED}✗${NC} $doc - MISSING"
        ((errors++))
    fi
done
echo

# Check architecture docs directory
echo "🏗️  Checking architecture docs..."
arch_docs=(
    "docs/architecture/00-overview.md"
    "docs/architecture/01-formats-and-engines.md"
    "docs/architecture/02-gpu-scheduling.md"
    "docs/architecture/04-gitops-deployment.md"
    "docs/architecture/06-resilience-and-dr.md"
    "docs/architecture/07-capacity-forecasting.md"
)

for doc in "${arch_docs[@]}"; do
    ((checked++))
    if [[ -f "$doc" ]]; then
        echo -e "  ${GREEN}✓${NC} $(basename $doc)"
    else
        echo -e "  ${YELLOW}⚠${NC} $(basename $doc) - not found (may not be implemented yet)"
        ((warnings++))
    fi
done
echo

# Check ADR directory
echo "📋 Checking ADRs..."
if [[ -d "docs/adr" ]]; then
    adr_count=$(find docs/adr -name "*.md" -type f 2>/dev/null | wc -l)
    echo -e "  ${GREEN}✓${NC} docs/adr/ exists ($adr_count ADR files found)"
else
    echo -e "  ${RED}✗${NC} docs/adr/ missing"
    ((errors++))
fi
echo

# Check runbooks
echo "📖 Checking runbooks..."
runbooks=(
    "docs/runbooks/gpu-node-failure.md"
    "docs/runbooks/latency-spike.md"
    "docs/runbooks/pod-crashloop.md"
)

for runbook in "${runbooks[@]}"; do
    ((checked++))
    if [[ -f "$runbook" ]]; then
        echo -e "  ${GREEN}✓${NC} $(basename $runbook)"
    else
        echo -e "  ${YELLOW}⚠${NC} $(basename $runbook) - not found (may not be implemented yet)"
        ((warnings++))
    fi
done
echo

# Check explain docs
echo "💡 Checking explain docs..."
explain_docs=(
    "docs/explain/kv-cache.md"
    "docs/explain/bible-kv-cache.md"
    "docs/explain/gpu.md"
)

for doc in "${explain_docs[@]}"; do
    ((checked++))
    if [[ -f "$doc" ]]; then
        echo -e "  ${GREEN}✓${NC} $(basename $doc)"
    else
        echo -e "  ${YELLOW}⚠${NC} $(basename $doc) - not found (may not be implemented yet)"
        ((warnings++))
    fi
done
echo

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Checked $checked documentation references"

if [[ $errors -eq 0 && $warnings -eq 0 ]]; then
    echo -e "${GREEN}✓ All documentation links validated successfully!${NC}"
    exit 0
elif [[ $errors -eq 0 ]]; then
    echo -e "${YELLOW}⚠ Documentation validation passed with $warnings warning(s)${NC}"
    echo -e "${YELLOW}  Some planned documentation files are not yet implemented.${NC}"
    exit 0
else
    echo -e "${RED}✗ Documentation validation failed with $errors error(s) and $warnings warning(s)${NC}"
    exit 1
fi
