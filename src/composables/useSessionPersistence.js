import { watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Tab-session persistence. Persists open collection/shell tabs (and which one is active)
// so they return after a restart. Only the persistable fields are projected — result sets
// and other runtime state are rebuilt on demand, so paging through data never saves. The
// tab spine (`tabs`, `activeTabId`) stays owned by App.vue and is passed in;
// `runRestoredTab` re-runs a restored find tab's stored query in place.
export function useSessionPersistence({ tabs, activeTabId, runRestoredTab }) {
  // Import tabs persist their target + chosen format + the list of sources (each a
  // file path plus its target db/collection/insertion mode). Preview data is
  // re-derived on demand, so it isn't stored.
  function projectTab(t) {
    if (t.kind === 'shell') {
      return {
        id: t.id, kind: 'shell', title: t.title, color: t.color,
        connectionId: t.connectionId, connectionName: t.connectionName,
        dbName: t.dbName, code: t.code, scriptPath: t.scriptPath || null,
      }
    }
    if (t.kind === 'import') {
      const target = {
        id: t.id, kind: 'import', title: t.title, color: t.color,
        connId: t.connId, connName: t.connName,
        dbName: t.dbName, collName: t.collName,
        format: t.format,
      }
      if (t.format === 'csv') {
        // CSV: single source + options + target. The field mapping is re-derived
        // from the source preview, so it isn't stored.
        return {
          ...target,
          sourceType: t.sourceType, filePath: t.filePath || '',
          csv: {
            delimiter: t.csv.delimiter, other: t.csv.other,
            qualifier: t.csv.qualifier, skipLines: t.csv.skipLines, hasHeader: t.csv.hasHeader,
          },
          targetDb: t.targetDb, targetColl: t.targetColl, mode: t.mode,
        }
      }
      return {
        ...target,
        validate: !!t.validate,
        sources: (t.sources || []).map(s => ({
          path: s.path, name: s.name,
          targetDb: s.targetDb, targetColl: s.targetColl, mode: s.mode,
        })),
      }
    }
    return {
      id: t.id, kind: 'collection', title: t.title, color: t.color,
      connectionId: t.connectionId, connectionName: t.connectionName,
      dbName: t.dbName, collectionName: t.collectionName,
      filter: t.filter, sort: t.sort, projection: t.projection,
      skip: t.skip, limit: t.limit, mode: t.mode, pipeline: t.pipeline,
      vqb: t.vqb,
    }
  }

  function projectSession() {
    return {
      activeTabId: activeTabId.value,
      tabs: tabs.value
        .filter(t => t.kind === 'collection' || t.kind === 'shell' || t.kind === 'import')
        .map(projectTab),
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
          // drop tabs for deleted connections (import tabs key on connId)
          .filter(t => validIds.has(t.connectionId || t.connId))
          .map(t => t.kind === 'import'
            ? (t.format === 'csv'
              ? {
                  // CSV import tab. The field mapping is re-derived from the source
                  // preview (the referenced file may have changed), so start empty.
                  id: t.id, kind: 'import', title: t.title, color: t.color,
                  connId: t.connId, connName: t.connName,
                  dbName: t.dbName, collName: t.collName,
                  format: 'csv',
                  subTab: 'source',
                  sourceType: t.sourceType || 'file', filePath: t.filePath || '',
                  csv: {
                    delimiter: t.csv?.delimiter ?? ',', other: t.csv?.other ?? '',
                    qualifier: t.csv?.qualifier ?? '"', skipLines: t.csv?.skipLines ?? 0,
                    hasHeader: t.csv?.hasHeader ?? true,
                  },
                  targetDb: t.targetDb, targetColl: t.targetColl, mode: t.mode || 'insert',
                  fields: [],
                }
              : {
                  // JSON import tab: restore its sources; preview is re-derived on demand.
                  id: t.id, kind: 'import', title: t.title, color: t.color,
                  connId: t.connId, connName: t.connName,
                  dbName: t.dbName, collName: t.collName,
                  format: t.format, validate: !!t.validate,
                  sources: (t.sources || []).map(s => ({
                    path: s.path, name: s.name,
                    targetDb: s.targetDb, targetColl: s.targetColl, mode: s.mode,
                  })),
                  selectedSource: (t.sources && t.sources.length) ? 0 : -1,
                  previewOpen: false,
                })
            : t.kind === 'shell'
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
