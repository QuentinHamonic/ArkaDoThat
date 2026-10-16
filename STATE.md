# État TaskHUD

- Objectif actuel : livrer une première tranche locale autour de `TODO.md` et préparer sa vérification QA indépendante.
- Dernier résultat vérifié : `npm run check` réussit ; le parcours navigateur couvre coche, ajout, acceptation/refus, chargement Markdown et absence de débordement à 320/768 px. Le rapport Claude reste bloqué après un refus d’export du contenu du dépôt vers le service externe.
- Prochaine action : obtenir l’accord explicite pour transmettre le contrat et les références à Claude, puis faire exécuter la QA indépendante sur un commit candidat.
- Blocages : build Tauri non vérifiable sans Rust ; création ou publication GitHub bloquée par l’authentification `gh` ; QA indépendante bloquée tant que l’export vers Claude n’est pas autorisé.
- Références Git : branche `main`, aucun commit au démarrage, remote local configuré vers `https://github.com/QuentinHamonic/ArkaDoThat.git`.
