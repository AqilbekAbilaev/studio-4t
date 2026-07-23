// Global toolbar buttons, left→right. `sep: true` renders a divider instead of a button;
// `badge` is a small dot colour on the icon; `drop` shows a caret. `name` is both the
// BaseIcon icon name and the action id handed to App.vue's handleTool dispatcher.
export const TOOLS = [
  { name: 'connect',   label: 'Connect',      drop: true },
  { name: 'collection',label: 'Collection' },
  { name: 'shell',     label: 'IntelliShell' },
  { name: 'sql',       label: 'SQL' },
  { name: 'aggregate', label: 'Aggregate' },
  { name: 'search',    label: 'Search in…' },
  { sep: true },
  { name: 'compare',   label: 'Compare' },
  { name: 'schema',    label: 'Schema' },
  { name: 'reschema',  label: 'Reschema' },
  { name: 'tasks',     label: 'Tasks' },
  { sep: true },
  { name: 'export',    label: 'Export' },
  { name: 'import',    label: 'Import' },
  { name: 'mask',      label: 'Data Masking' },
  { name: 'migration', label: 'SQL Migration', drop: true },
]
