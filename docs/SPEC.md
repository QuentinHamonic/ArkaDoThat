# TaskHUD MVP specification

## Scope

This version validates a local, deterministic journey around a `TODO.md`. It does not yet promise the native Tauri window, VS Code detection, OS-level file watching or cross-platform support.

## Requirements and acceptance criteria

### T1 — Read a Markdown list

On opening, the user sees a task list built from a `TODO.md` text containing `- [ ]` and `- [x]` lines. Completed tasks are distinguished by more than color alone.

**Failure:** a valid line disappears, the checked state is inverted, or a task is shown as changed when it is not.

### T2 — Change a task's state

When the user activates a task's checkbox, its visible state changes immediately and the matching Markdown text is updated in memory.

**Failure:** only the appearance changes, or another task is changed.

### T3 — Add a task

From the main field, the user can type a task and add it with Enter or the add button. The field is cleared after success and the new task appears not done.

**Failure:** an empty entry is added, the action requires Markdown syntax, or keyboard focus is lost for no reason.

### T4 — Propose, then decide

An agent proposal clearly shows its type, its before/after text and the `Accept` and `Reject` actions. Accepting changes the list; rejecting leaves the list unchanged. A proposal must never be applied silently.

**Failure:** the diff is missing, the state changes before acceptance, or both decisions have the same effect.

### T5 — Source and local state

The interface shows which project or file is displayed and distinguishes the synchronized state from the locally modified state. A file chosen from the interface can be loaded when the browser file API is available; otherwise the demo mode stays explicit.

**Failure:** the user cannot tell which content they are looking at, or an unavailable capability is presented as persisted.

### T6 — Compact and accessible interface

The main journey fits in a small window, works at 320 px wide without horizontal scrolling, and has native keyboard controls, accessible labels and a visible focus. Secondary options must not clutter the main list.

**Failure:** a control unreachable by keyboard, an icon action without a name, text cut off with no alternative, or horizontal overflow.

## Expected checks

- `npm run check` must pass when the frontend dependencies are installed.
- The T1 to T4 journey must run in a headless browser against the local server.
- The rendering must be inspected at 320 px and 768 px wide.
- Limits tied to Tauri, Rust, VS Code, Wayland, macOS and Linux must stay reported as unverified until they are run on their environments.

## TODO/connectors slice additions

### T7 — Bounded connectors

MCP and JSONL function calls can read the project, explicitly request the full content and submit a versioned proposal. They cannot apply a change to `TODO.md` directly. A pending proposal is visible in the application and its human decision is persisted.

**Failure:** a connector writes directly to the file, returns the full content by default for no reason, or hides a proposal that has already been decided.

### T8 — Load and display sizes

The list stays usable with several hundred tasks. At 320, 768, 1024 and 1440 px wide, it creates no horizontal scrolling; vertical scrolling is contained in the list area.

**Check performed:** browser journey on a `TODO.md` with 600 tasks, plus toggle, add with Enter and acceptance of a proposal.

## Interface language slice

### T9 — Choose the interface language

The interface is in English by default. From the Settings page, the user can switch the interface between English and French. The change applies immediately to every visible text and accessible label, including the header title, without reloading or losing the current list. The choice survives a reload, and the document language (`<html lang>`) matches the displayed language. Task titles and file content are never translated.

**Failure:** some interface text or accessible label stays in the previous language, the choice is lost after a reload, `<html lang>` does not match the displayed language, the current tasks or unsaved changes are lost when switching, or unavailable local storage breaks the interface instead of keeping the choice for the current session only.
