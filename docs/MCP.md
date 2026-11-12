# TaskHUD MCP

TaskHUD provides a standalone local MCP server, `taskhud-mcp`, over `stdio`. It only reads the projects declared in a JSON file and stores proposals in `.taskhud/proposals.json` under the project concerned.

On purpose, the server offers no tool that applies a change directly. An agent can read the tasks and submit a diff, but applying that diff stays a human decision in TaskHUD.

## Local configuration

By default the server reads the registry written by the TaskHUD overlay: `%APPDATA%\TaskHUD\config.json` on Windows, `$XDG_CONFIG_HOME/TaskHUD/config.json` or `~/.config/TaskHUD/config.json` elsewhere. `--config <path>` overrides it, then the `TASKHUD_CONFIG` environment variable.

To declare projects by hand, create a `config.json` file:

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

The `root` path must point to an explicitly allowed project. TaskHUD only reads `TODO.md` there and writes proposals to `.taskhud/`. Add `.taskhud/` to the Git ignore file of the tracked project; the TaskHUD repository already does.

## Run the server

From an installed release:

```text
taskhud-mcp.exe --config C:\Users\you\AppData\Roaming\TaskHUD\config.json
```

From source:

```powershell
cargo run -p taskhud-mcp -- --config C:\Users\you\AppData\Roaming\TaskHUD\config.json
```

The server does not need the Tauri window to be open.

## Exposed tools

- `list_projects`
- `get_project_tasks`
- `get_active_project`
- `propose_task_changes`
- `list_proposals`
- `get_proposal`

`propose_task_changes` requires the SHA-256 version returned by `get_project_tasks`. An outdated version is rejected. Identical proposals that are still pending are deduplicated.

## MCP clients

Example of a generic `stdio` configuration:

```json
{
  "mcpServers": {
    "taskhud": {
      "command": "C:\\Users\\you\\AppData\\Local\\TaskHUD\\taskhud-mcp.exe",
      "args": [
        "--config",
        "C:\\Users\\you\\AppData\\Roaming\\TaskHUD\\config.json"
      ]
    }
  }
}
```

The exact paths and configuration locations depend on the client. The versioned smoke test checks MCP negotiation, discovery of the six tools, reading a project, persisting a proposal and the absence of any silent write to `TODO.md`. Full integration with Codex CLI and Claude Code is not declared validated until it has been run with their respective installations.
