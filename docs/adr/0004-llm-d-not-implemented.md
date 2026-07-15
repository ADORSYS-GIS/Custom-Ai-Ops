# ADR-0004: Ne Pas Implémenter llm-d Maintenant

**Date**: 15 Juillet 2026  
**Status**: ✅ ACCEPTED  
**Décideurs**: Architecture Team  
**Contexte**: Analyse de l'implémentation llm-d

---

## Contexte

Le fichier `docs/explain/llm-d.md` contient une documentation exhaustive (20 sections, ~15000 mots) sur **llm-d**, un middleware d'orchestration Kubernetes pour l'inférence distribuée de LLMs, développé par Red Hat et accepté comme projet CNCF Sandbox en mars 2026.

llm-d fournit 4 piliers principaux:
1. **Intelligent routing** - cache-aware routing au lieu de round-robin
2. **Disaggregated serving** - prefill et decode sur pods séparés
3. **KV-cache management** - indexation globale des blocs de cache
4. **Operational excellence** - SLO-aware autoscaling, flow control, batch processing

---

## Décision

**Nous décidons de NE PAS implémenter llm-d dans Custom-AI-Ops maintenant**, et de conserver la stack actuelle basée sur :
- vLLM natif avec prefix caching
- LMCache pour tiering multi-niveau
- KEDA pour autoscaling basique
- Kubernetes Service standard pour routing

---

## Rationale

### 1. Use Case du Projet

Custom-AI-Ops est un **template/reference architecture**, pas une production multi-tenant à l'échelle de millions de requêtes. llm-d apporte le plus de valeur dans des scénarios de **très grande échelle** que ce projet ne cible pas prioritairement.

### 2. Complexité vs. Bénéfice

**Complexité ajoutée par llm-d**:
- 7+ nouveaux composants à gérer (Router, EPP, KV-Cache Indexer, InferencePool CRD, Gateway API, etc.)
- Nouveaux CRDs à maintenir (GAIE v0.3.0)
- Nouvelle surface d'erreur et de debug
- Expertise supplémentaire requise

**Bénéfice** :
- Significatif **seulement** à grande échelle (millions de req/jour)
- Nécessite infrastructure RDMA pour disaggregation
- Cache-aware routing apporte 60-80% du gain, mais vLLM natif + LMCache couvre déjà beaucoup via prefix caching local

### 3. Maturité du Projet

llm-d est **CNCF Sandbox** (early-stage):
- Breaking changes attendus entre releases
- Pas encore Incubating ou Graduated
- Documentation officielle recommande validation staging extensive
- Écosystème en évolution rapide

Notre stack actuelle utilise des composants **matures et stables**:
- vLLM: stable, production-ready
- LMCache: stable, bien intégré avec vLLM
- KEDA: CNCF Graduated
- Kubernetes natif: infrastructure de base

### 4. Prérequis Infrastructure

llm-d nécessite pour disaggregation et Wide EP:
- RDMA/InfiniBand/RoCE pour transfer KV cache
- Sans cela, transfer peut être **plus lent** que recompute
- Plain 1GbE/10GbE: disaggregation contre-productive

Custom-AI-Ops doit fonctionner sur infrastructure générique, pas seulement HPC clusters.

### 5. Ce Qui Fonctionne Déjà

Notre stack actuelle fournit :
- ✅ vLLM avec PagedAttention et prefix caching
- ✅ LMCache multi-tier (L0→L1→L2→L3→L4)
- ✅ KEDA autoscaling (queue depth, cache utilization)
- ✅ QoS Guaranteed partout
- ✅ Multi-environnement cohérent (dev/staging/prod)
- ✅ 0 erreurs bloquantes, production-ready

**Suffisant** pour la majorité des use cases.

---

## Conséquences

### Positives ✅

1. **Simplicité opérationnelle**
   - Moins de composants à gérer
   - Stack plus facile à comprendre et débugger
   - Onboarding nouveaux ingénieurs plus rapide

2. **Stabilité**
   - Composants matures (vLLM, KEDA, Kubernetes natif)
   - Pas de breaking changes CNCF Sandbox
   - Moins de risques en production

3. **Portabilité**
   - Fonctionne sur infrastructure générique
   - Pas de dépendance RDMA/InfiniBand
   - Déployable partout (cloud, on-prem, edge)

4. **Maintenance**
   - Moins de dépendances à upgrader
   - Moins de CVEs potentielles
   - Moins de compatibility issues

### Négatives ⚠️

1. **Routing cache-blind**
   - Service Kubernetes = round-robin aveugle
   - Pas d'affinité cache entre requêtes multi-replica
   - Cache hit rate sous-optimal sur traffic partagé

2. **Pas de P/D disaggregation**
   - Prefill et decode sur même GPU
   - Long prefill peut staller decode d'autres users
   - Pas de scaling indépendant des phases

3. **Pas de SLO-aware autoscaling**
   - KEDA réagit à queue depth, pas TTFT/TPOT direct
   - Pas d'optimisation latency-driven fine-grained
   - Scaling plus conservatif

4. **Pas de Wide Expert Parallelism**
   - MoE models limités à 1 GPU
   - Pas de distribution experts multi-node

**Impact**: Limité pour use cases < millions req/jour, critique seulement à **très grande échelle**.

---

## Alternatives Considérées

### Alternative 1: Implémenter llm-d Complet

**Rejeté car**:
- Trop complexe pour le bénéfice (use case du projet)
- CNCF Sandbox = early-stage, pas stable
- Nécessite RDMA (pas garanti)
- Équipe MLOps dédiée requise

### Alternative 2: Implémenter Seulement le Router

**Considéré mais reporté car**:
- Router seul apporte 60-80% du gain
- Mais vLLM prefix caching + LMCache couvrent déjà beaucoup
- Peut être ajouté plus tard si besoin
- Complexité + 3-4 composants quand même

### Alternative 3: Addon Optionnel (Choisi)

**Adopté comme compromis**:
- ✅ Stack actuelle par défaut (simple, stable)
- ✅ llm-d disponible comme addon **optionnel**
- ✅ Guide d'activation fourni
- ✅ Marqué **experimental**
- ✅ Utilisateurs avancés peuvent l'adopter

---

## Framework de Décision Future

### Quand Reconsidérer llm-d ?

**Implémenter llm-d SI** (tous ces critères):
- ☑️ Trafic > 1M requêtes/jour
- ☑️ Multi-tenant strict avec SLAs contractuels
- ☑️ Infrastructure RDMA disponible
- ☑️ Équipe MLOps dédiée (3+ ingénieurs)
- ☑️ Modèles MoE 70B+ nécessitant Wide EP
- ☑️ Cache hit rate prouvé insuffisant avec stack actuelle

### Chemin d'Adoption Incrémental

**Si adoption future**:
1. Router + EPP seul (cache-aware routing)
2. KV-Cache Indexer (routing précis)
3. SLO-Aware Autoscaling
4. P/D Disaggregation (seulement si RDMA)
5. Wide Expert Parallelism (seulement pour MoE)

### Métriques de Surveillance

Monitorer ces métriques pour décision future:
- **Cache hit rate** sur multi-replica (target: > 60%)
- **TTFT P95** (target: < 2000ms)
- **TPOT P95** (target: < 100ms)
- **GPU utilization** (target: > 70%)
- **Request volume** (critère: > 1M/jour)

---

## Actions Découlant de Cette Décision

### Immédiat ✅

1. ✅ Créer `LLM_D_ANALYSIS.md` - analyse complète
2. ✅ Créer cet ADR - décision documentée
3. ✅ Ajouter note dans README - clarification

### Court Terme 📄

4. Créer `addons/llm-d/` - addon optionnel
5. Ajouter guide activation - pour utilisateurs avancés
6. Marquer **experimental** - warning clair

### Moyen Terme 📖

7. Créer guide benchmark - mesure si llm-d nécessaire
8. Créer migration guide - adoption future
9. Monitorer CNCF status - passage Incubating/Graduated

---

## Notes

### Documentation Conservée

La documentation `docs/explain/llm-d.md` est **conservée** car:
- ✅ Référence technique excellente
- ✅ Utile pour utilisateurs qui veulent l'adopter
- ✅ Éducative sur distributed inference
- ✅ Peut servir pour addon optionnel

### Révision Prévue

Cette décision sera **révisée**:
- Quand llm-d atteint CNCF Incubating
- Quand trafic projet > 1M req/jour
- Quand feedback utilisateurs montre besoin
- Annual architecture review

---

## Références

- `docs/explain/llm-d.md` - Documentation complète llm-d
- `LLM_D_ANALYSIS.md` - Analyse détaillée
- llm-d official site: `https://llm-d.ai`
- CNCF Sandbox: `https://github.com/cncf/sandbox/issues/462`
- vLLM disaggregated prefill: `https://docs.vllm.ai/en/stable/features/disagg_prefill/`

---

**Status**: ✅ ACCEPTED  
**Date de décision**: 15 Juillet 2026  
**Prochaine révision**: Q1 2027 ou si llm-d → CNCF Incubating

