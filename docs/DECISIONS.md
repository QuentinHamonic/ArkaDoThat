# Architecture decisions

## 2026-10-16 — Local web MVP before Tauri

Rust/Cargo are not available in the initial environment. To get an observable slice without inventing a desktop build, the first increment uses Vite, Svelte and TypeScript in a local browser.

The Markdown core stays isolated in `src/lib/todo.ts`. The browser layer can load a file when the local API is available; the Tauri adapter will later replace this boundary without changing the domain contract.

## 2026-10-16 — Human source kept

`TODO.md` stays the reference format: it is readable, versionable and diffable. The interface state must not become the single source.

## 2026-10-16 — Never a silent proposal

An agent proposal is a pending intent. It is shown as a diff with two explicit decisions. The application does not treat the received text as a trusted instruction.

## Visual direction

The product draws on Markdown TODO extensions that keep the list human and diffable, and on small floating windows designed to stay visible without becoming a dashboard. In ArkaDoThat this becomes a compact graphite surface, an amber line reserved for the active signal and a distinct proposal area; no card grid and no heavy navigation.

## 2026-11-12 — English by default, French as an option

ArkaDoThat is going open source, so the code, documentation, connector messages and default interface are in English. The interface keeps a French dictionary selectable in Settings. Strings live in a small typed dictionary (`src/lib/i18n.ts`) with no i18n dependency: two languages and a few dozen strings do not justify one. Backend error messages and MCP descriptions stay English only, because they are read by developers and AI clients.

## 2026-11-12 — One task syntax for the core and the webview

A task line starts, after optional spaces or tabs, with `- [ ]`, `- [x]` or `- [X]`, followed by at least one space or tab and a non-empty title (trimmed). Anything else, such as `- [ ]` in the middle of a line, `-[ ] title` or an empty `- [ ]`, stays plain text. The Rust core (`parse_task_line`) and the webview (`src/lib/todo.ts`) both test this rule against the same table, `crates/taskhud-core/fixtures/task-syntax.json`, so their task IDs and line numbers cannot drift apart.

## 2026-11-27 — GPL-3.0-or-later as the project license

The repository uses the GNU General Public License version 3 or any later version. The license file, README, package metadata and Rust manifests must keep this choice aligned.

## 2026-11-27 — `dev` is the integration branch

The public repository remains maintainer-led while development is private in practice. `dev` is the integration branch and the default branch for now. A future stable `main` branch will be introduced with the first packaged release. Feature, fix, documentation and release work reaches those branches through pull requests.

## 2026-11-27 — Solo review is a checklist gate

GitHub does not count an author's own approval as an independent review. While ArkaDoThat has one maintainer, every change still goes through a pull request, green CI, a completed self-review checklist and an explicit merge decision. External contributions are not open by default. When an independent maintainer is explicitly added, the branch protection policy can require one approving review.
