# État TaskHUD

- État actuel : fondation V0.1, serveur MCP `stdio` et redesign de l’overlay en cours.
- Dernière vérification : 6 tests Rust, `cargo check --workspace`, smoke test MCP, `npm run check` et parcours navigateur à 320/768 px passent ; le focus revient après ajout.
- Limite constatée : l’inspection automatisée de la fenêtre native Tauri reste indisponible ; le rendu desktop réel est couvert seulement par la compilation et la vérification web.
- Prochaine action : commit/push de l’overlay compact, puis faire valider cette tranche par Claude avec `sonnet`/effort `medium` en autorisant ses contrôles locaux.
- QA indépendante : review MCP enregistrée dans `qa/verify-report-mcp.json`, verdict Claude `BLOCKED` faute d’exécution npm/Playwright dans sa session.
- Git : PR #1 `feat/taskhud-markdown-mvp` vers `dev` ; PR #2 `feat/taskhud-mcp-stdio` empilée ; branche courante `feat/taskhud-overlay-redesign`.
