# Studio-4T Roadmap

A high-level view of what's done and what's next. Studio-4T targets MongoDB today, with a
longer-term goal of supporting more databases.

## Done ✅

- **Foundation** — Tauri + Vue desktop app with a native system menu; async connection pool;
  a Rust backend with unit and integration tests; crash-safe atomic storage; Linux/WebKitGTK
  rendering and text-editing fixes.
- **Connections** — Connection Manager (create / edit / duplicate / import / export / delete);
  structured connection fields with a live Test Connection; passwords in the OS keychain;
  TLS/SSL; SSH tunnels with trust-on-first-use host-key verification; color tags; a connection
  tree with per-connection status.
- **Querying** — collection-bound tabs that persist across restarts; a query bar
  (filter / sort / projection / skip / limit); Find and aggregation modes; Explain; query
  history; save / load queries; a visual query builder; result paging and document counts.
- **Results & editing** — Table, JSON, and Tree views; a shell-code view; inline cell editing;
  nested drill-down; insert / replace / delete documents; copy helpers.
- **Collections, databases & indexes** — create / drop / rename collections; create / drop
  databases; index management; import / export to JSON and CSV.
- **Data tools** — GridFS; schema analysis; SQL → MQL translation; SQL migration; data masking;
  map-reduce; field restructuring; collection search and compare.
- **Admin & server** — server status / info, users & roles, profiler, current operations,
  validators, and background tasks.
- **Shell** — IntelliShell, an embedded mongosh-style JavaScript shell with a CodeMirror editor,
  Mongo-aware autocomplete, a persistent per-session context, and per-connection history.

## Planned 📋

- Light / dark theme toggle
- Keyboard-shortcut customization
- Save / open shell scripts
- Downloadable pre-built binaries
- Support for databases beyond MongoDB (longer term)
