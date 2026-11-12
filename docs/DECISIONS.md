# Architecture decisions

## 2026-10-16 — Local web MVP before Tauri

Rust/Cargo are not available in the initial environment. To get an observable slice without inventing a desktop build, the first increment uses Vite, Svelte and TypeScript in a local browser.

The Markdown core stays isolated in `src/lib/todo.ts`. The browser layer can load a file when the local API is available; the Tauri adapter will later replace this boundary without changing the domain contract.

## 2026-10-16 — Human source kept

`TODO.md` stays the reference format: it is readable, versionable and diffable. The interface state must not become the single source.

## 2026-10-16 — Never a silent proposal

An agent proposal is a pending intent. It is shown as a diff with two explicit decisions. The application does not treat the received text as a trusted instruction.

## Visual direction

The product draws on Markdown TODO extensions that keep the list human and diffable, and on small floating windows designed to stay visible without becoming a dashboard. In TaskHUD this becomes a compact graphite surface, an amber line reserved for the active signal and a distinct proposal area; no card grid and no heavy navigation.

## 2026-11-12 — English by default, French as an option

TaskHUD is going open source, so the code, documentation, connector messages and default interface are in English. The interface keeps a French dictionary selectable in Settings. Strings live in a small typed dictionary (`src/lib/i18n.ts`) with no i18n dependency: two languages and a few dozen strings do not justify one. Backend error messages and MCP descriptions stay English only, because they are read by developers and AI clients.
