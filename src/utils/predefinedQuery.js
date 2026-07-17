// Predefined-query presets for the Update / Delete dialogs' "Apply predefined query"
// dropdown. Each returns query text in the shell dialect parseField accepts, so the
// generated text can be edited and re-parsed like anything the user types:
//   'all'      → every document ({})
//   'current'  → the query currently in the collection view (the tab's filter)
//   'selected' → the rows selected in the results grid, matched by _id
import { mongoStringify } from './mongoFormat'

// Indices of the current row selection (multi-select), falling back to the single
// active row. Empty when nothing is selected. Mirrors the same helper in
// useDocumentActions (kept separate so the dialogs depend only on this util).
export function selectedRowIndices(tab) {
  if (!tab) return []
  if (tab.selectedRows && tab.selectedRows.length) return tab.selectedRows
  return (tab.selectedRow ?? -1) >= 0 ? [tab.selectedRow] : []
}

// Whether "Selected Document(s)" has anything to act on (gates the dropdown option).
export function hasSelectedDocs(tab) {
  return selectedRowIndices(tab).length > 0
}

export function predefinedQuery(kind, tab) {
  if (kind === 'current') {
    const filter = (tab && tab.filter ? tab.filter : '').trim()
    return filter || '{}'
  }
  if (kind === 'selected') {
    const ids = selectedRowIndices(tab)
      .map((i) => (tab && tab.results ? tab.results[i] : null))
      .filter((doc) => doc != null)
      .map((doc) => doc._id)
    // mongoStringify renders EJSON _id wrappers as shell types (ObjectId("…") etc.),
    // which parseField reads straight back.
    return mongoStringify({ _id: { $in: ids } })
  }
  return '{}' // 'all' and any unknown kind
}
