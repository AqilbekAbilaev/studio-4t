# Changelog

## v0.1.2

- **Declarative modal registry** — all top-level modals now render from a
  single registry, making each modal a lazy-loaded component with no wiring
  beyond one row of config. GridFS, export/import wizards, validator, masking,
  and reschema modals all moved into the registry.
- **Workspace tabs for tools** — SQL, Schema, Masking, Reschema, Compare,
  Tasks, and Search now open as workspace tabs instead of modals, giving them
  tab navigation, persistence, and a full-width working area.
- **Search redesigned** — new results grid with scope controls, match-case,
  regex toggles, and database/collection pickers.
- **SQL as per-collection tab** — SQL opens as a collection tab in `sql`
  mode, reusing the full result stack (grid, paging, Query Code, Explain).
  Backed by the `sqlparser` crate instead of a hand-rolled parser.
- **Escape-to-close everywhere** — all modals consistently close on Escape
  via BaseModal. Pop-out document windows also close on Escape.
- **Toast via provide/inject** — `showToast` is now provided app-wide instead
  of being bubbled through events, simplifying every component that needs it.
- **Linux Tab shortcut fix** — Ctrl+Tab / Ctrl+Shift+Tab now work on Linux
  (WebKitGTK was reporting Shift+Tab as Unidentified).
- **Data import** — Studio-3T-style CSV import with configurable parsing
  options, plus a JSON import tab and an import-format picker.
- **Unified component library** — every button, input, select, textarea,
  checkbox, radio, modal, and form field now routes through a shared base
  component, giving the app a consistent look and feel.
- **3T-style update/delete dialogs** — tabs, upsert/multi toggles, JSON
  validation, and a predefined-query selector, matching the shape users
  expect from Studio 3T.
- **Find-in-results bar** — search across table, JSON, and tree result views
  without scrolling.
- **Multi-row selection in table view** — shift-click ranges, ctrl-click
  disjoint rows, ctrl+a select-all, bulk copy and delete.
- **Index management tab** — full index create/drop workflow in a dedicated
  tab, with per-collection state preserved across sessions. Index operations
  are tracked in the Operations pane.
- **Operations pane** — long-running exports, imports, and index operations
  surface progress in a dedicated panel.
- **Quickstart tab** — interactive landing page with recent connections,
  common tasks, options, and help links.
- **Tab navigation** — Ctrl+Tab / Ctrl+Shift+Tab to cycle tabs. Index Manager
  and VQB state persist across restarts.
- **Pop-out document insert** — new documents open in a separate window
  instead of a modal.
- **Better error surfacing** — real MongoDB write-error messages (e.g.
  duplicate key) now reach the UI instead of being swallowed.
- **Linux desktop integration** — correct `StartupWMClass` and `Categories`
  so the app icon appears in the dock and task switcher.
- **CI** — pull-request test workflow runs `cargo test` and `vitest` on every
  PR.

## v0.1.1

- **Reduced memory usage after large queries** — switched to the mimalloc
  allocator and stream query results as pre-serialized JSON, cutting retained
  memory after a big fetch by roughly two-thirds.
