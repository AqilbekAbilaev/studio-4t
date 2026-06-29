<script setup>
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { installInputUndo } from './utils/inputUndo'
import { parseField } from './utils/queryParser'
import BaseIcon from './components/BaseIcon.vue'
import ConnectionTree from './components/ConnectionTree.vue'
import QueryWorkspace from './components/QueryWorkspace.vue'
import ConnectionManager from './components/ConnectionManager.vue'
import ContextMenu from './components/ContextMenu.vue'

import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

appWindow.listen('window-focus', async (event) => {
  if (event.payload === true) {
    await appWindow.setFocus();
  }
});

// ── tab-session persistence ────────────────────────────────
// Persist open collection tabs (and which one is active) so they return after a
// restart. Only the persistable fields are projected — result sets and other
// runtime state are rebuilt on demand, so paging through data never saves.
function projectSession() {
  return {
    activeTabId: activeTabId.value,
    tabs: tabs.value
      .filter(t => t.kind === 'collection' || t.kind === 'shell')
      .map(t => t.kind === 'shell'
        ? {
            id: t.id, kind: 'shell', title: t.title,
            connectionId: t.connectionId, connectionName: t.connectionName,
            dbName: t.dbName, code: t.code,
          }
        : {
            id: t.id, kind: 'collection', title: t.title,
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

onMounted(async () => {
  // WebKitGTK has no native undo/redo for text fields — install our own so Ctrl+Z works.
  installInputUndo()

  // Restore the previous session's tabs before wiring up the save watcher, so the
  // empty default never overwrites tabs.json first.
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
              id: t.id, kind: 'shell', title: t.title,
              connectionId: t.connectionId, connectionName: t.connectionName,
              dbName: t.dbName,
              sessionId: (crypto.randomUUID ? crypto.randomUUID() : t.id),
              code: t.code || '', history: [], isRunning: false,
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
        const active = tabs.value.find(t => t.id === activeTabId.value)
        if (active && active._restored) runRestoredTab(active)
      }
    }
  } catch (_) {}

  // Save on any change to the open tabs or the active tab. The watched getter
  // reads only persistable fields, so result-set updates don't trigger it.
  watch(() => JSON.stringify(projectSession()), scheduleSaveTabs)
});

// ── toolbar definition ─────────────────────────────────────
const TOOLS = [
  { name: 'connect',   label: 'Connect',      badge: '#4caf78', drop: true },
  { name: 'collection',label: 'Collection' },
  { name: 'shell',     label: 'IntelliShell' },
  { name: 'sql',       label: 'SQL',          badge: '#6ea8fe' },
  { name: 'aggregate', label: 'Aggregate',    badge: '#b07ddb' },
  { name: 'search',    label: 'Search in…' },
  { sep: true },
  { name: 'compare',   label: 'Compare' },
  { name: 'schema',    label: 'Schema',       badge: '#4caf78' },
  { name: 'reschema',  label: 'Reschema' },
  { name: 'tasks',     label: 'Tasks' },
  { sep: true },
  { name: 'export',    label: 'Export' },
  { name: 'import',    label: 'Import' },
  { name: 'mask',      label: 'Data Masking' },
  { name: 'migration', label: 'SQL Migration', drop: true },
]

// ── app state ──────────────────────────────────────────────
const tabs = ref([
  { id: 't0', kind: 'quickstart', title: 'Quickstart' }
])
const activeTabId = ref('t0')
const toast = ref(null)
let toastTimer = null
const connectionTreeRef = ref(null)
const showConnectionManager = ref(false)
const expandConnectionId = ref(null)
const vqbOpen        = ref(false)
const clipboardQuery = ref(null)
const contextMenu = ref(null)
const tagOverrides = ref({})

const addCollectionTarget = ref(null)   // { connId, dbName } | null
const newCollectionName   = ref('')
const addCollectionError  = ref(null)
const addCollectionSaving = ref(false)

const dropDatabaseTarget   = ref(null)  // { connId, dbName } | null
const dropDatabaseError    = ref(null)
const dropDatabaseDeleting = ref(false)

const dropCollectionTarget   = ref(null)  // { connId, dbName, collName } | null
const dropCollectionError    = ref(null)
const dropCollectionDeleting  = ref(false)

const renameCollectionTarget = ref(null)  // { connId, dbName, collName } | null
const renameCollectionName   = ref('')
const renameCollectionError  = ref(null)
const renameCollectionSaving = ref(false)

const addDatabaseTarget   = ref(null)  // { connId } | null
const newDatabaseName     = ref('')
const newDatabaseCollName = ref('')
const addDatabaseError    = ref(null)
const addDatabaseSaving   = ref(false)

const indexesTarget   = ref(null)  // { connId, dbName, collName } | null
const indexesList     = ref([])
const indexesLoading  = ref(false)
const indexesError    = ref(null)
const newIndexKeys    = ref('')
const newIndexName    = ref('')
const newIndexUnique  = ref(false)
const indexCreating   = ref(false)
const pendingDropIndex = ref(null)  // index name armed for a confirming second click

const contextActiveNodeKey = computed(() => {
  if (!contextMenu.value) return null
  const nd = contextMenu.value.nodeData
  if (contextMenu.value.type === 'connection') return nd.connId
  if (contextMenu.value.type === 'database') return nd.connId + '/' + nd.dbName
  return nd.connId + '/' + nd.dbName + '/' + nd.collName
})
const sidebarWidth = ref(320)
const sidebarResizing = ref(false)

function startSidebarResize(e) {
  e.preventDefault()
  const startX = e.clientX
  const startW = sidebarWidth.value
  sidebarResizing.value = true
  const onMove = (ev) => {
    sidebarWidth.value = Math.max(200, Math.min(560, startW + (ev.clientX - startX)))
  }
  const onUp = () => {
    sidebarResizing.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

function showToast(msg) {
  clearTimeout(toastTimer)
  toast.value = msg
  toastTimer = setTimeout(() => { toast.value = null }, 2200)
}

// ── active collection tracking (for tree highlight) ────────
const activeCollectionKey = computed(() => {
  const t = tabs.value.find(x => x.id === activeTabId.value)
  return t?.kind === 'collection'
    ? `${t.connectionId}/${t.dbName}/${t.collectionName}`
    : null
})

// ── toolbar handler ────────────────────────────────────────
function handleTool(name) {
  if (name === 'connect') {
    showConnectionManager.value = true
    return
  }
  const label = TOOLS.find(t => t.name === name)?.label || name
  showToast(`${label} — coming to Studio-4T`)
}

function onManagerConnect(id) {
  showConnectionManager.value = false
  expandConnectionId.value = id
}

async function handleContextAction(action) {
  const saved = contextMenu.value
  contextMenu.value = null

  if (action === 'Open Collection') {
    openCollectionTab({
      connectionId: saved.nodeData.connId,
      connectionName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
      collectionName: saved.nodeData.collName,
    })
    return
  }

  if (action === 'Open IntelliShell') {
    openShellTab({
      connectionId: saved.nodeData.connId,
      connectionName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
    })
    return
  }

  if (action.startsWith('Choose Color:')) {
    const color = action.split(':')[1]
    tagOverrides.value = { ...tagOverrides.value, [saved.nodeData.connId]: color }
    showToast('Color tag updated')
    return
  }

  if (action === 'Copy Name') {
    navigator.clipboard.writeText(saved.label)
    showToast('Copied')
    return
  }

  if (action === 'Disconnect') {
    try {
      await invoke('disconnect', { id: saved.nodeData.connId })
    } catch (_) {}
    connectionTreeRef.value.disconnectConn(saved.nodeData.connId)
    tabs.value = tabs.value.filter(t => t.connectionId !== saved.nodeData.connId)
    if (activeTabId.value && !tabs.value.find(t => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value.length ? tabs.value[tabs.value.length - 1].id : null
    }
    showToast('Disconnected from ' + saved.label)
    return
  }

  if (action === 'Disconnect Others') {
    const others = connectionTreeRef.value.getConnections()
      .filter(c => c.id !== saved.nodeData.connId)
    for (const conn of others) {
      try { await invoke('disconnect', { id: conn.id }) } catch (_) {}
      connectionTreeRef.value.disconnectConn(conn.id)
    }
    tabs.value = tabs.value.filter(t => t.kind !== 'collection' || t.connectionId === saved.nodeData.connId)
    if (activeTabId.value && !tabs.value.find(t => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value.length ? tabs.value[tabs.value.length - 1].id : null
    }
    showToast('Disconnected all other connections')
    return
  }

  if (action === 'Disconnect All') {
    const all = connectionTreeRef.value.getConnections()
    for (const conn of all) {
      try { await invoke('disconnect', { id: conn.id }) } catch (_) {}
      connectionTreeRef.value.disconnectConn(conn.id)
    }
    tabs.value = tabs.value.filter(t => t.kind !== 'collection')
    if (activeTabId.value && !tabs.value.find(t => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value.length ? tabs.value[tabs.value.length - 1].id : null
    }
    showToast('All connections closed')
    return
  }

  if (action === 'Refresh Selected Item' || action === 'Refresh') {
    await connectionTreeRef.value.refreshConn(saved.nodeData.connId)
    showToast('Refreshed')
    return
  }

  if (action === 'Add Collection…') {
    addCollectionTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName }
    newCollectionName.value = ''
    addCollectionError.value = null
    return
  }

  if (action === 'Drop Database…') {
    dropDatabaseTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName }
    dropDatabaseError.value = null
    return
  }

  if (action === 'Drop Collection…') {
    dropCollectionTarget.value = {
      connId: saved.nodeData.connId,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    dropCollectionError.value = null
    return
  }

  if (action === 'Rename Collection…') {
    renameCollectionTarget.value = {
      connId: saved.nodeData.connId,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    renameCollectionName.value = saved.nodeData.collName
    renameCollectionError.value = null
    return
  }

  if (action === 'Add Database…') {
    addDatabaseTarget.value = { connId: saved.nodeData.connId }
    newDatabaseName.value = ''
    newDatabaseCollName.value = ''
    addDatabaseError.value = null
    return
  }

  if (action === 'Open Aggregation Editor') {
    openCollectionTab({
      connectionId: saved.nodeData.connId,
      connectionName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
      collectionName: saved.nodeData.collName,
    }, 'aggregate')
    return
  }

  if (action === 'Indexes…') {
    indexesTarget.value = {
      connId: saved.nodeData.connId,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    indexesError.value = null
    newIndexKeys.value = ''
    newIndexName.value = ''
    newIndexUnique.value = false
    pendingDropIndex.value = null
    await loadIndexes()
    return
  }

  if (action === 'Refresh All') {
    for (const conn of connectionTreeRef.value.getConnections()) {
      await connectionTreeRef.value.refreshConn(conn.id)
    }
    showToast('All connections refreshed')
    return
  }

  // Import/Export are wired for collections only; the database/connection-level
  // variants stay stubs for now (they'd need multi-collection handling).
  if (action === 'Export…' && saved.type === 'collection') {
    await exportCollection(saved.nodeData)
    return
  }

  if (action === 'Import…' && saved.type === 'collection') {
    await importCollection(saved.nodeData)
    return
  }

  showToast(action + ' — coming to Studio-4T')
}

async function confirmAddCollection() {
  const target = addCollectionTarget.value
  const name = newCollectionName.value.trim()
  if (!target || !name) return
  addCollectionSaving.value = true
  addCollectionError.value = null
  try {
    await invoke('create_collection', { id: target.connId, database: target.dbName, name: name })
    await connectionTreeRef.value.refreshConn(target.connId)
    showToast(`Collection "${name}" created`)
    addCollectionTarget.value = null
  } catch (e) {
    addCollectionError.value = String(e)
  } finally {
    addCollectionSaving.value = false
  }
}

async function confirmDropDatabase() {
  const target = dropDatabaseTarget.value
  if (!target) return
  dropDatabaseDeleting.value = true
  dropDatabaseError.value = null
  try {
    await invoke('drop_database', { id: target.connId, database: target.dbName })
    await connectionTreeRef.value.refreshConn(target.connId)
    tabs.value = tabs.value.filter(t => !(t.kind === 'collection' && t.connectionId === target.connId && t.dbName === target.dbName))
    if (activeTabId.value && !tabs.value.find(t => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value.length ? tabs.value[tabs.value.length - 1].id : null
    }
    showToast(`Database "${target.dbName}" dropped`)
    dropDatabaseTarget.value = null
  } catch (e) {
    dropDatabaseError.value = String(e)
  } finally {
    dropDatabaseDeleting.value = false
  }
}

async function confirmDropCollection() {
  const target = dropCollectionTarget.value
  if (!target) return
  dropCollectionDeleting.value = true
  dropCollectionError.value = null
  try {
    await invoke('drop_collection', { id: target.connId, database: target.dbName, collection: target.collName })
    await connectionTreeRef.value.refreshConn(target.connId)
    tabs.value = tabs.value.filter(t => !(t.kind === 'collection' && t.connectionId === target.connId && t.dbName === target.dbName && t.collectionName === target.collName))
    if (activeTabId.value && !tabs.value.find(t => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value.length ? tabs.value[tabs.value.length - 1].id : null
    }
    showToast(`Collection "${target.collName}" dropped`)
    dropCollectionTarget.value = null
  } catch (e) {
    dropCollectionError.value = String(e)
  } finally {
    dropCollectionDeleting.value = false
  }
}

async function confirmRenameCollection() {
  const target = renameCollectionTarget.value
  const newName = renameCollectionName.value.trim()
  if (!target || !newName || newName === target.collName) return
  renameCollectionSaving.value = true
  renameCollectionError.value = null
  try {
    await invoke('rename_collection', { id: target.connId, database: target.dbName, collection: target.collName, newName: newName })
    await connectionTreeRef.value.refreshConn(target.connId)
    const open = tabs.value.find(t => t.kind === 'collection' && t.connectionId === target.connId && t.dbName === target.dbName && t.collectionName === target.collName)
    if (open) {
      open.collectionName = newName
      open.title = newName
    }
    showToast(`Collection renamed to "${newName}"`)
    renameCollectionTarget.value = null
  } catch (e) {
    renameCollectionError.value = String(e)
  } finally {
    renameCollectionSaving.value = false
  }
}

async function confirmAddDatabase() {
  const target = addDatabaseTarget.value
  const dbName = newDatabaseName.value.trim()
  const collName = newDatabaseCollName.value.trim()
  if (!target || !dbName || !collName) return
  addDatabaseSaving.value = true
  addDatabaseError.value = null
  try {
    await invoke('create_database', { id: target.connId, database: dbName, firstCollection: collName })
    await connectionTreeRef.value.refreshConn(target.connId)
    showToast(`Database "${dbName}" created`)
    addDatabaseTarget.value = null
  } catch (e) {
    addDatabaseError.value = String(e)
  } finally {
    addDatabaseSaving.value = false
  }
}

async function loadIndexes() {
  const target = indexesTarget.value
  if (!target) return
  indexesLoading.value = true
  indexesError.value = null
  try {
    indexesList.value = await invoke('list_indexes', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
    })
  } catch (e) {
    indexesError.value = String(e)
    indexesList.value = []
  } finally {
    indexesLoading.value = false
  }
}

async function confirmCreateIndex() {
  const target = indexesTarget.value
  const keys = newIndexKeys.value.trim()
  if (!target || !keys) return
  indexCreating.value = true
  indexesError.value = null
  try {
    await invoke('create_index', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
      keys: keys,
      unique: newIndexUnique.value,
      name: newIndexName.value.trim(),
    })
    newIndexKeys.value = ''
    newIndexName.value = ''
    newIndexUnique.value = false
    await loadIndexes()
    showToast('Index created')
  } catch (e) {
    indexesError.value = String(e)
  } finally {
    indexCreating.value = false
  }
}

// Two-click guard: the first click arms a row, the second actually drops it.
async function dropIndex(name) {
  if (pendingDropIndex.value !== name) {
    pendingDropIndex.value = name
    return
  }
  const target = indexesTarget.value
  if (!target) return
  indexesError.value = null
  try {
    await invoke('drop_index', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
      name: name,
    })
    pendingDropIndex.value = null
    await loadIndexes()
    showToast(`Index "${name}" dropped`)
  } catch (e) {
    indexesError.value = String(e)
    pendingDropIndex.value = null
  }
}

function indexKeyLabel(index) {
  if (!index || !index.key) return ''
  return Object.entries(index.key).map(([k, v]) => `${k}: ${v}`).join(', ')
}

async function exportCollection(nodeData) {
  let path
  try {
    path = await saveDialog({
      defaultPath: `${nodeData.collName}.json`,
      filters: [
        { name: 'JSON', extensions: ['json'] },
        { name: 'CSV', extensions: ['csv'] },
      ],
    })
  } catch (e) {
    showToast('Export failed: ' + String(e))
    return
  }
  if (!path) return  // user cancelled
  const format = path.toLowerCase().endsWith('.csv') ? 'csv' : 'json'
  try {
    const count = await invoke('export_collection', {
      id: nodeData.connId,
      database: nodeData.dbName,
      collection: nodeData.collName,
      path: path,
      format: format,
    })
    showToast(`Exported ${count} document${count !== 1 ? 's' : ''} to ${format.toUpperCase()}`)
  } catch (e) {
    showToast('Export failed: ' + String(e))
  }
}

async function importCollection(nodeData) {
  let path
  try {
    path = await openDialog({
      multiple: false,
      filters: [{ name: 'JSON or CSV', extensions: ['json', 'csv'] }],
    })
  } catch (e) {
    showToast('Import failed: ' + String(e))
    return
  }
  if (!path) return  // user cancelled
  const format = String(path).toLowerCase().endsWith('.csv') ? 'csv' : 'json'
  try {
    const count = await invoke('import_collection', {
      id: nodeData.connId,
      database: nodeData.dbName,
      collection: nodeData.collName,
      path: path,
      format: format,
    })
    await connectionTreeRef.value.refreshConn(nodeData.connId)
    showToast(`Imported ${count} document${count !== 1 ? 's' : ''}`)
  } catch (e) {
    showToast('Import failed: ' + String(e))
  }
}

// ── tab management ─────────────────────────────────────────
async function openCollectionTab({ connectionId, connectionName, dbName, collectionName }, startMode = 'find') {
  // Every open creates a new tab — the same collection may be opened in several
  // tabs (Studio-3T behavior). No dedup/focus-existing here by design.
  const id = 't' + Date.now()
  tabs.value.push({
    id: id, kind: 'collection',
    title: collectionName,
    connectionId: connectionId,
    connectionName: connectionName,
    dbName: dbName,
    collectionName: collectionName,
    filter: '', projection: '', sort: '', skip: 0, limit: 50,
    mode: startMode, pipeline: '',
    results: [], hasRun: false, isRunning: false, runError: null,
    selectedRow: -1, elapsedMs: null,
  })
  activeTabId.value = id

  let def = null
  try {
    def = await invoke('get_default_query', {
      connectionId: connectionId,
      database:     dbName,
      collection:   collectionName,
    })
  } catch (_) {}

  // Aggregation tabs open with an empty pipeline; nothing to run until the user writes one.
  if (startMode !== 'find') return

  if (def) {
    const tab = tabs.value.find(t => t.id === id)
    if (tab) {
      tab.filter     = def.filter     || ''
      tab.sort       = def.sort       || ''
      tab.projection = def.projection || ''
      tab.skip       = Number(def.skip)
      tab.limit      = Number(def.limit)
    }
    const pf = parseField(def.filter     || '')
    const ps = parseField(def.sort       || '')
    const pp = parseField(def.projection || '')
    runQuery(id, {
      filter:     pf.ok ? pf.ejson : '{}',
      sort:       ps.ok ? ps.ejson : '{}',
      projection: pp.ok ? pp.ejson : '{}',
      skip:       Number(def.skip),
      limit:      Number(def.limit),
    })
  } else {
    runQuery(id, { filter: '{}', projection: '{}', sort: '{}', skip: 0, limit: 50 })
  }
}

// Open an IntelliShell tab scoped to a connection + database. Each shell tab has
// its own backend JS session (sessionId), so variables persist across runs.
function openShellTab({ connectionId, connectionName, dbName }) {
  const id = 't' + Date.now()
  tabs.value.push({
    id: id, kind: 'shell',
    title: 'mongosh: ' + dbName,
    connectionId: connectionId,
    connectionName: connectionName,
    dbName: dbName,
    sessionId: (crypto.randomUUID ? crypto.randomUUID() : id),
    // editor + command history (dropdown)
    code: '', history: [], isRunning: false,
    // result state, read by the reused result grid (ResultTable / TreeView)
    results: [], resultView: 'table', resultTab: 'Console',
    runError: null, elapsedMs: null, drillPath: [], hasRun: false, selectedRow: -1,
    logs: [], scalar: undefined, hasScalar: false,
  })
  activeTabId.value = id
}

function activateTab(id) {
  activeTabId.value = id
  const tab = tabs.value.find(t => t.id === id)
  if (tab && tab._restored) runRestoredTab(tab)
}

// A tab restored from a previous session carries its query text but no results.
// We run it lazily — the first time it becomes active — so a restart doesn't
// reconnect to every server at once. Find tabs re-run their stored query;
// aggregate tabs just keep their pipeline text and wait for a manual run.
function runRestoredTab(tab) {
  tab._restored = false
  if (tab.mode !== 'find') return
  const pf = parseField(tab.filter     || '')
  const ps = parseField(tab.sort       || '')
  const pp = parseField(tab.projection || '')
  runQuery(tab.id, {
    filter:     pf.ok ? pf.ejson : '{}',
    sort:       ps.ok ? ps.ejson : '{}',
    projection: pp.ok ? pp.ejson : '{}',
    skip:       Number(tab.skip),
    limit:      Number(tab.limit),
  })
}

function onCopyQuery() {
  const tab = tabs.value.find(t => t.id === activeTabId.value)
  if (!tab) return
  clipboardQuery.value = {
    mode:       tab.mode       || 'find',
    filter:     tab.filter     || '',
    sort:       tab.sort       || '',
    projection: tab.projection || '',
    skip:       tab.skip       ?? 0,
    limit:      tab.limit      ?? 50,
    pipeline:   tab.pipeline   || '',
  }
  showToast('Query copied.')
}

async function onPasteQuery() {
  const tab = tabs.value.find(t => t.id === activeTabId.value)
  if (!tab || !clipboardQuery.value) return
  const q = clipboardQuery.value
  tab.mode       = q.mode
  tab.filter     = q.filter
  tab.sort       = q.sort
  tab.projection = q.projection
  tab.skip       = Number(q.skip)
  tab.limit      = Number(q.limit)
  tab.pipeline   = q.pipeline
  if (q.mode !== 'find') return
  const pf = parseField(q.filter     || '')
  const ps = parseField(q.sort       || '')
  const pp = parseField(q.projection || '')
  await nextTick()
  runQuery(tab.id, {
    filter:     pf.ok ? pf.ejson : '{}',
    sort:       ps.ok ? ps.ejson : '{}',
    projection: pp.ok ? pp.ejson : '{}',
    skip:       Number(q.skip),
    limit:      Number(q.limit),
  })
}

function closeTab(id) {
  const idx = tabs.value.findIndex(t => t.id === id)
  if (idx < 0) return
  const closing = tabs.value[idx]
  if (closing.kind === 'shell' && closing.sessionId) {
    invoke('close_shell_session', { sessionId: closing.sessionId }).catch(() => {})
  }
  tabs.value.splice(idx, 1)
  if (activeTabId.value === id) {
    const next = tabs.value[Math.max(0, idx - 1)]
    activeTabId.value = next?.id ?? null
  }
}

// ── query execution ────────────────────────────────────────
async function runQuery(tabId, params) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (!tab) return
  tab.isRunning = true
  tab.runError = null
  const t0 = Date.now()
  const { addToHistory = true, ...queryParams } = params
  try {
    tab.results = await invoke('find_documents', {
      id:         tab.connectionId,
      database:   tab.dbName,
      collection: tab.collectionName,
      ...queryParams,
    })
    tab.hasRun = true
    tab.elapsedMs = Date.now() - t0
    showToast(`Query returned ${tab.results.length} document${tab.results.length !== 1 ? 's' : ''} in ${(tab.elapsedMs / 1000).toFixed(3)}s`)
    if (addToHistory) {
      invoke('push_query_history', {
        connectionId: tab.connectionId,
        database:     tab.dbName,
        collection:   tab.collectionName,
        mode:         'find',
        filter:       tab.filter     || '',
        sort:         tab.sort       || '',
        projection:   tab.projection || '',
        skip:         queryParams.skip  ?? 0,
        limit:        queryParams.limit ?? 50,
        pipeline:     '',
      }).catch(() => {})
    }
  } catch (e) {
    tab.runError = String(e)
  } finally {
    tab.isRunning = false
  }
}

async function runAggregate(tabId, params) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (!tab) return
  tab.isRunning = true
  tab.runError = null
  const t0 = Date.now()
  try {
    tab.results = await invoke('run_aggregate', {
      id:         tab.connectionId,
      database:   tab.dbName,
      collection: tab.collectionName,
      ...params,
    })
    tab.hasRun = true
    tab.elapsedMs = Date.now() - t0
    showToast(`Aggregation returned ${tab.results.length} document${tab.results.length !== 1 ? 's' : ''} in ${(tab.elapsedMs / 1000).toFixed(3)}s`)
    invoke('push_query_history', {
      connectionId: tab.connectionId,
      database:     tab.dbName,
      collection:   tab.collectionName,
      mode:         'aggregate',
      filter:       '',
      sort:         '',
      projection:   '',
      skip:         0,
      limit:        50,
      pipeline:     tab.pipeline || '',
    }).catch(() => {})
  } catch (e) {
    tab.runError = String(e)
  } finally {
    tab.isRunning = false
  }
}
</script>

<template>
  <div class="app-layout">
    <!-- Toolbar -->
    <div class="toolbar">
      <template v-for="(t, i) in TOOLS" :key="i">
        <div v-if="t.sep" class="tb-sep"></div>
        <button v-else class="tbtn" :title="t.label" @click="handleTool(t.name)">
          <span class="ic" :class="{ 'ic-badge': t.badge }">
            <BaseIcon :name="t.name" :size="22" />
            <i v-if="t.badge" class="dotmark" :style="{ background: t.badge }"></i>
          </span>
          <span class="lbl">{{ t.label }}</span>
          <BaseIcon v-if="t.drop" name="caretDown" :size="11" class="drop" />
        </button>
      </template>
    </div>

    <!-- Main row -->
    <div class="app-main">
      <!-- Left rail -->
      <div class="rail-left">
        <span class="rail-label">Open connections</span>
        <span class="rail-label" style="margin-top:auto">Operations</span>
      </div>

      <!-- Sidebar -->
      <ConnectionTree
        ref="connectionTreeRef"
        :width="sidebarWidth"
        :active-collection-key="activeCollectionKey"
        :expand-id="expandConnectionId"
        :tag-overrides="tagOverrides"
        :context-active-node-key="contextActiveNodeKey"
        @select-collection="openCollectionTab"
        @expanded="expandConnectionId = null"
        @context-menu="contextMenu = $event"
      />
      <div class="resizer" :class="{ dragging: sidebarResizing }" title="Drag to resize" @mousedown="startSidebarResize">
        <span class="resizer-grip"></span>
      </div>

      <!-- Workspace -->
      <QueryWorkspace
        :tabs="tabs"
        :active-tab-id="activeTabId"
        :vqb-open="vqbOpen"
        :clipboard-query="clipboardQuery"
        @activate-tab="activateTab"
        @close-tab="closeTab"
        @run-query="runQuery"
        @run-aggregate="runAggregate"
        @toggle-vqb="vqbOpen = !vqbOpen"
        @open-vqb="vqbOpen = true"
        @close-vqb="vqbOpen = false"
        @toast="showToast"
        @copy-query="onCopyQuery"
        @paste-query="onPasteQuery"
      />
    </div>

    <!-- Context menu -->
    <ContextMenu
      v-if="contextMenu"
      :menu="contextMenu"
      @close="contextMenu = null"
      @pick="handleContextAction"
    />

    <!-- Connection Manager modal -->
    <ConnectionManager
      v-if="showConnectionManager"
      @close="showConnectionManager = false"
      @connect="onManagerConnect"
    />

    <!-- Add Collection modal -->
    <div v-if="addCollectionTarget" class="del-overlay" @mousedown.self="addCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add Collection</div>
          <button class="close-btn" @click="addCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newCollectionName"
            class="prompt-input"
            placeholder="Collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddCollection"
          />
          <div v-if="addCollectionError" class="del-error">{{ addCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newCollectionName.trim() || addCollectionSaving" @click="confirmAddCollection">
            {{ addCollectionSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Drop Database confirm -->
    <div v-if="dropDatabaseTarget" class="del-overlay" @mousedown.self="dropDatabaseTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Database</div>
          <button class="close-btn" @click="dropDatabaseTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>Are you sure you want to drop "<strong>{{ dropDatabaseTarget.dbName }}</strong>"? This deletes all of its collections and cannot be undone.</p>
          <div v-if="dropDatabaseError" class="del-error">{{ dropDatabaseError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropDatabaseTarget = null">Cancel</button>
          <button class="btn danger" :disabled="dropDatabaseDeleting" @click="confirmDropDatabase">
            {{ dropDatabaseDeleting ? 'Dropping…' : 'Drop' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Drop Collection confirm -->
    <div v-if="dropCollectionTarget" class="del-overlay" @mousedown.self="dropCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Collection</div>
          <button class="close-btn" @click="dropCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>Are you sure you want to drop "<strong>{{ dropCollectionTarget.collName }}</strong>"? This deletes all of its documents and cannot be undone.</p>
          <div v-if="dropCollectionError" class="del-error">{{ dropCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropCollectionTarget = null">Cancel</button>
          <button class="btn danger" :disabled="dropCollectionDeleting" @click="confirmDropCollection">
            {{ dropCollectionDeleting ? 'Dropping…' : 'Drop' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rename Collection modal -->
    <div v-if="renameCollectionTarget" class="del-overlay" @mousedown.self="renameCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Rename Collection</div>
          <button class="close-btn" @click="renameCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="renameCollectionName"
            class="prompt-input"
            placeholder="New collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmRenameCollection"
          />
          <div v-if="renameCollectionError" class="del-error">{{ renameCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="renameCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!renameCollectionName.trim() || renameCollectionName.trim() === renameCollectionTarget.collName || renameCollectionSaving" @click="confirmRenameCollection">
            {{ renameCollectionSaving ? 'Renaming…' : 'Rename' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Add Database modal -->
    <div v-if="addDatabaseTarget" class="del-overlay" @mousedown.self="addDatabaseTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add Database</div>
          <button class="close-btn" @click="addDatabaseTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newDatabaseName"
            class="prompt-input"
            placeholder="Database name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <input
            v-model="newDatabaseCollName"
            class="prompt-input"
            style="margin-top:8px"
            placeholder="First collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddDatabase"
          />
          <p style="margin-top:8px;color:var(--text-faint);font-size:12px">MongoDB only creates a database once it holds a collection, so a first collection is required.</p>
          <div v-if="addDatabaseError" class="del-error">{{ addDatabaseError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addDatabaseTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newDatabaseName.trim() || !newDatabaseCollName.trim() || addDatabaseSaving" @click="confirmAddDatabase">
            {{ addDatabaseSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Indexes modal -->
    <div v-if="indexesTarget" class="del-overlay" @mousedown.self="indexesTarget = null">
      <div class="del-dialog idx-dialog">
        <div class="del-title">
          <div class="t">Indexes — {{ indexesTarget.collName }}</div>
          <button class="close-btn" @click="indexesTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <div v-if="indexesLoading" class="idx-msg">Loading indexes…</div>
          <table v-else-if="indexesList.length" class="idx-table">
            <thead>
              <tr><th>Name</th><th>Keys</th><th>Unique</th><th></th></tr>
            </thead>
            <tbody>
              <tr v-for="idx in indexesList" :key="idx.name">
                <td class="idx-name">{{ idx.name }}</td>
                <td class="idx-keys">{{ indexKeyLabel(idx) }}</td>
                <td>{{ idx.unique ? 'Yes' : '—' }}</td>
                <td class="idx-actions">
                  <button
                    v-if="idx.name !== '_id_'"
                    class="btn"
                    :class="{ danger: pendingDropIndex === idx.name }"
                    @click="dropIndex(idx.name)"
                  >{{ pendingDropIndex === idx.name ? 'Confirm' : 'Drop' }}</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="idx-msg">No indexes.</div>

          <div class="idx-create">
            <div class="idx-create-title">Create index</div>
            <input
              v-model="newIndexKeys"
              class="prompt-input"
              placeholder='Keys, e.g. {"field": 1}'
              spellcheck="false"
              autocorrect="off"
              autocapitalize="off"
            />
            <input
              v-model="newIndexName"
              class="prompt-input"
              style="margin-top:8px"
              placeholder="Index name (optional)"
              spellcheck="false"
              autocorrect="off"
              autocapitalize="off"
            />
            <label class="idx-unique">
              <input type="checkbox" v-model="newIndexUnique" />
              <span>Unique</span>
            </label>
          </div>

          <div v-if="indexesError" class="del-error">{{ indexesError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="indexesTarget = null">Close</button>
          <button class="btn primary" :disabled="!newIndexKeys.trim() || indexCreating" @click="confirmCreateIndex">
            {{ indexCreating ? 'Creating…' : 'Create index' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Toast -->
    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-window);
}

/* ── Titlebar ── */
.titlebar {
  height: 38px;
  flex: none;
  background: linear-gradient(#34363a, #2c2e31);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 14px;
  position: relative;
  -webkit-app-region: drag;
}
.traffic {
  display: flex;
  gap: 8px;
  -webkit-app-region: no-drag;
}
.light {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}
.light.r { background: #ec6a5e; }
.light.y { background: #f4bf4f; }
.light.g { background: #61c554; }
.title {
  position: absolute;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}

/* ── Toolbar ── */
.toolbar {
  flex: none;
  background: var(--bg-toolbar);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: stretch;
  padding: 6px 8px;
  gap: 2px;
}
.tbtn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 5px 9px;
  border: none;
  background: none;
  border-radius: 6px;
  color: var(--text);
  min-width: 54px;
  position: relative;
}
.tbtn:hover { background: var(--bg-hover); }
.tbtn .ic { color: var(--text-dim); position: relative; }
.tbtn:hover .ic { color: var(--text); }
.tbtn .lbl { font-size: 10.5px; color: var(--text-dim); white-space: nowrap; }
.tbtn .drop { position: absolute; right: 2px; top: 3px; color: var(--text-faint); }
.tb-sep { width: 1px; flex: none; background: var(--border-soft); margin: 6px 4px; align-self: stretch; }
.ic-badge { position: relative; }
.dotmark {
  position: absolute;
  right: -1px;
  bottom: 1px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1.5px solid var(--bg-toolbar);
}

/* ── Main split ── */
.app-main {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* ── Left rail ── */
.rail-left {
  width: 26px;
  flex: none;
  background: var(--bg-panel-2);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 0;
}
.rail-label {
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  font-size: 11px;
  color: var(--text-dim);
  letter-spacing: .3px;
}

/* ── Resizer ── */
.resizer {
  width: 3px;
  flex: none;
  cursor: col-resize;
  background: var(--border);
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}
.resizer-grip {
  width: 2px;
  height: 32px;
  background: transparent;
  border-radius: 1px;
  cursor: col-resize;
  transition: background 0.12s;
}
.resizer:hover .resizer-grip,
.resizer.dragging .resizer-grip { background: var(--accent); }

/* ── Toast ── */
.toast {
  position: fixed;
  bottom: 18px;
  left: 50%;
  transform: translateX(-50%);
  background: #303236;
  border: 1px solid var(--border-soft);
  color: var(--text);
  padding: 9px 16px;
  border-radius: 8px;
  font-size: 12.5px;
  box-shadow: 0 12px 30px rgba(0,0,0,.5);
  z-index: 80;
  white-space: nowrap;
}

/* ── Add Collection / Drop Database dialogs ── */
.del-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 60;
}
.del-dialog {
  width: 400px;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px #000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.del-title {
  height: 36px;
  flex: none;
  background: linear-gradient(#34363a, #2c2e31);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
  position: relative;
}
.del-title .t {
  position: absolute;
  left: 0; right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}
.close-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  border-radius: 4px;
  z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
.del-body {
  padding: 20px 20px 12px;
  font-size: 13px;
  color: var(--text);
  line-height: 1.5;
}
.del-body p { margin: 0 0 8px; }
.del-error { font-size: 12px; color: #e05555; margin-top: 6px; }
.prompt-input {
  width: 100%;
  height: 30px;
  padding: 0 10px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-size: 13px;
  box-sizing: border-box;
}
.prompt-input:focus { outline: none; border-color: var(--accent); }

.idx-dialog { width: 560px; }
.idx-msg { padding: 16px 0; color: var(--text-faint); font-size: 12px; }
.idx-table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
.idx-table th { text-align: left; font-weight: 500; color: var(--text-faint); padding: 4px 8px; border-bottom: 1px solid var(--border); }
.idx-table td { padding: 5px 8px; border-bottom: 1px solid var(--border-soft); vertical-align: middle; }
.idx-name { font-family: var(--mono); }
.idx-keys { font-family: var(--mono); color: var(--text-dim); }
.idx-actions { text-align: right; width: 1%; white-space: nowrap; }
.idx-create { margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border); }
.idx-create-title { font-size: 12px; color: var(--text-faint); margin-bottom: 8px; }
.idx-unique { display: flex; align-items: center; gap: 6px; margin-top: 8px; font-size: 12.5px; color: var(--text-dim); cursor: pointer; }
.del-footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
}
.spacer { flex: 1; }
.btn {
  height: 28px;
  padding: 0 14px;
  border-radius: 5px;
  border: none;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-toolbar);
  color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn:disabled { opacity: .5; cursor: default; }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover:not(:disabled) { opacity: .88; }
.btn.danger { background: #c0392b; color: #fff; }
.btn.danger:hover:not(:disabled) { background: #a93226; }
</style>
