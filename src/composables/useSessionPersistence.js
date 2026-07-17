import { watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Tab-session persistence. Persists open collection/shell tabs (and which one is active)
// so they return after a restart. Only the persistable fields are projected — result sets
// and other runtime state are rebuilt on demand, so paging through data never saves. The
// tab spine (`tabs`, `activeTabId`) stays owned by App.vue and is passed in;
// `runRestoredTab` re-runs a restored find tab's stored query in place.
export function useSessionPersistence({ tabs, activeTabId, runRestoredTab }) {
  function projectSession() {
    return {
      activeTabId: activeTabId.value,
      tabs: tabs.value
        .filter(t => t.kind === 'collection' || t.kind === 'shell')
        .map(t => t.kind === 'shell'
          ? {
              id: t.id, kind: 'shell', title: t.title, color: t.color,
              connectionId: t.connectionId, connectionName: t.connectionName,
              dbName: t.dbName, code: t.code, scriptPath: t.scriptPath || null,
            }
          : {
              id: t.id, kind: 'collection', title: t.title, color: t.color,
              connectionId: t.connectionId, connectionName: t.connectionName,
              dbName: t.dbName, collectionName: t.collectionName,
              filter: t.filter, sort: t.sort, projection: t.projection,
              skip: t.skip, limit: t.limit, mode: t.mode, pipeline: t.pipeline,
            }),
    }
  }

  let saveTabsTimer = null
  function scheduleSaveTabs() {
    clearTimeout(saveTabsTimer)
    saveTabsTimer = setTimeout(() => {
      invoke('set_open_tabs', { session: projectSession() }).catch(() => {})
    }, 400)
  }

  // Restore the previous session's tabs. Call before startAutoSave so the empty default
  // never overwrites tabs.json first.
  async function restoreSession() {
    try {
      const session = await invoke('get_open_tabs')
      const saved = session?.tabs
      if (saved?.length) {
        const conns = await invoke('list_connections')
        const validIds = new Set(conns.map(c => c.id))
        const restored = saved
          .filter(t => validIds.has(t.connectionId))    // drop tabs for deleted connections
          .map(t => t.kind === 'shell'
            ? {
                // Rebuild a shell tab with a fresh backend session (JS contexts are
                // ephemeral); the editor text is restored, history loads on mount.
                id: t.id, kind: 'shell', title: t.title, color: t.color,
                connectionId: t.connectionId, connectionName: t.connectionName,
                dbName: t.dbName,
                sessionId: (crypto.randomUUID ? crypto.randomUUID() : t.id),
                code: t.code || '', scriptPath: t.scriptPath || null, history: [], isRunning: false,
                results: [], resultView: 'table', resultTab: 'Console',
                runError: null, elapsedMs: null, drillPath: [], hasRun: false, selectedRow: -1, selectedRows: [],
                logs: [], scalar: undefined, hasScalar: false,
              }
            : {
                ...t,
                results: [], hasRun: false, isRunning: false, runError: null,
                selectedRow: -1, selectedRows: [], elapsedMs: null, _restored: true,
              })
        if (restored.length) {
          tabs.value.push(...restored)
          if (restored.some(t => t.id === session.activeTabId)) {
            activeTabId.value = session.activeTabId
          }
          // Lazily run the active restored tab (find mode re-runs its query).
          const active = tabs.value.find(x => x.id === activeTabId.value)
          if (active && active._restored) runRestoredTab(active)
        }
      }
    } catch (_) {}
  }

  // Save on any change to the open tabs or the active tab. The watched getter reads only
  // persistable fields, so result-set updates don't trigger it.
  function startAutoSave() {
    watch(() => JSON.stringify(projectSession()), scheduleSaveTabs)
  }

  return {
    restoreSession: restoreSession,
    startAutoSave: startAutoSave,
  }
}
