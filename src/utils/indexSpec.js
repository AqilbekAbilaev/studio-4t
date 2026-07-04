// Pure helpers for the Index menu features (selection, copy, display). Kept out of
// App.vue so they can be unit-tested (see indexSpec.test.js). An "index" here is one
// of the raw documents returned by the `list_indexes` command, e.g.
//   { v: 2, key: { email: 1 }, name: "email_1", unique: true, hidden: true }

// The `_id_` index is created and required by MongoDB; it can never be dropped or
// hidden. Mirrors the backend's guard so the UI refuses these actions up front.
export function isProtectedIndex(name) {
  return name === '_id_'
}

// A compact, human-readable rendering of an index's key spec, e.g.
// `{ email: 1 }` → "email: 1". Empty/malformed specs render as an empty string.
export function indexKeyLabel(index) {
  if (!index || !index.key || typeof index.key !== 'object') return ''
  return Object.entries(index.key).map(([key, dir]) => `${key}: ${dir}`).join(', ')
}

// The clipboard payload for "Copy Index": the full index definition as pretty
// JSON. This is the spec as MongoDB reports it, so it round-trips for reference.
export function indexSpecJson(index) {
  return JSON.stringify(index ?? {}, null, 2)
}

// Whether an index is currently hidden from the query planner.
export function isIndexHidden(index) {
  return !!(index && index.hidden)
}
