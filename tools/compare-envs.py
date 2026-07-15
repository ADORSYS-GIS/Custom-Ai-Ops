#!/usr/bin/env python3
"""
Environment Configuration Comparison Tool
Compares key parameters across dev/staging/prod environments
"""

import yaml
import sys

envs = ['dev', 'staging', 'prod']
configs = {}

# Load configurations
for env in envs:
    try:
        with open(f'environments/{env}/values.yaml') as f:
            configs[env] = yaml.safe_load(f)
    except FileNotFoundError:
        print(f"Error: environments/{env}/values.yaml not found")
        sys.exit(1)

# Extract key parameters
def get_vllm_arg(config, arg_name):
    args = config.get('engine', {}).get('vllm', {}).get('args', [])
    try:
        idx = args.index(arg_name)
        return args[idx + 1] if idx + 1 < len(args) else 'N/A'
    except ValueError:
        return 'N/A'

# Build comparison table
print("\nEnvironment Configuration Comparison")
print("=" * 85)
print(f"{'Parameter':<25} {'dev':<20} {'staging':<20} {'prod':<20}")
print("-" * 85)

comparisons = [
    ('Namespace', lambda c: c.get('global', {}).get('namespace', 'N/A')),
    ('Replicas', lambda c: c.get('replicaCount', 'N/A')),
    ('max-model-len', lambda c: get_vllm_arg(c, '--max-model-len')),
    ('gpu-memory-util', lambda c: get_vllm_arg(c, '--gpu-memory-utilization')),
    ('max-num-seqs', lambda c: get_vllm_arg(c, '--max-num-seqs')),
    ('CPU limits', lambda c: c.get('engine', {}).get('vllm', {}).get('resources', {}).get('limits', {}).get('cpu', 'N/A')),
    ('Memory limits', lambda c: c.get('engine', {}).get('vllm', {}).get('resources', {}).get('limits', {}).get('memory', 'N/A')),
    ('Storage size', lambda c: c.get('persistence', {}).get('size', 'N/A')),
    ('Storage class', lambda c: c.get('persistence', {}).get('storageClass', 'N/A')),
    ('Autoscaling', lambda c: 'enabled' if c.get('autoscaling', {}).get('enabled') else 'disabled'),
    ('KEDA', lambda c: 'enabled' if c.get('autoscaling', {}).get('keda', {}).get('enabled') else 'disabled'),
    ('LMCache', lambda c: 'enabled' if c.get('lmcache', {}).get('enabled') else 'disabled'),
    ('Cache persistence', lambda c: 'enabled' if c.get('cachePersistence', {}).get('enabled') else 'disabled'),
    ('PDB', lambda c: 'enabled' if c.get('podDisruptionBudget', {}).get('enabled', False) else 'disabled'),
    ('GPU swapoff', lambda c: 'enabled' if c.get('gpuNodeSwapoff', False) else 'disabled'),
]

for name, extractor in comparisons:
    row = [name]
    for env in envs:
        try:
            value = extractor(configs[env])
            row.append(str(value))
        except Exception as e:
            row.append('ERROR')
    print(f"{row[0]:<25} {row[1]:<20} {row[2]:<20} {row[3]:<20}")

print("=" * 85)
print("\n✓ Environment comparison completed\n")

# Check for consistency issues
print("Configuration Consistency Checks:")
print("-" * 85)

issues = []

# Check QoS Guaranteed (requests == limits)
for env in envs:
    vllm = configs[env].get('engine', {}).get('vllm', {})
    resources = vllm.get('resources', {})
    requests = resources.get('requests', {})
    limits = resources.get('limits', {})
    
    if requests != limits:
        issues.append(f"{env}: QoS not Guaranteed (requests != limits)")

# Check storage progression
storage_sizes = {env: configs[env].get('persistence', {}).get('size', '') for env in envs}
if storage_sizes['dev'] > storage_sizes['staging'] or storage_sizes['staging'] > storage_sizes['prod']:
    issues.append("Storage sizes: expected dev < staging < prod")

# Check replica progression
replicas = {env: configs[env].get('replicaCount', 1) for env in envs}
if replicas['dev'] > replicas['staging'] or replicas['staging'] > replicas['prod']:
    issues.append("Replica counts: expected dev <= staging <= prod")

if not issues:
    print("✓ No consistency issues found")
else:
    print("⚠ Issues detected:")
    for issue in issues:
        print(f"  - {issue}")

print()
