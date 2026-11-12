# Connecteurs de fonctions TaskHUD

Le cœur TaskHUD expose des opérations métier bornées. Un connecteur peut les appeler sans recevoir un accès général au système de fichiers.

## Façade JSONL locale

`taskhud-functions` lit une fonction par ligne sur `stdin` et écrit une réponse JSON par ligne sur `stdout` :

```json
{"function":"get_tasks","arguments":{"projectId":"maths"}}
```

Les fonctions disponibles sont :

- `list_projects`
- `get_active_project`
- `get_tasks`
- `get_tasks_with_content`
- `propose_tasks`
- `propose_task_changes`
- `list_proposals`
- `get_proposal`

`get_tasks` renvoie seulement le chemin, la version et les tâches. Le contenu complet demande un appel explicite à `get_tasks_with_content` afin de limiter le contexte.

Le connecteur ne peut pas appliquer une proposition. L'application desktop reste responsable de l'afficher, puis d'enregistrer la décision humaine avec contrôle de version.

`propose_tasks` est le chemin court pour une IA qui ajoute plusieurs tâches : il accepte un nom ou un identifiant de projet, utilise le projet actif si le projet est omis, lit la version courante et crée une seule proposition. `propose_task_changes` reste disponible pour les modifications et suppressions ciblées.

## MCP

`taskhud-mcp` expose le même domaine via le protocole MCP `stdio`, pour les hôtes qui savent découvrir et appeler des outils MCP. Voir [MCP.md](MCP.md).
