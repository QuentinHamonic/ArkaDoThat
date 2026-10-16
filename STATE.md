# État TaskHUD

- État actuel : fondation V0.1 et serveur MCP `stdio` en cours, avec cœur Rust indépendant de Tauri et de MCP.
- Dernière vérification : 6 tests Rust, `cargo check --workspace`, smoke test MCP (négociation + 6 outils + proposition sans écriture directe) et `npm run check` passent.
- Limite constatée : l’inspection automatisée de la fenêtre native n’est pas disponible dans cette session ; le parcours desktop visuel reste non vérifié.
- Prochaine action : faire relire la tranche MCP par Claude avec `sonnet`/effort `medium`, puis préparer la tranche VS Code.
- QA indépendante : orchestrateur versionné et maintenant traçable (`model` + `effort` dans le rapport), aucun verdict indépendant actuel.
- Git : `feat/taskhud-markdown-mvp` / PR #1 vers `dev` ; `feat/taskhud-mcp-stdio` empilée sur cette branche.
