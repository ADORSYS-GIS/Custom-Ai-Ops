#!/usr/bin/env bash
# Registry Consistency Validator
# Ensures every model in registry.yaml has:
# - A corresponding directory in models/
# - Required files: model.md, budget.md, eval-report.md
# - A valid chart reference

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MODELS_DIR="$PROJECT_ROOT/models"
REGISTRY_FILE="$MODELS_DIR/registry.yaml"
CHARTS_DIR="$PROJECT_ROOT/charts"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

errors=0
warnings=0

echo "🔍 Validating model registry consistency..."
echo

# Check if registry.yaml exists
if [[ ! -f "$REGISTRY_FILE" ]]; then
    echo -e "${RED}✗ ERROR: registry.yaml not found at $REGISTRY_FILE${NC}"
    exit 1
fi

# Check if yq is installed (for YAML parsing)
if ! command -v yq &> /dev/null; then
    echo -e "${YELLOW}⚠ WARNING: yq not installed. Using basic grep validation.${NC}"
    USE_YQ=false
else
    USE_YQ=true
fi

# Extract model names from registry.yaml
if [[ "$USE_YQ" == "true" ]]; then
    model_names=$(yq eval '.models[].name' "$REGISTRY_FILE")
else
    # Fallback: extract model names with grep
    model_names=$(grep -E '^\s*-\s*name:' "$REGISTRY_FILE" | sed 's/.*name:\s*//' | tr -d '"' | tr -d "'")
fi

# Validate each model
for model_name in $model_names; do
    echo "📦 Checking model: $model_name"
    
    model_dir="$MODELS_DIR/$model_name"
    
    # Check if model directory exists
    if [[ ! -d "$model_dir" ]]; then
        echo -e "  ${RED}✗ Directory not found: $model_dir${NC}"
        ((errors++))
        continue
    fi
    
    # Check required files
    required_files=("model.md" "budget.md" "eval-report.md")
    for file in "${required_files[@]}"; do
        if [[ ! -f "$model_dir/$file" ]]; then
            echo -e "  ${RED}✗ Missing required file: $file${NC}"
            ((errors++))
        else
            echo -e "  ${GREEN}✓ Found: $file${NC}"
        fi
    done
    
    # Extract chart reference from registry
    if [[ "$USE_YQ" == "true" ]]; then
        chart_name=$(yq eval ".models[] | select(.name == \"$model_name\") | .chart" "$REGISTRY_FILE")
    else
        # Fallback: extract chart with grep (crude but works)
        chart_name=$(grep -A 20 "name: $model_name" "$REGISTRY_FILE" | grep "chart:" | head -1 | sed 's/.*chart:\s*//' | tr -d '"' | tr -d "'")
    fi
    
    # Validate chart exists
    if [[ -n "$chart_name" ]]; then
        chart_path="$CHARTS_DIR/$chart_name"
        if [[ ! -d "$chart_path" ]]; then
            echo -e "  ${RED}✗ Referenced chart not found: $chart_name${NC}"
            ((errors++))
        else
            if [[ ! -f "$chart_path/Chart.yaml" ]]; then
                echo -e "  ${RED}✗ Chart.yaml missing in: $chart_name${NC}"
                ((errors++))
            else
                echo -e "  ${GREEN}✓ Chart exists: $chart_name${NC}"
            fi
        fi
    else
        echo -e "  ${YELLOW}⚠ No chart reference found in registry${NC}"
        ((warnings++))
    fi
    
    echo
done

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [[ $errors -eq 0 && $warnings -eq 0 ]]; then
    echo -e "${GREEN}✓ Registry validation passed!${NC}"
    exit 0
elif [[ $errors -eq 0 ]]; then
    echo -e "${YELLOW}⚠ Registry validation passed with $warnings warning(s)${NC}"
    exit 0
else
    echo -e "${RED}✗ Registry validation failed with $errors error(s) and $warnings warning(s)${NC}"
    exit 1
fi
