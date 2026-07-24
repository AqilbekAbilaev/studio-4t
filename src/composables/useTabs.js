import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Tab manipulation: activate/close/cycle/duplicate/reorder/rename + the tab
// context-menu handler. The tab *state* (`tabs`, `activeTabId`) is the app spine
// and stays in App.vue — it's read by session restore/onMounted and injected into
// several composables — so it's passed IN here rather than owned. The tab
// *creators* (openCollectionTab/openShellTab/…) also stay in App.vue because they
// depend on the query runner and settings; those are the only query-coupled bits.
// `runRestoredTab` is injected so re-activating/duplicating a restored tab re-runs
// its saved query without this composable importing the query runner.
export function useTabs({ tabs, activeTabId, contextMenu, runRestoredTab }) {
  // ── rename tab dialog ──
  const renameTabTarget = ref(null)   // id of the tab being renamed
  const renameTabValue = ref('')

  function activateTab(id) {
    activeTabId.value = id
    const tab = tabs.value.find(t => t.id === id)
    if (tab && tab._restored) runRestoredTab(tab)
  }

  // Move the active-tab selection by `delta` (+1 next, -1 previous), wrapping around.
  // No-ops when fewer than two tabs are open.
  function cycleTab(delta) {
    if (tabs.value.length < 2) return
    const idx = tabs.value.findIndex(t => t.id === activeTabId.value)
    if (idx < 0) {
      activateTab(tabs.value[0].id)
      return
    }
    const next = (idx + delta + tabs.value.length) % tabs.value.length
    activateTab(tabs.value[next].id)
  }

  function closeTab(id) {
    const idx = tabs.value.findIndex(t => t.id === id)
    if (idx < 0) return
    const closing = tabs.value[idx]
    if (closing.kind === 'shell' && closing.sessionId) {
      invoke('close_shell_session', { sessionId: closing.sessionId }).catch(() => {})
    }
    tabs.value.splice(idx, 1)
    // If we closed the active tab, move to an adjacent one (the nearest preceding
    // tab, else the new first tab).
    if (activeTabId.value === id) {
      const next = tabs.value[idx - 1] || tabs.value[0]
      activeTabId.value = next ? next.id : null
    }
  }

  // closeTab reindexes each call, so iterate over a snapshot of the target ids.
  function closeTabsExcept(tabId) {
    tabs.value.filter(t => t.id !== tabId).map(t => t.id).forEach(closeTab)
  }
  function closeTabsToSide(tabId, side) {
    const idx = tabs.value.findIndex(t => t.id === tabId)
    if (idx < 0) return
    const victims = side === 'left' ? tabs.value.slice(0, idx) : tabs.value.slice(idx + 1)
    victims.map(t => t.id).forEach(closeTab)
  }
  function closeAllTabs() {
    tabs.value.map(t => t.id).forEach(closeTab)
  }
  function moveTabToFront(tabId) {
    const idx = tabs.value.findIndex(t => t.id === tabId)
    if (idx <= 0) return
    const [tab] = tabs.value.splice(idx, 1)
    tabs.value.unshift(tab)
  }
  function duplicateTab(tabId) {
    const src = tabs.value.find(t => t.id === tabId)
    if (!src) return
    const id = 't' + Date.now()
    if (src.kind === 'shell') {
      tabs.value.push({
        id: id, kind: 'shell', title: src.title,
        connectionId: src.connectionId, connectionName: src.connectionName,
        dbName: src.dbName,
        sessionId: (crypto.randomUUID ? crypto.randomUUID() : id),
        code: src.code || '', history: [], isRunning: false,
        results: [], resultView: 'table', resultTab: 'Console',
        runError: null, elapsedMs: null, drillPath: [], hasRun: false, selectedRow: -1, selectedRows: [],
        logs: [], scalar: undefined, hasScalar: false,
        color: src.color ?? null,
      })
      activeTabId.value = id
      return
    }
    const dup = {
      id: id, kind: 'collection', title: src.title,
      connectionId: src.connectionId, connectionName: src.connectionName,
      dbName: src.dbName, collectionName: src.collectionName,
      filter: src.filter, projection: src.projection, sort: src.sort,
      skip: src.skip, limit: src.limit, mode: src.mode, pipeline: src.pipeline,
      color: src.color ?? null,
      colOrder: src.colOrder || {},
      results: [], hasRun: false, isRunning: false, runError: null,
      selectedRow: -1, selectedRows: [], elapsedMs: null,
    }
    tabs.value.push(dup)
    activeTabId.value = id
    runRestoredTab(dup)   // re-run from the cloned query state (find mode only)
  }

  function openRenameTab(tabId) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (!tab) return
    renameTabTarget.value = tabId
    renameTabValue.value = tab.title || ''
  }
  function confirmRenameTab() {
    const tab = tabs.value.find(t => t.id === renameTabTarget.value)
    const name = renameTabValue.value.trim()
    if (tab && name) tab.title = name
    renameTabTarget.value = null
  }

  // ── tab context menu ──
  function onTabContext({ id, x, y }) {
    contextMenu.value = { type: 'tab', x: x, y: y, nodeData: { tabId: id } }
  }

  function handleTabAction(action, tabId) {
    if (action.startsWith('Choose Color:')) {
      const color = action.split(':')[1]
      const tab = tabs.value.find(t => t.id === tabId)
      if (tab) tab.color = color === 'none' ? null : color
      return
    }
    switch (action) {
      case 'Close Tab':               closeTab(tabId); break
      case 'Close Other Tabs':        closeTabsExcept(tabId); break
      case 'Close Tabs to the Left':  closeTabsToSide(tabId, 'left'); break
      case 'Close Tabs to the Right': closeTabsToSide(tabId, 'right'); break
      case 'Close All Tabs':          closeAllTabs(); break
      case 'Duplicate Tab':           duplicateTab(tabId); break
      case 'Move Tab to the Front':   moveTabToFront(tabId); break
      case 'Rename Tab…':             openRenameTab(tabId); break
    }
  }

  return {
    activateTab: activateTab,
    cycleTab: cycleTab,
    closeTab: closeTab,
    onTabContext: onTabContext,
    handleTabAction: handleTabAction,
    renameTabTarget: renameTabTarget,
    renameTabValue: renameTabValue,
    confirmRenameTab: confirmRenameTab,
  }
}
