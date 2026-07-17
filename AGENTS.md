# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

1. Ask, don't assume. If something is unclear, ask before writing a single line. Never make silent assumptions about intent, architecture, or requirements. When running unattended, pick the most reasonable interpretation, proceed, and record the assumption rather than blocking.

2. Implement the simplest solution for simple problems, better solutions for harder problems. Do not over-engineer or add flexibility that isn't needed yet. 

3. Don't touch unrelated code but please do surface bad code or design smells you discover with me so we can address them as a separate issue.

4. Flag uncertainty explicitly. If you're unsure about something, see point 1 above. If it makes sense to do so, conduct a small, localised and low-risk experiment and bring the hypothesis and results to me to discuss. Confidence without certainty causes more damage than admitting a gap.

5. I'm always open to ideas on better ways to do things. Please don't hesitate to suggest a better way, or one that has long lasting impact over a tactical change. (as a few examples)

# OzenDB — Claude Guidelines

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

# Run frontend unit tests (Vitest; specs live next to sources, e.g. src/utils/*.test.js)
npm test
```

---

## Architecture

**Stack:** Tauri 2 (Rust backend) + Vue 3 (frontend, Vite). No router, no Pinia — plain `ref`/`computed`.

### Data flow

```
App.vue  (holds the tab/pane spine: tabs[], activeTabId, split-pane sizing)
  ├── Toolbar.vue          (global toolbar actions)
  ├── ConnectionTree.vue   (sidebar; calls list_connections, list_databases on mount/expand)
  ├── QueryWorkspace.vue   (tabs + query UI; emits run-query → App.vue calls find_documents)
  ├── OperationsPane.vue   (surface for long-running operations)
  ├── AppModals.vue        (renders every top-level modal, incl. ConnectionManager → NewConnection)
  └── ContextMenu.vue      (handled entirely in App.vue's handleContextAction)
```

Most app state and logic now live in `src/composables/*` (`useTabs`, `useModals`, `useQueryRunner`, `useDbActions`, `useMenu`, `useOperations`, `useSessionPersistence`, …) — `useModals` owns the open-state for every modal. App.vue composes these and passes props/handlers down; treat the composable as the source of truth for its slice. The tab/pane spine is intentionally kept in App.vue.

**Tab state** lives in `App.vue`'s `tabs` ref as plain objects. Child components mutate tab properties directly (e.g. `tab.filter`, `tab.skip`) — this works because Vue 3 makes array items reactive. `QueryWorkspace` receives `tabs` as a prop and reads `activeTab` via a computed, then emits `run-query` up to App.vue which calls `invoke('find_documents', ...)`.

### Rust backend (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `commands/` | All `#[tauri::command]` functions, split by area (`query`, `admin`, `connection`, `schema`, `sql`, `masking`, `migration`, `gridfs`, `compare`, `stats`, …) and re-exported from `commands/mod.rs`. `mod.rs` also holds shared helpers — notably `client_for(pool, storage, id)`, the single entry point every command uses to resolve a connection to a live client, plus the EJSON/CSV parse helpers. |
| `pool.rs` | `ConnectionPool`: one `Client` per connection id behind a `tokio::Mutex` (and the live `SshTunnel` for tunnelled connections). `connect()` returns the cached client on a hit and only reads the keychain / builds the URI on a miss. |
| `storage/mod.rs` | JSON persistence for `ConnectionConfig` (`connections.json`). Read-modify-write goes through the locked `update_with`; the raw `save` is private so writes can't bypass the lock. Other JSON stores (`folders`, `history`, `saved_queries`, `default_queries`, `settings`, `tabs`, `shell_history`, `known_hosts`) share the same shape via the generic `JsonStore<T>` in `json_store.rs`. |
| `persist.rs` | `atomic_write()` — write-to-temp-then-rename so a crash can't leave a truncated file. Shared by every JSON store. |
| `keychain.rs` | Secrets (passwords, SSH key passphrases) in the OS keychain, keyed by connection id (SSH secrets under `id::ssh-*`). Configs on disk are credential-free. |
| `ssh.rs` / `known_hosts/mod.rs` | Optional SSH tunnel (pure-Rust `russh`) with trust-on-first-use host-key verification: unchanged key accepted, new host prompts, changed key refused. |
| `shell/` | Embedded JS shell ("IntelliShell"): `engine.rs` runs one `boa` context per session on its own worker thread; `bridge/mod.rs` exposes the `db` object that forwards to the driver. |
| `uri/mod.rs` | `build_uri()` assembles the connection string from a config; `with_timeout()` appends MongoDB timeout params; `tcp_probe()` does a fast TCP check before the MongoDB handshake. |
| `error.rs` | `AppError` enum serialized as `{ code, message }` so the frontend gets a stable category plus a human-readable message. |
| `menu.rs` | Native OS menu (source of truth); File → Connect opens a **second Tauri webview window** at `src/pages/connect.html`. See "Native menu" below. |

### Native menu

The app menu is the **native OS menu**, built entirely in `src-tauri/src/menu.rs` (macOS
system menu bar with the standard application menu + ⌘ accelerators; native in-window menu on
Windows/Linux). There is no in-window Vue menu bar — the old `src/components/Menubar.vue` was
removed.

- **Structure** is a data table (`menus()`): each item has an id, label, optional accelerator, and
  an optional `Gate` (`Connection` / `Database` / `Collection` / `AnyConnection`). Placeholders are
  the `built:false` features — carried over as present-but-disabled items.
- **Clicks** → `handle_event` emits `menu-action` with the item id → `App.vue` listens and routes
  through the existing `handleMenuAction` (same handlers the toolbar/right-click use). Actions are
  never reimplemented in Rust.
- **Enable/disable** reflects the current selection, which is the UNION of the active tab **and the
  sidebar/tree selection** (`ConnectionTree` emits `select-node` / `connections-changed`). The
  frontend `menuContext` (see `src/utils/menuContext.js`, unit-tested) is pushed to Rust via the
  `set_menu_context` command, which flips each gated item's `enabled`. Menu actions resolve their
  target via `resolveMenuTarget`, which is level-aware: it picks whichever of the sidebar selection
  or active tab actually satisfies the action's required depth (`connection`/`database`/`collection`),
  with the sidebar selection winning when both qualify and the active tab used as fallback when the
  selection is too shallow — so an enabled item always fires on a node deep enough for the gate that
  lit it up.
- **Accelerators** are attached on macOS/Windows only. On Linux they're omitted (WebKitGTK swallows
  editing keys) and `App.vue`'s `onGlobalKeydown` keeps the JS shortcuts instead — gated by
  `NATIVE_MENU_OWNS_SHORTCUTS`.
- The gate→enabled derivation is unit-tested in `menu.rs` (`cargo test`) and `menuContext.test.js`
  (`npm test`).

### Design conventions

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
- **Never write long and verbose, detailed git commit messages, just include high-level overview of what has been done
