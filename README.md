# TaskHUD

TaskHUD is a small local desktop overlay for developers who work with AI agents. It keeps one readable `TODO.md` per project, lets a human edit it, and exposes agent suggestions as reviewable local proposals.

No account, cloud service, database, or HTTP server is required for the local workflow.

## Current status

The repository currently contains the V0.1 Tauri/Rust foundation, a working manual TODO flow, and two agent-facing connectors. `taskhud-mcp` exposes the domain through MCP `stdio`; `taskhud-functions` exposes the same bounded operations as compact JSONL function calls. Both can read tasks and persist proposals, but neither can apply a change silently. The desktop bridge loads pending proposals, shows their diff, and accepts or rejects them with a version check.

This is not yet a packaged public release. VS Code workspace tracking, installers, cross-platform builds, and the complete native file-dialog/proposal journey are still open. The native WebView manual controls and Tauri commands have been checked. See [STATE.md](STATE.md) for the operational checkpoint.

## Local development

Requirements: Node.js, Rust with the MSVC toolchain on Windows, and the platform prerequisites documented by Tauri.

```powershell
npm install
npm run check
$env:Path = "C:\Users\you\.cargo\bin;$env:Path"
cargo test -p taskhud-core
cargo check --workspace
npx tauri dev
```

Pour générer l'application Windows et ses installateurs :

```powershell
.\build-desktop.cmd
```

Le binaire brut est généré dans `target\release\taskhud.exe`. Les installateurs Tauri sont dans `target\release\bundle\` (notamment les formats `.exe` et `.msi` lorsque les outils Windows correspondants sont disponibles). Cette commande ne publie rien et ne nécessite aucun compte.

The MCP server can be run independently:

```powershell
cargo run -p taskhud-mcp -- --config C:\Users\you\AppData\Local\TaskHUD\config.json
```

See [docs/MCP.md](docs/MCP.md) for the local registry format and client configuration.

The provider-neutral function facade can be run as a JSONL process:

```powershell
cargo run -p taskhud-functions -- --config C:\Users\you\AppData\Local\TaskHUD\config.json
```

See [docs/CONNECTORS.md](docs/CONNECTORS.md) for the compact function contract.

## QA

The acceptance contract is in [docs/SPEC.md](docs/SPEC.md). The versioned runner in `scripts/qa-runner.mjs` invokes Claude Code with a pinned Sonnet model and medium effort by default, records the tested commit and configuration, and never treats an unavailable review as a pass.

## License

TaskHUD is released under the MIT License. See [LICENSE](LICENSE).
