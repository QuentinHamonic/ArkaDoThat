# Spécification MVP TaskHUD

## Périmètre

Cette version valide un parcours local et déterministe autour d’un `TODO.md`. Elle ne promet pas encore la fenêtre native Tauri, la détection VS Code, la surveillance OS des fichiers ni le support multi-plateforme.

## Exigences et critères d’acceptation

### T1 — Lire une liste Markdown

À l’ouverture, l’utilisateur voit une liste de tâches issue d’un texte `TODO.md` contenant des lignes `- [ ]` et `- [x]`. Les tâches terminées sont distinguées autrement que par la couleur seule.

**Échec :** une ligne valide disparaît, l’état coché est inversé ou une tâche est présentée comme modifiée alors qu’elle ne l’est pas.

### T2 — Changer l’état d’une tâche

Quand l’utilisateur active la case d’une tâche, son état visible change immédiatement et le texte Markdown associé est mis à jour en mémoire.

**Échec :** seule l’apparence change, ou une autre tâche est modifiée.

### T3 — Ajouter une tâche

Depuis le champ principal, l’utilisateur peut saisir une tâche et l’ajouter avec Entrée ou le bouton d’ajout. Le champ est vidé après succès et la nouvelle tâche apparaît non terminée.

**Échec :** une entrée vide est ajoutée, l’action exige une syntaxe Markdown ou le focus clavier est perdu sans raison.

### T4 — Proposer puis décider

Une proposition d’agent affiche clairement son type, son texte avant/après et les actions `Accepter` et `Refuser`. Accepter modifie la liste ; refuser laisse la liste inchangée. Une proposition ne doit pas être appliquée silencieusement.

**Échec :** le diff est absent, l’état change avant l’acceptation ou les deux décisions ont le même effet.

### T5 — Source et état local

L’interface indique le projet ou fichier actuellement affiché et distingue l’état synchronisé de l’état modifié localement. Un fichier choisi depuis l’interface peut être chargé quand l’API de fichier du navigateur est disponible ; le mode de démonstration reste explicite sinon.

**Échec :** l’utilisateur ne peut pas savoir quel contenu il regarde ou une capacité non disponible est présentée comme persistée.

### T6 — Interface compacte et accessible

Le parcours principal tient dans une petite fenêtre, fonctionne à 320 px de large sans défilement horizontal, possède des contrôles clavier natifs, des libellés accessibles et un focus visible. Les options secondaires ne doivent pas encombrer la liste principale.

**Échec :** contrôle inaccessible au clavier, action iconographique sans nom, texte coupé sans alternative ou débordement horizontal.

## Vérifications attendues

- `npm run check` doit réussir quand les dépendances frontend sont installées.
- Le parcours T1 à T4 doit être exécuté dans un navigateur headless sur le serveur local.
- Le rendu doit être inspecté à 320 px et 768 px de large.
- Les limites liées à Tauri, Rust, VS Code, Wayland, macOS et Linux doivent rester signalées comme non vérifiées tant qu’elles ne sont pas exécutées sur leurs environnements.
