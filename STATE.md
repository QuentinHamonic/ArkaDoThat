# État TaskHUD

- État actuel : fondation V0.1 en cours, avec shell Tauri 2, cœur Rust indépendant et lecture/écriture locale de `TODO.md`.
- Dernière vérification : `cargo test -p taskhud-core` (5 tests), `cargo check --workspace`, `cargo fmt --all -- --check` et `npm run check` passent ; `taskhud.exe` compile et démarre en développement.
- Limite constatée : l’inspection automatisée de la fenêtre native n’est pas disponible dans cette session ; le parcours desktop visuel reste donc non vérifié.
- Prochaine action : commit/push de cette tranche, puis serveur MCP `stdio` sur le même cœur sans application directe des propositions.
- QA indépendante : orchestrateur Claude présent, mais aucun verdict indépendant actuel sur cette fondation.
- Git : branche `feat/taskhud-markdown-mvp`, PR #1 vers `dev`, remote `https://github.com/QuentinHamonic/ArkaDoThat.git`.
