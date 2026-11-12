# TaskHUD function connectors

The TaskHUD core exposes bounded domain operations. A connector can call them without getting general access to the file system.

## Local JSONL facade

`taskhud-functions` reads one function call per line on `stdin` and writes one JSON response per line on `stdout`:

```json
{"function":"get_tasks","arguments":{"projectId":"maths"}}
```

The available functions are:

- `list_projects`
- `get_active_project`
- `get_tasks`
- `get_tasks_with_content`
- `propose_tasks`
- `propose_task_changes`
- `list_proposals`
- `get_proposal`

`get_tasks` only returns the path, the version and the tasks. The full content requires an explicit call to `get_tasks_with_content`, to keep the context small.

The connector cannot apply a proposal. The desktop application stays responsible for showing it, then recording the human decision with a version check.

`propose_tasks` is the short path for an AI that adds several tasks: it accepts a project name or identifier, uses the active project when the project is omitted, reads the current version and creates a single proposal. `propose_task_changes` stays available for targeted updates and deletions.

## MCP

`taskhud-mcp` exposes the same domain over the MCP `stdio` protocol, for hosts that can discover and call MCP tools. See [MCP.md](MCP.md).
