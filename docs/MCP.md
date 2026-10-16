# TaskHUD MCP

TaskHUD expose un serveur MCP local autonome, `taskhud-mcp`, sur `stdio`. Il lit uniquement les projets déclarés dans un fichier JSON et enregistre les propositions dans `.taskhud/proposals.json` sous le projet concerné.

Le serveur ne propose volontairement aucun outil d’application directe. Un agent peut lire les tâches et soumettre un diff, mais l’application de ce diff reste une décision humaine dans TaskHUD.

## Configuration locale

Créer un fichier `config.json` :

```json
{
  "activeProjectId": "taskhud",
  "projects": [
    {
      "id": "taskhud",
      "name": "TaskHUD",
      "root": "C:/Users/you/src/TaskHUD"
    }
  ]
}
```

Le chemin `root` doit désigner un projet explicitement autorisé. TaskHUD y lit uniquement `TODO.md` et écrit les propositions dans `.taskhud/`. Ajoutez `.taskhud/` à l’ignore Git du projet suivi ; le dépôt TaskHUD le fait déjà.

## Lancer le serveur

Depuis une release installée :

```text
taskhud-mcp.exe --config C:\Users\you\AppData\Local\TaskHUD\config.json
```

Depuis les sources :

```powershell
cargo run -p taskhud-mcp -- --config C:\Users\you\AppData\Local\TaskHUD\config.json
```

Le serveur n’a pas besoin que la fenêtre Tauri soit ouverte.

## Outils exposés

- `list_projects`
- `get_project_tasks`
- `get_active_project`
- `propose_task_changes`
- `list_proposals`
- `get_proposal`

`propose_task_changes` exige la version SHA-256 retournée par `get_project_tasks`. Une version obsolète est refusée. Les propositions identiques encore en attente sont dédupliquées.

## Clients MCP

Exemple de configuration générique `stdio` :

```json
{
  "mcpServers": {
    "taskhud": {
      "command": "C:\\Users\\you\\AppData\\Local\\TaskHUD\\taskhud-mcp.exe",
      "args": [
        "--config",
        "C:\\Users\\you\\AppData\\Local\\TaskHUD\\config.json"
      ]
    }
  }
}
```

Les chemins et emplacements de configuration exacts dépendent du client. Le smoke test versionné vérifie la négociation MCP, la découverte des six outils, la lecture d’un projet, la persistance d’une proposition et l’absence d’écriture silencieuse dans `TODO.md`. L’intégration complète de Codex CLI et de Claude Code n’est pas déclarée validée tant qu’elle n’a pas été exécutée avec leurs installations respectives.
