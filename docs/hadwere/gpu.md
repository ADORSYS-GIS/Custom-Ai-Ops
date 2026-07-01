# Guide Approfondi : Choisir, Comprendre et Exploiter les GPU pour l'IA (2026)

> Document de référence technique pour décider **quel GPU choisir, pourquoi, pour quel cas d'usage, et comment l'exploiter concrètement** en production.

---

## Table des matières

1. [Les 3 familles de GPU et leur logique de conception](#1)
2. [Concepts fondamentaux à maîtriser avant de choisir](#2)
3. [Fiche détaillée par GPU : pourquoi, cas d'usage, comment l'exploiter](#3)
4. [Comparatif chiffré des microarchitectures](#4)
5. [Runtimes d'inférence : lequel utiliser avec quel GPU](#5)
6. [Contraintes infrastructure (énergie, refroidissement, réseau)](#6)
7. [Arbre de décision et tableau de synthèse](#7)

---

<a name="1"></a>
## 1. Les 3 familles de GPU et leur logique de conception

Le choix d'un GPU n'est jamais une question de "meilleur GPU dans l'absolu" mais de **contrainte dominante** : budget, fiabilité requise, volume de données, latence tolérée. Trois familles répondent à trois logiques différentes.

### 1.1 GPU grand public (RTX 3090 / 4090 / 5090)
**Pourquoi ils existent :** NVIDIA réutilise le même silicium que les cartes professionnelles mais retire les garanties de fiabilité (ECC, pilotes certifiés) pour baisser le coût. Ce sont des puces optimisées pour le rendu graphique **détournées** vers le calcul IA — d'où un excellent rapport performance brute/prix, mais aucune garantie d'intégrité des données sur la durée.

**Ce qui les limite structurellement :**
- Pas de mémoire ECC → une erreur bit-flip silencieuse peut corrompre un entraînement long sans qu'aucune alerte ne se déclenche.
- Pas de NVLink → impossible de fusionner la mémoire de plusieurs cartes en un pool cohérent ; chaque carte reste une île mémoire isolée reliée seulement par PCIe (64 Go/s), très lent comparé à NVLink (900 Go/s+).
- TDP élevé en format bureau (jusqu'à 600 W sur 3-4 slots) → difficile à densifier en rack.

### 1.2 GPU professionnels / workstation (RTX 6000 Ada)
**Pourquoi ils existent :** combler l'écart entre le grand public et le datacenter pour les équipes qui ont besoin de fiabilité (ECC) et de densité rack, sans payer le prix d'un accélérateur SXM à 30-40k$.

**Ce qui les distingue :**
- ECC sur la GDDR6 → corruption mémoire détectée et corrigée automatiquement, essentiel dès qu'un job tourne plus de quelques heures.
- Format blower double-slot → conçu pour être empilé densément dans un rack, contrairement au format 3-4 slots des cartes gaming.
- Pilotes d'entreprise certifiés → stabilité à long terme, support fournisseur.

### 1.3 GPU datacenter (H100/H200/B200, MI300X/325X/300A)
**Pourquoi ils existent :** répondre à deux besoins que les catégories précédentes ne couvrent pas du tout : (1) entraîner un modèle qui ne tient sur aucune carte seule (des centaines de milliards de paramètres), et (2) servir des milliers de requêtes concurrentes avec une latence garantie.

**Ce qui les distingue fondamentalement :** la mémoire HBM montée *on-package* (directement sur le die, pas sur la carte) et des interconnexions propriétaires (NVLink, Infinity Fabric) qui permettent à plusieurs GPU physiques de se comporter comme **un seul GPU logique** avec un espace mémoire unifié.

---

<a name="2"></a>
## 2. Concepts fondamentaux à maîtriser avant de choisir

Ces concepts expliquent *pourquoi* deux GPU aux specs proches peuvent avoir des performances réelles très différentes. Il faut les comprendre avant de lire les fiches produit, sinon le choix se fait sur du marketing plutôt que sur la charge de travail réelle.

### 2.1 Prefill vs Decode — le concept le plus important pour choisir un GPU d'inférence

Une requête LLM traverse deux phases dont les besoins matériels sont **opposés** :

- **Prefill** (lecture et encodage du prompt) : le GPU multiplie de grandes matrices denses sur tout le prompt d'un coup. C'est un calcul massivement parallèle → **limité par les TFLOPS** (puissance de calcul brute des Tensor Cores).
- **Decode** (génération token par token) : pour produire *chaque* nouveau token, le GPU doit recharger l'intégralité des poids du modèle + le KV Cache accumulé depuis la HBM vers ses registres. Le calcul en lui-même est trivial ; ce qui coûte cher, c'est le **transfert mémoire**. → **limité par la bande passante HBM**, pas par les TFLOPS.

**Conséquence pratique directe :** si votre charge est dominée par le decode (chatbots, génération longue), acheter un GPU avec plus de TFLOPS mais la même bande passante mémoire n'apporte presque rien. Il faut acheter de la **bande passante**.

Preuve chiffrée — Llama 2 70B (140 Go en FP16), un seul flux :

```
Temps de lecture = Poids du modèle / Bande passante mémoire

H100 SXM (3 350 Go/s) : 140 / 3350 ≈ 41,8 ms → ~24 tokens/s
H200 SXM (4 800 Go/s) : 140 / 4800 ≈ 29,2 ms → ~34 tokens/s
```

Même puce de calcul, même TFLOPS : le seul changement (mémoire) donne **+42 % de débit**. C'est la démonstration que pour du decode, la bande passante prime sur tout le reste.

### 2.2 Le "CUDA Gap" — pourquoi les specs papier mentent

Sur le papier, l'AMD MI300X affiche **1 307 TFLOPS** en FP16/BF16 dense contre **990 TFLOPS** pour le H100 — un avantage théorique de +32,1 %. En production, c'est l'inverse qui se produit, et l'écart grandit avec l'échelle :

| Contexte | Débit réel NVIDIA vs AMD |
|---|---|
| 2 GPU | H100 +29,4 % |
| 4 GPU | H100 +38,9 % |
| 8 GPU (nœud complet) | H100 +46 %, latence -31,9 % |
| 16 utilisateurs concurrents | H100 +30,8 % / B200 +76,5 % |
| 128 utilisateurs concurrents | H100 +38,7 % / B200 +105,3 % |
| 512 utilisateurs concurrents | H100 +67 % / B200 +77,9 % |

**Pourquoi cet écart existe :** la performance réelle dépend de la capacité du compilateur et des bibliothèques bas niveau (cuBLAS, cuDNN, TensorRT) à *effectivement remplir* les unités de calcul sans temps mort. NVIDIA a 15 ans d'optimisation CUDA accumulée ; ROCm (AMD) est un écosystème plus jeune qui rattrape son retard mais n'a pas encore atteint la même maturité, en particulier sous forte concurrence où l'ordonnancement des requêtes devient critique.

**Ce que ça implique pour le choix :** ne jamais dimensionner un cluster sur la base des TFLOPS annoncés seuls. Toujours pondérer par le score CUDA Gap au niveau de concurrence visé (16, 128, 512 utilisateurs selon le profil réel de trafic).

### 2.3 NUMA interne — le piège caché des GPU à chiplets (AMD MI300X)

Le MI300X n'est pas un GPU monolithique : c'est **8 chiplets de calcul (XCD)** connectés à de la mémoire HBM répartie. Cela crée un comportement NUMA (Non-Uniform Memory Access) *à l'intérieur même du GPU* :

```
Accès à la HBM locale d'un XCD    : ~0,66 To/s, latence ~50 cycles
Accès à la HBM d'un XCD voisin    : ~0,30 To/s, latence ~100 cycles (via Infinity Fabric)
```

**Pourquoi c'est important :** si le compilateur ou le runtime ne place pas intelligemment les tenseurs sur les bons XCD, une part significative des accès mémoire traverse l'interconnexion interne à moitié débit. C'est une source fréquente de sous-performance non documentée dans les fiches produit — à vérifier explicitement si vous évaluez du MI300X pour de l'inférence à faible latence.

### 2.4 Formule de dimensionnement mémoire (VRAM)

```
VRAM minimale = Poids du modèle + Headroom d'activité

Poids du modèle = Paramètres (milliards) × Précision (octets/paramètre)
   FP16/BF16 = 2 octets  |  FP8 = 1 octet  |  FP4 = 0,5 octet

Headroom d'activité = KV Cache + Activations
   (croît avec la longueur de contexte et la taille du batch)
```

Exemple : Llama 70B en FP16 = 70 × 2 = **140 Go** de poids seuls, avant même de réserver de la place pour le KV Cache — d'où l'impossibilité de le faire tenir sur une carte de 80 Go (H100) sans quantifier ou répartir sur plusieurs GPU.

---

<a name="3"></a>
## 3. Fiche détaillée par GPU : pourquoi, cas d'usage, comment l'exploiter

### 🔹 RTX 4090 / RTX 5090 — Prototypage et modèles légers

**Pourquoi les choisir :** rapport performance/prix imbattable pour du calcul brut à faible batch. La RTX 5090 apporte GDDR7 (1 792 Go/s, +78 % vs 4090) qui profite directement au decode.

**Cas d'usage concrets :**
- Développement local d'un pipeline d'inférence avant déploiement.
- Service de modèles ≤8B (Llama 3.1 8B) à très haut débit pour un coût d'exploitation minime (>90 tok/s observés).
- Exécution de modèles quantifiés (Q4/Q8) pour des besoins de recherche individuelle.

**Comment les exploiter :**
- Toujours quantifier au format Q4/Q8 pour libérer de la VRAM à des activations/KV Cache plus larges, plutôt que rester en FP16 par défaut.
- Utiliser vLLM ou llama.cpp plutôt que des runtimes propriétaires — l'écosystème CUDA grand public est mieux couvert par les outils open-source.
- Ne pas tenter du multi-GPU dense : sans NVLink, l'agrégation de mémoire entre cartes se fait via PCIe, ce qui annule une bonne partie du bénéfice.

**Ce qu'il ne faut PAS en attendre :** fiabilité ECC, service 24/7 en production critique, agrégation mémoire multi-carte efficace.

---

### 🔹 RTX 6000 Ada — Fine-tuning local et modèles intermédiaires

**Pourquoi la choisir :** c'est le seul point d'entrée avec ECC et format rack en dessous du seuil des accélérateurs SXM. 48 Go permet de loger un modèle ~32B quantifié en Q4 (≈16 Go de poids) tout en gardant plus de 30 Go pour le KV Cache et la concurrence.

**Cas d'usage concrets :**
- Fine-tuning local de modèles intermédiaires (Qwen 3 32B et équivalents) sans dépendre du cloud.
- Serveur d'inférence interne pour une équipe (dizaines d'utilisateurs simultanés, pas des milliers).
- Environnement de test avant migration vers un cluster H100/H200 en production.

**Comment l'exploiter :**
- Quantifier systématiquement les modèles >16B en Q4 pour rester confortablement sous la limite de 48 Go et garder de la marge pour le batching dynamique.
- Le TDP de 300 W et le format blower double-slot permettent une densification à 4-8 cartes par serveur — à envisager pour un petit cluster interne avant d'investir dans du SXM.
- Privilégier ce GPU dès que la fiabilité (ECC) devient un critère non négociable, même sans besoin de scale massif.

---

### 🔹 NVIDIA H100 SXM — Le standard entreprise établi

**Pourquoi le choisir :** écosystème logiciel le plus mature (CUDA, TensorRT-LLM, cuBLAS optimisés depuis des années), NVLink 4.0 pour agréger plusieurs cartes en pool mémoire cohérent, disponibilité large sur le marché (cloud et occasion).

**Cas d'usage concrets :**
- Inférence d'entreprise à latence garantie (SLA) sur des modèles jusqu'à ~70B avec parallélisme de tenseur (TP2 minimum, car 80 Go < 140 Go requis en FP16).
- Entraînement distribué de modèles de taille moyenne à grande sur des clusters de plusieurs centaines de GPU.
- Toute charge où la maturité logicielle prime sur la capacité mémoire brute.

**Comment l'exploiter :**
- Pour du 70B+, prévoir le parallélisme de tenseur (TP2 ou plus) dès la conception — une seule carte H100 ne suffit pas.
- Utiliser TensorRT-LLM en production (gain 20-40 % vs vLLM) une fois la configuration figée ; garder vLLM pour l'itération rapide en développement.
- Exploiter le Transformer Engine (FP8 dynamique) pour réduire de moitié l'empreinte mémoire sans réécrire le modèle.

---

### 🔹 NVIDIA H200 SXM — Le choix par défaut pour l'inférence de production en 2026

**Pourquoi le choisir :** même die de calcul que le H100 (donc même maturité logicielle et mêmes TFLOPS), mais 141 Go de HBM3e à 4,8 To/s au lieu de 80 Go à 3,35 To/s. Comme le decode est limité par la bande passante (voir §2.1), c'est un gain de débit direct sans changer une ligne de code applicatif.

**Cas d'usage concrets :**
- Service de modèles 70B en FP8 (~70 Go) ou FP16 sur **une seule carte**, sans TP requis — simplifie fortement l'architecture de déploiement.
- Inférence à très forte concurrence où l'espace libéré par la mémoire supplémentaire sert à agrandir le KV Cache et donc le nombre de requêtes traitées en parallèle.
- Contextes longs (RAG, agents multi-tours) où le KV Cache devient le facteur limitant plutôt que les poids du modèle.

**Comment l'exploiter :**
- Dimensionner le batch dynamique (continuous batching) en profitant de l'espace mémoire libéré plutôt que de garder les mêmes réglages qu'en H100.
- Variante NVL (4 cartes, 1,8 To/s, 564 Go unifiés) à considérer si le besoin dépasse une carte unique sans vouloir monter un cluster SXM complet.

---

### 🔹 NVIDIA B200 (Blackwell) — Le maximum de débit et de flexibilité de précision

**Pourquoi le choisir :** architecture double-die avec espace mémoire unifié 192 Go @ 8 To/s, Tensor Cores 5ᵉ gen avec exécution asynchrone par thread (élimine les temps d'attente warp-synchrone des générations précédentes), et surtout support natif **FP4/FP6** qui réduit drastiquement l'empreinte mémoire des poids (70B en FP4 ≈ 35-40 Go au lieu de 140 Go).

**Cas d'usage concrets :**
- Inférence à très forte concurrence (128-512 utilisateurs) où l'écart avec AMD atteint +77 à +105 % — le choix le plus rentable dès que le trafic est élevé et soutenu.
- Modèles géants nécessitant FP4 pour tenir sur un nombre raisonnable de cartes.
- Entraînement exascale via le rack unifié GB200 NVL72 (72 GPU en un seul domaine NVLink à 130 To/s).

**Comment l'exploiter :**
- Migrer les poids en FP4/FP6 dès que la précision de sortie du modèle le permet — c'est le principal levier pour exploiter pleinement la capacité mémoire de cette génération.
- Le moteur RAS autonome permet de détecter les dérives thermiques ou pannes silencieuses avant interruption de service — à intégrer dans le monitoring cluster plutôt que de le laisser en tâche de fond ignorée.
- Anticiper le TDP (jusqu'à 1 200 W/carte) : refroidissement liquide obligatoire, l'air pulsé classique ne suffit plus (voir §6).

---

### 🔹 AMD Instinct MI300X — Capacité mémoire brute maximale

**Pourquoi le choisir :** 192 Go de HBM3 sur une seule carte, soit plus que le H200 (141 Go), à un TCO souvent inférieur au NVIDIA équivalent. Pertinent quand la contrainte dominante est *la capacité mémoire disponible*, pas la latence à forte concurrence.

**Cas d'usage concrets :**
- Chargement de modèles 70B en FP16 complet sur une seule carte, sans quantification ni parallélisme, quand la précision maximale est requise.
- Environnements où le budget matériel prime sur la latence de pointe (trafic modéré, pas de SLA agressif).
- Organisations engagées dans l'écosystème ouvert ROCm pour des raisons stratégiques (indépendance vis-à-vis de NVIDIA).

**Comment l'exploiter :**
- Utiliser vLLM avec le backend `ROCM_AITER_FA` (optimisé entreprise) plutôt que `ROCM_ATTN` dès que la carte est un MI300X — le second bascule sur des noyaux d'émulation lents pour les têtes d'attention asymétriques.
- Tenir compte du comportement NUMA interne (§2.3) lors du placement des tenseurs : mal géré, l'avantage mémoire brut peut être annulé par la latence d'accès inter-XCD.
- Éviter les déploiements à très forte concurrence (512+ utilisateurs) où l'écart CUDA Gap avec NVIDIA est le plus large — réserver ce GPU aux charges à concurrence modérée.

---

### 🔹 AMD Instinct MI300A — Calcul scientifique et HPC

**Pourquoi le choisir :** c'est un APU, pas un GPU pur — CPU (Zen 4, 24 cœurs) et GPU partagent la même mémoire HBM3 cohérente (128 Go, 5,3 To/s) via l'Infinity Cache. Cela élimine complètement les copies de données via PCIe entre hôte et périphérique.

**Cas d'usage concrets :**
- Calcul scientifique double précision (FP64) où les allers-retours CPU↔GPU sont fréquents (simulations physiques, dynamique moléculaire).
- Charges HPC classiques migrées vers l'IA où le code existant suppose un espace mémoire unifié.
- Tout pipeline où la latence de transfert PCIe hôte-périphérique est le goulot d'étranglement identifié.

**Comment l'exploiter :**
- Concevoir le code pour tirer parti de la cohérence mémoire native plutôt que de répliquer un pattern de copie explicite hérité d'architectures GPU discrètes classiques.
- Réserver ce choix aux charges où le FP64 est réellement nécessaire — pour de l'inférence LLM standard (FP16/FP8/FP4), ce n'est pas le bon outil.

---

<a name="4"></a>
## 4. Comparatif chiffré des microarchitectures

| GPU | Microarchitecture | Mémoire | Bande passante | TDP | Interconnexion | Format |
|---|---|---|---|---|---|---|
| RTX 4090 | Ada Lovelace | 24 Go GDDR6X | 1 008 Go/s | 450–600 W | PCIe4 x16 (64 Go/s) | Bureau |
| RTX 5090 | Ada Lovelace (AD102) | 32 Go GDDR7 | 1 792 Go/s | 575 W | PCIe4 x16 (64 Go/s) | Bureau |
| RTX 6000 Ada | Ada Lovelace | 48 Go GDDR6 ECC | 960 Go/s | 300 W | PCIe4 x16 (64 Go/s) | Rack (blower) |
| A100 SXM | Ampere | 80 Go HBM2e | 2 039 Go/s | 400 W | NVLink 3.0 (600 Go/s) | SXM |
| H100 SXM | Hopper | 80 Go HBM3 | 3,35 To/s | 700 W | NVLink 4.0 (900 Go/s) | SXM |
| H200 SXM | Hopper | 141 Go HBM3e | 4,8 To/s | 700 W | NVLink 4.0 (900 Go/s) | SXM |
| B200 | Blackwell | 192 Go HBM3e | 8,0 To/s | 1 000–1 200 W | NVLink 5.0 (1,8 To/s) | SXM / HGX |
| MI300X | CDNA 3 | 192 Go HBM3 | 5,3 To/s | 750 W | Infinity Fabric 3 | OAM |
| MI325X | CDNA 3 | 256 Go HBM3e | 6,0 To/s | 750 W | Infinity Fabric 3 | OAM |
| MI300A | CDNA 3 (APU) | 128 Go HBM3 partagée | 5,3 To/s | 550–760 W | Infinity Fabric interne | Socket SH5 |

---

<a name="5"></a>
## 5. Runtimes d'inférence : lequel utiliser avec quel GPU

### vLLM — agilité et portabilité
**Pourquoi :** implémente **PagedAttention**, qui élimine la fragmentation du KV Cache (gestion mémoire par pages plutôt que par blocs contigus, comme un OS gère la RAM).

**Comment l'exploiter :**
- vLLM V1 (2025+) a remplacé les noyaux HIP par des noyaux Triton, développés en 3 phases d'optimisation (chargements vectorisés pour le prefill, noyau spécialisé pour le decode à séquence=1, fusion en noyau unique) — utiliser cette version plutôt que V0, désormais obsolète.
- Sur AMD, choisir explicitement `ROCM_AITER_FA` sur MI300X+ plutôt que le backend générique `ROCM_ATTN`.
- Idéal pour prototypage rapide et environnements mixtes NVIDIA/AMD grâce à sa portabilité.

### TensorRT-LLM — performance maximale, propriétaire
**Pourquoi :** compile le modèle en graphe figé optimisé pour le GPU et la précision exacts (FP8/FP4), exploitant pleinement le Transformer Engine et le TMA de Hopper/Blackwell.

**Comment l'exploiter :**
- Réserver à la production stabilisée : la phase de build (15-30 min) fige la config matérielle, donc pas adapté à un environnement qui change souvent de modèle ou de GPU.
- Prévoir un pipeline de recompilation dans le CI/CD dès qu'un modèle est mis à jour.
- Gain net attendu : +20 à 40 % de débit vs vLLM, à condition d'accepter la rigidité de configuration.

### Triton Inference Server — orchestration multi-modèle
**Pourquoi :** ce n'est pas un moteur d'inférence mais un **orchestrateur** — il fait tourner vLLM ou TensorRT-LLM comme backends et gère le partage dynamique de la mémoire GPU entre plusieurs modèles hétérogènes.

**Comment l'exploiter :**
- À utiliser dès qu'un même GPU doit héberger plusieurs modèles (vision + texte + audio par exemple) — le partage mémoire dynamique réduit les coûts opérationnels de 40-60 %.
- Utile pour le rechargement de versions de modèles sans interruption de service (déploiement continu).
- Combiner avec Business Logic Scripting (BLS) pour du chaînage d'inférences complexes (ex : retrieval → rerank → génération).

| Dimension | vLLM | TensorRT-LLM | Triton |
|---|---|---|---|
| Matériel | NVIDIA + AMD ROCm 6.2+ | NVIDIA uniquement | Multi-architectures |
| Complexité | Faible | Élevée (build 15-30 min) | Moyenne |
| Multi-modèle | Limité | Limité | Excellent |
| Cas d'usage | Prototypage, mixte NVIDIA/AMD | Production haute performance, SLA stricts | Serveurs multi-modèles |

---

<a name="6"></a>
## 6. Contraintes infrastructure : pourquoi elles conditionnent le choix du GPU

Choisir un GPU sans vérifier sa faisabilité énergétique et thermique mène à des projets bloqués en phase de déploiement — le goulot d'étranglement en 2026 n'est plus le silicium mais l'électricité et le refroidissement.

### Énergie
- 1 nœud 8× H100 ≈ **10,1 kW** ; 1 nœud 8× B200 ≈ **14,3 kW**.
- Cluster 100 GPU ≈ 176 kW (raccordement commercial standard) · 1 000 GPU ≈ 1,76 MW (transformateurs industriels) · 10 000 GPU ≈ 17,6 MW (sous-station dédiée).
- Délai de raccordement haute tension : souvent **24 à 36 mois** dans les grands pôles technologiques → un critère à vérifier *avant* de valider un choix de GPU à grande échelle, pas après.
- **Stratégie de contournement :** répartir la charge sur plusieurs sites de 100 GPU plutôt que concentrer un cluster de 1 000 sur un site unique — chaque site reste sous les seuils d'alerte réseau.

### Refroidissement
- Limite physique de l'air pulsé : **~20 kW/rack**.
- Un rack H100 (~40 kW) dépasse déjà cette limite ; un rack Blackwell (>120 kW) l'impose absolument → **refroidissement liquide obligatoire dès la génération Hopper dense**.
- Direct-to-Chip (cold plates cuivre) : ~3 500× plus efficace que l'air.
- Immersion biphasique pour les densités extrêmes (cycle passif ébullition/condensation).

**Implication directe sur le choix GPU :** si l'infrastructure existante n'a pas de refroidissement liquide, un déploiement dense de B200 ou même de H100 en rack complet n'est pas réalisable sans travaux préalables — un facteur souvent négligé au moment de choisir le matériel.

### Réseau inter-GPU
- **InfiniBand** (400-800 Gbps) : RDMA natif, latence minimale, référence pour l'entraînement massif.
- **Ethernet optimisé** (Spectrum-X + BlueField-3 + RoCEv2) : performance proche d'InfiniBand avec la flexibilité Ethernet standard — pertinent si l'équipe réseau maîtrise déjà Ethernet plutôt qu'InfiniBand.
- Topologie **Rail-Optimized** : chaque GPU a son propre NIC dédié vers un switch Leaf → All-Reduce sans collision de paquets.

---

<a name="7"></a>
## 7. Arbre de décision et tableau de synthèse

```
Quelle est la contrainte dominante ?
│
├─ Budget minimal, prototypage seul
│   → RTX 4090/5090 + vLLM, modèles quantifiés Q4
│
├─ Fiabilité (ECC) + fine-tuning local, pas de scale massif
│   → RTX 6000 Ada + vLLM
│
├─ Production 70B avec SLA strict, écosystème mature exigé
│   → H200 SXM + TensorRT-LLM
│
├─ Capacité mémoire brute prioritaire sur la latence de pointe
│   → MI300X + vLLM (backend ROCM_AITER_FA)
│
├─ Très forte concurrence (128-512+ utilisateurs simultanés)
│   → B200 + TensorRT-LLM (écart de performance le plus large)
│
├─ Calcul scientifique FP64 / HPC
│   → MI300A
│
└─ Entraînement exascale multi-milliers de GPU
    → GB200 NVL72 + infrastructure liquide dédiée
```

| Profil | GPU recommandé | Runtime | Pourquoi |
|---|---|---|---|
| Cloud provider / entraînement géant | GB200 NVL72 | — | Seule option pour interconnexion à l'échelle rack sans goulot réseau |
| Entreprise / LLM en production, SLA strict | H200 SXM | TensorRT-LLM | Meilleur compromis débit/latence, écosystème le plus mature |
| Budget mémoire prioritaire | MI300X | vLLM (AITER_FA) | 192 Go sur une carte, TCO compétitif |
| HPC scientifique FP64 | MI300A | ROCm | Mémoire cohérente CPU/GPU, zéro copie PCIe |
| PME / recherche / fine-tuning local | RTX 6000 Ada | vLLM + Triton | Coût maîtrisé, ECC, itération rapide |
| Prototypage individuel | RTX 4090/5090 | vLLM ou llama.cpp | Rapport perf/prix, pas de contrainte fiabilité |

---
