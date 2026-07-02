// Pure logic for the native menu's enable/disable and action targeting. Extracted
// from App.vue so it can be unit-tested (see menuContext.test.js).
//
// The key fix these encode: the menu context is the UNION of the active tab and
// the sidebar/tree selection, so items enable when the user selects a
// connection/database/collection in the tree — not only when a matching tab is
// active (which at launch is always the context-less Quickstart tab).

// Which item groups should be enabled, given the active tab, the current sidebar
// selection, and how many connections are open.
//   activeTab      { connectionId, dbName, collectionName, kind } | null
//   treeSelection  { connectionId, dbName, collectionName, kind } | null
//   connectionCount  number of connections open in the tree
export function deriveMenuContext(activeTab, treeSelection, connectionCount) {
  const tab = activeTab || null
  const sel = treeSelection || null
  const tabConnection = !!(tab && tab.connectionId)
  return {
    hasConnection: tabConnection || !!(sel && sel.connectionId),
    hasDatabase: !!(tab && tab.connectionId && tab.dbName) || !!(sel && sel.dbName),
    hasCollection: !!(tab && tab.kind === 'collection' && tab.collectionName) || !!(sel && sel.collectionName),
    // Refresh acts on every open connection, so it enables whenever one exists.
    anyConnection: (connectionCount || 0) > 0 || tabConnection,
  }
}

// The node a native menu action should act on: the sidebar selection when there is
// one (that's what the user just clicked in the tree), otherwise the active tab.
// Returns a tab-shaped object so it drops straight into the existing handlers.
export function resolveMenuTarget(activeTab, treeSelection) {
  const sel = treeSelection || null
  if (sel && sel.connectionId) {
    return {
      connectionId: sel.connectionId,
      connectionName: sel.connectionName,
      dbName: sel.dbName || null,
      collectionName: sel.collectionName || null,
      kind: sel.collectionName ? 'collection' : (sel.dbName ? 'database' : 'connection'),
    }
  }
  return activeTab || null
}
