# Décisions d’architecture

## 2026-10-16 — MVP web local avant Tauri

Rust/Cargo ne sont pas disponibles dans l’environnement initial. Pour obtenir une tranche observable sans inventer un build desktop, le premier incrément utilise Vite, Svelte et TypeScript dans un navigateur local.

Le noyau Markdown reste isolé dans `src/lib/todo.ts`. La couche navigateur peut charger un fichier lorsque l’API locale est disponible ; l’adaptateur Tauri viendra ensuite remplacer cette frontière sans changer le contrat métier.

## 2026-10-16 — Source humaine conservée

`TODO.md` reste le format de référence : il est lisible, versionnable et diffable. L’état d’interface ne doit pas devenir la source unique.

## 2026-10-16 — Proposition jamais silencieuse

Une proposition d’agent est une intention en attente. Elle est affichée comme un diff avec deux décisions explicites. L’application ne traite pas le texte reçu comme une instruction de confiance.

## Direction visuelle

Le produit s’inspire des extensions TODO Markdown qui gardent la liste humaine et diffable, ainsi que des petites fenêtres flottantes conçues pour rester visibles sans devenir un tableau de bord. La traduction TaskHUD est une surface graphite compacte, une ligne ambre réservée au signal actif et une zone de proposition distincte ; pas de grille de cartes ni de navigation lourde.
