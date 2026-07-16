import { watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Tab-session persistence. Persists open collection/shell tabs (and which one is active,
// plus any two-pane split) so they return after a restart. Only the persistable fields are
// projected — result sets and other runtime state are rebuilt on demand, so paging through
// data never saves. The tab/pane spine (`tabs`, `panes`, …) stays owned by App.vue and is
// passed in; `runRestoredTab` re-runs a restored find tab's stored query in place.
export function useSessionPersistence({ tabs, panes, splitOrientation, focusedPaneId, activeTabId, runRestoredTab }) {
  function projectSession() {
    return {
      activeTabId: activeTabId.value,
      panes: panes.value.map(p => ({ id: p.id, activeTabId: p.activeTabId })),
      splitOrientation: splitOrientation.value,
      focusedPaneId: focusedPaneId.value,
      tabs: tabs.value
        .filter(t => t.kind === 'collection' || t.kind === 'shell')
        .map(t => t.kind === 'shell'
          ? {
              id: t.id, kind: 'shell', title: t.title, color: t.color,
              paneId: t.paneId || 'p0',
              connectionId: t.connectionId, connectionName: t.connectionName,
              dbName: t.dbName, code: t.code, scriptPath: t.scriptPath || null,
            }
          : {
              id: t.id, kind: 'collection', title: t.title, color: t.color,
              paneId: t.paneId || 'p0',
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
                paneId: t.paneId || 'p0',
                connectionId: t.connectionId, connectionName: t.connectionName,
                dbName: t.dbName,
                sessionId: (crypto.randomUUID ? crypto.randomUUID() : t.id),
                code: t.code || '', scriptPath: t.scriptPath || null, history: [], isRunning: false,
                results: [], resultView: 'table', resultTab: 'Console',
                runError: null, elapsedMs: null, drillPath: [], hasRun: false, selectedRow: -1,
                logs: [], scalar: undefined, hasScalar: false,
              }
            : {
                ...t,
                results: [], hasRun: false, isRunning: false, runError: null,
                selectedRow: -1, elapsedMs: null, _restored: true,
              })
        if (restored.length) {
          tabs.value.push(...restored)
          if (restored.some(t => t.id === session.activeTabId)) {
            activeTabId.value = session.activeTabId
          }
          // Restore a saved two-pane split when both panes still point at live tabs.
          if (session.splitOrientation && Array.isArray(session.panes) && session.panes.length === 2
              && session.panes.every(p => tabs.value.some(t => t.id === p.activeTabId))) {
            panes.value = session.panes.map((p, i) => ({ id: 'p' + i, activeTabId: p.activeTabId }))
            splitOrientation.value = session.splitOrientation === 'horizontal' ? 'horizontal' : 'vertical'
            focusedPaneId.value = panes.value.some(p => p.id === session.focusedPaneId) ? session.focusedPaneId : 'p0'
          } else {
            // No split restored — collapse every tab into the single pane so a tab saved
            // as p1 doesn't end up orphaned with no pane to show it.
            for (const t of tabs.value) t.paneId = 'p0'
          }
          // Lazily run each pane's active restored tab (find mode re-runs its query).
          for (const pane of panes.value) {
            const t = tabs.value.find(x => x.id === pane.activeTabId)
            if (t && t._restored) runRestoredTab(t)
          }
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
