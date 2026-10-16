# TaskHUD

TaskHUD est un mini gestionnaire de tâches local pour les développeurs qui travaillent avec des agents IA.

Le projet vise une petite fenêtre toujours accessible, centrée sur un fichier `TODO.md` par projet. Les données restent locales et lisibles ; aucun compte ni service cloud n’est nécessaire.

## État du projet

Le premier prototype vérifiable est une interface web locale qui couvre le parcours `TODO.md` : chargement, lecture, coche, ajout, proposition sous forme de diff et acceptation/refus. Il ne s’agit pas encore du binaire Tauri.

La compilation Tauri est bloquée sur la machine actuelle tant que Rust/Cargo ne sont pas installés. Voir [STATE.md](STATE.md) pour le point de reprise.

## Développement local

```text
npm install
npm run dev
```

Puis ouvrir l’URL affichée par Vite. La commande `npm run check` vérifie le typage et le build frontend.

## Vérification indépendante

Le contrat QA est figé dans [docs/SPEC.md](docs/SPEC.md) et [qa/QA-PROMPT.md](qa/QA-PROMPT.md). Le script `scripts/qa-runner.mjs` invoque Claude CLI dans une session neuve et conserve un rapport persistant.

## Licence

À définir avant la première publication publique.
