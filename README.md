# Atelier VSM

Application desktop (Rust + Tauri 2) de **cartographie de la chaîne de valeur (VSM)** :
on dessine la carte, l'application mesure et calcule en même temps.

## Fonctionnalités

- **24 symboles VSM en SVG faits main** : fournisseur, client, processus, processus partagé,
  stock, supermarché, FIFO, tampon, camion, contrôle de production, boîte de données, heijunka,
  go-see, kanbans (production, retrait, signal, lots), poteau kanban, retrait, bille de séquence,
  opérateur, éclair kaizen, texte, boucle.
- **5 types de flux** accrochés aux symboles : poussé, expédition, retrait, information manuelle,
  information électronique.
- **Ergonomie inspirée de draw.io** : palette avec recherche, glisser-déposer, sélection au
  rectangle, zoom/panoramique, pages (état actuel / futur), annuler/rétablir, copier/coller.
- **Assistance au dessin** : guides d'alignement, détection et reproduction des équidistances avec
  cotes, magnétisme grille, aligner/répartir, mise en page VSM automatique.
- **Calculs Lean en direct** : takt, couverture des stocks, ligne de temps automatique, lead time,
  temps VA, PCE, en-cours (loi de Little), opérateurs théoriques, capacité et charge par poste,
  goulot, équilibrage C/T vs takt, comparaison des états.
- **Assistant** : diagnostics (TPM, SMED, stocks, alignement…) et les 8 questions de l'état futur.
- **Desktop** : ouverture/enregistrement de fichiers `.vsm` (Ctrl+O / Ctrl+S), export SVG,
  rapport CSV calculé par le moteur Rust.
- Onglet **Brainstorming** : dossier de conception complet (génie industriel, modélisation,
  formules, architecture, feuille de route).

## Structure

```
src/                 Interface (HTML/SVG/JS, polices embarquées, fonctionne hors ligne)
src-tauri/src/lib.rs Commandes natives (fichiers, rapport, métriques)
src-tauri/src/vsm.rs Moteur de calcul VSM + tests unitaires
.github/workflows/   Build multi-plateforme et release automatique
```

## Développer

Prérequis : Rust stable, Node 20+, et sous Linux les paquets
`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`.

```bash
npm install
npm run dev        # lance l'application
npm run build      # produit les installateurs dans src-tauri/target/release/bundle
cargo test --manifest-path src-tauri/Cargo.toml
```

## Publier une version

```bash
git tag v0.1.0
git push origin v0.1.0
```

Le workflow `Release` construit les installateurs Windows (`.msi`, `.exe`), macOS (`.dmg`
Apple Silicon et Intel) et Linux (`.AppImage`, `.deb`, `.rpm`) et les attache à la release GitHub.
