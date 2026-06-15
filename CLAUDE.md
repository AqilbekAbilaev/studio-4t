# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Studio-4T — Claude Guidelines

## Rust

- Never use shorthands. Always write out field names explicitly, even when the variable name matches the field name.

  ```rust
  // BAD
  Foo { x, y }

  // GOOD
  Foo { x: x, y: y }
  ```

- Never use the `?` operator. Always expand it to an explicit `match` block.

  ```rust
  // BAD
  let val = some_result?;

  // GOOD
  let val = match some_result {
      Ok(val) => val,
      Err(e) => return Err(e.into()),
  };
  ```

  For `Option`-returning functions:
  ```rust
  // BAD
  let val = some_option?;

  // GOOD
  let val = match some_option {
      Some(val) => val,
      None => return None,
  };
  ```

## Commands

```bash
# Run the full app (Vite dev server + Tauri shell)
npm run tauri dev

# Verify Rust compiles after any backend change
cd src-tauri && cargo build

# Run Rust unit tests
cd src-tauri && cargo test

# Frontend-only Vite dev server (no Tauri; invoke() calls won't work)
npm run dev
```

There are no frontend tests — Vitest/Jest are not configured.

---

## Architecture

**Stack:** Tauri 2 (Rust backend) + Vue 3 (frontend, Vite). No router, no Pinia — plain `ref`/`computed`.

### Data flow

```
App.vue  (owns all app state: tabs[], showConnectionManager, etc.)
  ├── ConnectionTree.vue   (sidebar; calls list_connections, list_databases on mount/expand)
  ├── QueryWorkspace.vue   (tabs + query UI; emits run-query → App.vue calls find_documents)
  ├── ConnectionManager.vue (modal; calls list_connections, delete_connection, update_last_accessed)
  │     └── NewConnection.vue (calls test_connection, save_connection, set_connection_tag)
  └── ContextMenu.vue      (handled entirely in App.vue's handleContextAction)
```

**Tab state** lives in `App.vue`'s `tabs` ref as plain objects. Child components mutate tab properties directly (e.g. `tab.filter`, `tab.skip`) — this works because Vue 3 makes array items reactive. `QueryWorkspace` receives `tabs` as a prop and reads `activeTab` via a computed, then emits `run-query` up to App.vue which calls `invoke('find_documents', ...)`.

### Rust backend (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `commands.rs` | All `#[tauri::command]` functions wired into the invoke handler |
| `pool.rs` | `ConnectionPool`: `HashMap<id, Client>` behind a `tokio::Mutex`; `get_or_create` avoids holding the lock during network I/O |
| `storage.rs` | JSON file persistence for `ConnectionConfig` at the OS app-data dir (`connections.json`) |
| `uri.rs` | `with_timeout()` appends MongoDB timeout params; `tcp_probe()` does a fast TCP check before the MongoDB handshake |
| `error.rs` | `AppError` enum serialized as a plain string so the frontend receives a human-readable message |
| `menu.rs` | Native system menu; File → Connect opens a **second Tauri webview window** at `src/pages/connect.html` |

### Design reference

`ui-design/` contains the authoritative design prototype (React/JSX, browser-runnable). Before implementing any new screen or component, read the relevant JSX file and `ui-design/design_handoff_studio4t/README.md` for exact spacing, colors, and interaction spec.

Key rules from the handoff:
- **Dialog headers must not have macOS traffic lights.** Only the real OS window gets them. Dialogs use a centered title + a single close ✕ button on the right.
- All colors come from CSS custom properties in `src/assets/theme.css` — never hardcode hex values that already exist as tokens.
- Icons are inline SVG rendered by `BaseIcon.vue` via a `name` prop — add new icons there, never use external icon fonts or raster images.

---

## Workflow

This project is human-delivered, AI-developed. The human must stay in full control of what ships.

- **One logical change per session.** Never bundle unrelated changes into a single response. If a task touches more than ~3 files, split it into steps and confirm with the user between each step.
- **Explain before committing.** Always describe what changed and why in plain language before reporting the work as done. No code jargon — write as if explaining to someone who will review the diff.
- **Never mix refactoring with bug fixes.** Each commit must have a single concern. If a bug fix requires a refactor, do them in separate steps.
- **Always verify the build compiles** after any Rust change before reporting done. Run `cargo build` inside `src-tauri/` and confirm it succeeds.
- **Let the user commit.** Do not create git commits unless explicitly asked. Explain the change, then wait.
