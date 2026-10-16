# TaskHUD

TaskHUD is a small local desktop overlay for developers who work with AI agents. It keeps one readable `TODO.md` per project, lets a human edit it, and exposes agent suggestions as reviewable local proposals.

No account, cloud service, database, or HTTP server is required for the local workflow.

## Current status

The repository currently contains the V0.1 Tauri/Rust foundation and an independent `taskhud-mcp` stdio server. The desktop shell can load and save `TODO.md`; the MCP server can read registered projects and persist proposals without applying them.

This is not yet a packaged public release. VS Code workspace tracking, the final proposal UI, installers, and cross-platform builds are still in progress. See [STATE.md](STATE.md) for the operational checkpoint.

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

The MCP server can be run independently:

```powershell
cargo run -p taskhud-mcp -- --config C:\Users\you\AppData\Local\TaskHUD\config.json
```

See [docs/MCP.md](docs/MCP.md) for the local registry format and client configuration.

## QA

The acceptance contract is in [docs/SPEC.md](docs/SPEC.md). The versioned runner in `scripts/qa-runner.mjs` invokes Claude Code with a pinned Sonnet model and medium effort by default, records the tested commit and configuration, and never treats an unavailable review as a pass.

## License

TaskHUD is released under the MIT License. See [LICENSE](LICENSE).
