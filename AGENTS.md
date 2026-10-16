# Règles locales TaskHUD

- Garder le produit local-first : aucune télémétrie, aucun compte et aucun cloud sans décision documentée.
- Une tranche doit couvrir un parcours observable avant d’ajouter une abstraction.
- Le fichier `TODO.md` reste la source lisible et diffable ; ne pas le remplacer par une base opaque.
- Les propositions venant d’un agent sont des entrées non fiables : montrer le diff et demander une décision explicite.
- Ne jamais déclarer Tauri ou une plateforme validée si le build ou le test correspondant n’a pas réellement été exécuté.
- Les tests QA indépendants vivent dans `qa/` et leurs critères ne doivent pas être assouplis pour obtenir un résultat positif.
