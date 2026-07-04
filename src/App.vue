<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { installInputUndo } from './utils/inputUndo'
import { parseField, parsePipeline } from './utils/queryParser'
import { errMessage, errCode } from './utils/errors'
import { deriveMenuContext, resolveMenuTarget } from './utils/menuContext'
import { isProtectedIndex, indexKeyLabel, indexSpecJson, isIndexHidden } from './utils/indexSpec'
import BaseIcon from './components/BaseIcon.vue'
import ConnectionTree from './components/ConnectionTree.vue'
import QueryWorkspace from './components/QueryWorkspace.vue'
import ConnectionManager from './components/ConnectionManager.vue'
import ContextMenu from './components/ContextMenu.vue'
import SshHostKeyModal from './components/SshHostKeyModal.vue'
import ServerStatusModal from './components/ServerStatusModal.vue'
import DatabaseStatsModal from './components/DatabaseStatsModal.vue'
import CurrentOpsModal from './components/CurrentOpsModal.vue'
import ValidatorModal from './components/ValidatorModal.vue'
import AboutModal from './components/AboutModal.vue'
import SchemaModal from './components/SchemaModal.vue'
import SqlModal from './components/SqlModal.vue'
import MaskingModal from './components/MaskingModal.vue'
import StatsModal from './components/StatsModal.vue'
import ServerInfoModal from './components/ServerInfoModal.vue'
import MigrationModal from './components/MigrationModal.vue'
import SearchModal from './components/SearchModal.vue'
import GridFsModal from './components/GridFsModal.vue'
import CompareModal from './components/CompareModal.vue'
import ShortcutsModal from './components/ShortcutsModal.vue'
import PreferencesModal from './components/PreferencesModal.vue'

import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

const appWindow = getCurrentWindow();

// On macOS/Windows the native menu registers the keyboard accelerators. On Linux
// it doesn't (WebKitGTK would swallow editing keys), so the webview keeps its own
// shortcut handling there. Detected from the webview's platform string.
const NATIVE_MENU_OWNS_SHORTCUTS = !/Linux/i.test(navigator.userAgent);

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
            id: t.id, kind: 'shell', title: t.title, color: t.color,
            connectionId: t.connectionId, connectionName: t.connectionName,
            dbName: t.dbName, code: t.code,
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

onMounted(async () => {
  // WebKitGTK has no native undo/redo for text fields — install our own so Ctrl+Z works.
  installInputUndo()

  // Backend-raised SSH host-key prompts (global emits, so use the app-wide listen).
  listen('ssh-host-key-prompt', (e) => { sshHostKeyPrompt.value = e.payload })
  listen('ssh-host-key-changed', (e) => { sshHostKeyChanged.value = e.payload })

  // Native menu clicks arrive here; route them through the same handlers the
  // custom bar used. (menu.rs emits the clicked item's id.)
  listen('menu-action', (e) => handleMenuAction(e.payload))

  // On Linux the native menu carries no accelerators (they'd swallow editing keys
  // on WebKitGTK — see menu.rs), so we keep our own keyboard shortcuts there. On
  // macOS/Windows the native menu owns the accelerators, so we don't double-bind.
  if (NATIVE_MENU_OWNS_SHORTCUTS === false) {
    window.addEventListener('keydown', onGlobalKeydown)
  }

  // Load persisted preferences so new tabs adopt the configured default limit.
  try {
    const settings = await invoke('get_settings')
    if (settings && Number(settings.default_query_limit)) {
      defaultQueryLimit.value = Number(settings.default_query_limit)
    }
    if (settings && settings.theme) applyTheme(settings.theme)
  } catch (_) {}

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
              id: t.id, kind: 'shell', title: t.title, color: t.color,
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

onUnmounted(() => {
  window.removeEventListener('keydown', onGlobalKeydown)
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
// The sidebar's current single-click selection and how many connections are open.
// Both feed `menuContext`, so the native menu enables items based on what's
// selected/open in the tree, not only on the active tab.
const treeSelection = ref(null)       // { connectionId, connectionName, dbName, collectionName, kind } | null
const treeConnectionCount = ref(0)
// A one-shot request routed from the native menu down to the active collection's
// ResultsPanel (which owns the editors and results view). Used for Document/Collection
// editing as well as the View menu's view-mode toggles and Refresh Document. Bumping
// `nonce` re-fires the panel's watcher; `action` is the menu item id.
const docMenuRequest = ref(null)      // { action, nonce } | null
const toolbarHidden = ref(false)      // View → Hide Global Toolbar toggle
const historyRequest = ref(null)      // View → History Manager: { nonce } signal to the QueryBar
const showConnectionManager = ref(false)
const serverStatusTarget = ref(null)  // { connId, connName } when the Server Status modal is open
const dbStatsTarget = ref(null)       // { connId, connName, dbName } when the Database Statistics modal is open
const currentOpsTarget = ref(null)    // { connId, connName } when the Current Operations modal is open
const validatorTarget = ref(null)     // { connId, connName, dbName, collName } when the Validator modal is open
const migrationTarget = ref(null)     // { connId, connName, dbName, collName } for the SQL Migration modal
const searchTarget = ref(null)        // { connId, connName, dbName } for the Global Search modal
const gridfsTarget = ref(null)        // { connId, connName, dbName } for the GridFS modal
const compareTarget = ref(null)       // { connId, connName, dbName } for the Data Compare modal
const schemaTarget = ref(null)  // { connId, connName, dbName, collName } when the Schema modal is open
const showSqlModal = ref(false)       // SQL → MQL translator modal (top-bar SQL button)
const maskingTarget = ref(null)       // { connId, connName, dbName, collName } for the Data Masking modal
const statsTarget = ref(null)         // { connId, connName, dbName, collName } for the Collection Stats modal
const serverInfoTarget = ref(null)    // { connId, connName, kind, title } for Build/Host/Replica info
const showShortcuts = ref(false)      // Help → Keyboard Shortcuts reference
const showAbout = ref(false)          // Help → About
const showPreferences = ref(false)    // File → Preferences
const defaultQueryLimit = ref(50)     // from settings; applied to newly opened collection tabs
const theme = ref('dark')             // from settings; drives <html data-theme>

// Apply a theme everywhere it needs to live: the ref (for the Preferences select),
// the <html> attribute (which the CSS tokens key off), and the localStorage mirror
// that lets both webviews pre-paint on next launch without a flash.
function applyTheme(next) {
  const value = next === 'light' ? 'light' : 'dark'
  theme.value = value
  document.documentElement.dataset.theme = value
  localStorage.setItem('s4t-theme', value)
}

// SSH host-key prompts raised by the backend during a tunnel handshake. At most
// one of each is active at a time; the modal shows the prompt first.
const sshHostKeyPrompt = ref(null)   // { requestId, host, port, fingerprint }
const sshHostKeyChanged = ref(null)  // { host, port, storedFingerprint, presentedFingerprint }

function onHostKeyTrust() {
  if (sshHostKeyPrompt.value) {
    invoke('respond_ssh_host_key', { requestId: sshHostKeyPrompt.value.requestId, trust: true })
    sshHostKeyPrompt.value = null
  }
}
function onHostKeyCancel() {
  if (sshHostKeyPrompt.value) {
    invoke('respond_ssh_host_key', { requestId: sshHostKeyPrompt.value.requestId, trust: false })
    sshHostKeyPrompt.value = null
  }
}
async function onHostKeyForget() {
  if (sshHostKeyChanged.value) {
    await invoke('forget_ssh_host', { host: sshHostKeyChanged.value.host, port: sshHostKeyChanged.value.port })
    sshHostKeyChanged.value = null
  }
}
const expandConnectionId = ref(null)
const vqbOpen        = ref(false)
const clipboardQuery = ref(null)
const contextMenu = ref(null)
const tagOverrides = ref({})

const addCollectionTarget = ref(null)   // { connId, dbName } | null
const newCollectionName   = ref('')
const addCollectionError  = ref(null)
const addCollectionSaving = ref(false)

const addViewTarget   = ref(null)       // { connId, dbName } | null
const newViewName     = ref('')
const newViewSource   = ref('')         // source collection the view reads from
const newViewPipeline = ref('')         // aggregation pipeline (JSON array, optional)
const addViewError    = ref(null)
const addViewSaving   = ref(false)

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

const duplicateCollectionTarget = ref(null)  // { connId, dbName, collName } | null
const duplicateCollectionName   = ref('')
const duplicateCollectionError  = ref(null)
const duplicateCollectionSaving = ref(false)

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

// Index-menu selection & dialogs. `selectedIndex` is the index row highlighted in
// the Indexes dialog; it drives the Index menu's enablement (see menuContext) and
// is the target of every Index-menu action.
const selectedIndex        = ref(null)   // the selected index doc | null
const indexFormMode        = ref('create')  // 'create' | 'edit'
const indexEditOriginalName = ref('')    // name of the index being edited (edit mode)
const indexDetailsTarget   = ref(null)   // the index shown in the View Details modal | null
const indexDetailsStats    = ref(null)   // its $indexStats entry | null
const indexDetailsLoading  = ref(false)
const dropIndexTarget      = ref(null)   // { name } armed for the type-to-confirm drop | null
const dropIndexConfirmText = ref('')
const dropIndexError       = ref(null)
const dropIndexBusy        = ref(false)

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
// `target` lets the native menu inject the sidebar selection; the toolbar buttons
// omit it and so act on the active tab, exactly as before.
function handleTool(name, target = null) {
  if (name === 'connect') {
    showConnectionManager.value = true
    return
  }
  if (name === 'sql') {
    showSqlModal.value = true
    return
  }
  if (name === 'collection') {
    // Opens the collection currently highlighted in the sidebar, same as
    // double-clicking it. Guides the user when nothing is highlighted.
    if (!connectionTreeRef.value.openSelectedCollection()) {
      showToast('Select a collection in the sidebar first')
    }
    return
  }
  // The remaining actions operate on a specific node. From the toolbar that's the
  // active tab; from the native menu the caller passes the sidebar selection.
  // Collection and shell tabs carry the connection/database fields; Quickstart
  // (and any context-less tab) does not, so we guide the user instead.
  const tab = target || tabs.value.find(t => t.id === activeTabId.value)
  if (name === 'shell') {
    if (tab && tab.connectionId && tab.dbName) {
      openShellTab({
        connectionId: tab.connectionId,
        connectionName: tab.connectionName,
        dbName: tab.dbName,
      })
    } else {
      showToast('Select a database or collection first to open IntelliShell')
    }
    return
  }
  // Aggregate / Export / Import are collection-scoped features that already exist
  // (via the sidebar right-click); the target must be a collection.
  if (name === 'aggregate' || name === 'export' || name === 'import') {
    if (!tab || tab.kind !== 'collection') {
      showToast('Open a collection first')
      return
    }
    if (name === 'aggregate') {
      openCollectionTab({
        connectionId: tab.connectionId,
        connectionName: tab.connectionName,
        dbName: tab.dbName,
        collectionName: tab.collectionName,
      }, 'aggregate')
    } else {
      const nodeData = { connId: tab.connectionId, dbName: tab.dbName, collName: tab.collectionName }
      if (name === 'export') exportCollection(nodeData)
      else importCollection(nodeData)
    }
    return
  }
  if (name === 'mask') {
    if (!tab || tab.kind !== 'collection') {
      showToast('Open a collection first')
      return
    }
    maskingTarget.value = {
      connId: tab.connectionId,
      connName: tab.connectionName,
      dbName: tab.dbName,
      collName: tab.collectionName,
    }
    return
  }
  if (name === 'migration') {
    if (!tab || tab.kind !== 'collection') {
      showToast('Open a collection first')
      return
    }
    migrationTarget.value = {
      connId: tab.connectionId,
      connName: tab.connectionName,
      dbName: tab.dbName,
      collName: tab.collectionName,
    }
    return
  }
  if (name === 'search') {
    if (!tab || !tab.connectionId || !tab.dbName) {
      showToast('Open a collection or database first')
      return
    }
    searchTarget.value = {
      connId: tab.connectionId,
      connName: tab.connectionName,
      dbName: tab.dbName,
    }
    return
  }
  if (name === 'compare') {
    if (!tab || !tab.connectionId || !tab.dbName) {
      showToast('Open a collection or database first')
      return
    }
    compareTarget.value = {
      connId: tab.connectionId,
      connName: tab.connectionName,
      dbName: tab.dbName,
    }
    return
  }
  const label = TOOLS.find(t => t.name === name)?.label || name
  showToast(`${label} — coming to Studio-4T`)
}

// Bridges a native-menu item into the existing right-click context handler by
// synthesizing the "selected node" from the current target (sidebar selection, or
// the active tab). `requiredType` guards the action: server-level items need a
// connection, most Collection-menu items need a collection. Guides the user when
// the context is missing.
function menuNode(action, requiredType) {
  const tab = menuTarget(requiredType)
  if (!tab || !tab.connectionId) {
    showToast('Open a connection, database, or collection first')
    return
  }
  if (requiredType === 'collection' && (tab.kind !== 'collection' || !tab.collectionName)) {
    showToast('Open a collection first')
    return
  }
  if (requiredType === 'database' && !tab.dbName) {
    showToast('Open a database or collection first')
    return
  }
  contextMenu.value = {
    type: requiredType,
    label: tab.collectionName || tab.dbName || tab.connectionName,
    nodeData: {
      connId: tab.connectionId,
      connName: tab.connectionName,
      dbName: tab.dbName,
      collName: tab.collectionName,
    },
  }
  handleContextAction(action)
}

// What the native menu treats as "selected", so items enable/disable live. The
// context is the UNION of the active tab and the sidebar/tree selection: a
// collection tab satisfies all three, and so does a collection highlighted in the
// tree even while Quickstart is the active tab (the original bug). `anyConnection`
// is true whenever at least one connection is open — it gates View → Refresh,
// which refreshes every connection rather than one specific node.
const menuContext = computed(() => deriveMenuContext(
  tabs.value.find(t => t.id === activeTabId.value),
  treeSelection.value,
  treeConnectionCount.value,
  !!selectedIndex.value,
))

// Push the context down to the native menu so gated items enable/disable in step
// with the selection. Runs immediately for the initial (empty) state too.
watch(menuContext, (ctx) => {
  invoke('set_menu_context', {
    hasConnection: ctx.hasConnection,
    hasDatabase: ctx.hasDatabase,
    hasCollection: ctx.hasCollection,
    anyConnection: ctx.anyConnection,
    hasDocument: ctx.hasDocument,
    hasField: ctx.hasField,
    hasIndex: ctx.hasIndex,
  }).catch(() => {})
}, { immediate: true })

// The node a native menu action should act on: the sidebar selection when there
// is one (that's what the user just clicked in the tree), otherwise the active
// tab. Shaped like a tab so it drops straight into the existing handlers.
function menuTarget(requiredLevel = null) {
  return resolveMenuTarget(
    tabs.value.find(t => t.id === activeTabId.value),
    treeSelection.value,
    requiredLevel,
  )
}

// Routes menu-bar actions (emitted by id) to the same handlers the toolbar and
// right-click menus already use. The menu bar never emits a disabled item.
function handleMenuAction(id) {
  switch (id) {
    // --- direct modals / app ---
    case 'file:connect':     invoke('open_connect_window').catch(() => {}); return
    case 'file:exit':        appWindow.close(); return
    case 'edit:preferences': showPreferences.value = true; return
    case 'help:shortcuts':   showShortcuts.value = true; return
    case 'help:quickstart':  openQuickstart(); return
    case 'help:about':       showAbout.value = true; return
    case 'coll:vqb': {
      const tab = menuTarget('collection')
      if (!tab || tab.kind !== 'collection' || !tab.collectionName) {
        showToast('Open a collection first')
        return
      }
      openCollectionTab({
        connectionId: tab.connectionId,
        connectionName: tab.connectionName,
        dbName: tab.dbName,
        collectionName: tab.collectionName,
      })
      vqbOpen.value = true
      return
    }

    // --- toolbar dispatcher (targets the sidebar selection, else the active tab) ---
    case 'file:intellishell': handleTool('shell', menuTarget('database')); return
    case 'file:sql':          handleTool('sql'); return
    case 'file:search':       handleTool('search', menuTarget('database')); return
    case 'coll:open_tab':     handleTool('collection'); return
    case 'coll:export':       handleTool('export', menuTarget('collection')); return
    case 'coll:import':       handleTool('import', menuTarget('collection')); return
    case 'coll:mask':         handleTool('mask', menuTarget('collection')); return
    case 'coll:compare':      handleTool('compare', menuTarget('database')); return

    // --- server / connection scoped ---
    case 'file:server_status': menuNode('Server Status', 'connection'); return
    case 'file:server_build':  menuNode('Build Info', 'connection'); return
    case 'db:database_stats':  menuNode('Database Statistics', 'database'); return
    case 'db:current_ops':     menuNode('Current Operations', 'connection'); return

    // --- database scoped ---
    case 'db:add_collection':  menuNode('Add Collection…', 'database'); return
    case 'file:add_database':
    case 'db:add_database':    menuNode('Add Database…', 'connection'); return
    case 'db:add_view':        menuNode('Add View…', 'database'); return
    case 'coll:add_view':      menuNode('Add View Here…', 'collection'); return
    case 'coll:validator':     menuNode('Add / Edit Validator…', 'collection'); return
    case 'db:drop_database':   menuNode('Drop Database…', 'database'); return
    case 'gridfs:open':        menuNode('GridFS…', 'database'); return

    // --- collection scoped ---
    case 'coll:aggregation':   menuNode('Open Aggregation Editor', 'collection'); return
    case 'coll:add_index':     menuNode('Indexes…', 'collection'); return

    // --- index scoped (act on the row selected in the Indexes dialog) ---
    case 'idx:edit':   startEditIndex(); return
    case 'idx:view':   openIndexDetails(); return
    case 'idx:copy':   copyIndex(); return
    case 'idx:drop':   openDropIndexConfirm(); return
    case 'idx:hide':   setIndexHidden(true); return
    case 'idx:unhide': setIndexHidden(false); return
    case 'coll:stats':
    case 'db:collection_stats': menuNode('Collection Stats', 'collection'); return
    case 'coll:schema':        menuNode('View Schema', 'collection'); return
    case 'coll:rename':        menuNode('Rename Collection…', 'collection'); return
    case 'coll:duplicate':     menuNode('Duplicate Collection…', 'collection'); return
    case 'coll:drop':          menuNode('Drop Collection…', 'collection'); return

    // --- collection: document editing (open/activate a collection tab, then run) ---
    case 'coll:insert_document':
    case 'coll:update_dialog':
    case 'coll:delete_dialog':
    case 'coll:clear':
      requestCollectionDocAction(id); return

    // --- edit: clipboard copies act on the selected row/field in the active view ---
    case 'edit:copy':
    case 'edit:copy_value':
    case 'edit:copy_field':
    case 'edit:copy_field_path':
    case 'edit:copy_document':
      requestDocMenuAction(id); return

    // --- edit: paste inserts clipboard document(s) into the active collection ---
    case 'edit:paste_documents':
      requestCollectionDocAction(id); return

    // --- document: act on the selected row/field in the active results view ---
    case 'doc:edit_value':
    case 'doc:add_field':
    case 'doc:remove_field':
    case 'doc:rename_field':
    case 'doc:view_json':
    case 'doc:edit_json':
    case 'doc:delete':
      requestDocMenuAction(id); return

    // --- view ---
    case 'view:refresh':
      for (const conn of connectionTreeRef.value.getConnections()) {
        connectionTreeRef.value.refreshConn(conn.id)
      }
      showToast('Refreshed')
      return

    // Tab navigation/closing. Close Tab and Close Tab (No Prompt) behave the same
    // today — there is no unsaved-changes prompt to differ on yet.
    case 'view:next_tab':      cycleTab(1); return
    case 'view:prev_tab':      cycleTab(-1); return
    case 'view:close_tab':
    case 'view:close_tab_np':
      if (activeTabId.value != null) closeTab(activeTabId.value)
      return

    // Results view mode + Refresh Document act on the active collection tab's
    // ResultsPanel; signal it directly (no row selection required).
    case 'view:tree':
    case 'view:table':
    case 'view:json':
    case 'view:refresh_document':
    case 'view:step_column':
    case 'view:step_cell':
    case 'view:step_out': {
      const tab = tabs.value.find(t => t.id === activeTabId.value)
      if (!tab || tab.kind !== 'collection') { showToast('Open a collection tab first'); return }
      docMenuRequest.value = { action: id, nonce: Date.now() }
      return
    }

    // Toggle the global toolbar. The native menu label stays "Hide Global Toolbar";
    // a toast reports the resulting state.
    case 'view:hide_toolbar':
      toolbarHidden.value = !toolbarHidden.value
      showToast(toolbarHidden.value ? 'Toolbar hidden' : 'Toolbar shown')
      return

    // History Manager: open the active collection tab's query-history menu.
    case 'view:history': {
      const tab = tabs.value.find(t => t.id === activeTabId.value)
      if (!tab || tab.kind !== 'collection') { showToast('Open a collection tab first'); return }
      historyRequest.value = { nonce: Date.now() }
      return
    }
  }
}

// Route a Document-menu action to the active collection tab's ResultsPanel, which
// owns the field/document editors. The Document gates already guarantee an active
// collection tab with a selected row/field, so this only needs to signal the panel.
function requestDocMenuAction(action) {
  const tab = tabs.value.find(t => t.id === activeTabId.value)
  if (!tab || tab.kind !== 'collection' || (tab.selectedRow ?? -1) < 0) {
    showToast('Select a document in the results first')
    return
  }
  docMenuRequest.value = { action: action, nonce: Date.now() }
}

// Route a Collection document-editing action (Insert / Update / Delete dialog, Clear)
// to a collection's ResultsPanel. Resolve the target collection (sidebar selection or
// active tab), open it as a tab so its results view exists and can refresh, then — once
// that tab is mounted — signal the panel to open the matching dialog.
async function requestCollectionDocAction(action) {
  const target = menuTarget('collection')
  if (!target || target.kind !== 'collection' || !target.collectionName) {
    showToast('Open a collection first')
    return
  }
  const active = tabs.value.find(t => t.id === activeTabId.value)
  const sameCollectionActive = active && active.kind === 'collection'
    && active.connectionId === target.connectionId
    && active.dbName === target.dbName
    && active.collectionName === target.collectionName
  // Reuse the active tab when it already shows this collection; otherwise open one so
  // the operation has a results view to refresh afterward.
  if (!sameCollectionActive) {
    openCollectionTab({
      connectionId: target.connectionId,
      connectionName: target.connectionName,
      dbName: target.dbName,
      collectionName: target.collectionName,
    })
  }
  await nextTick()
  docMenuRequest.value = { action: action, nonce: Date.now() }
}

// The menu bar's keyboard shortcuts, used on Linux only (elsewhere the native
// menu's accelerators own them — see NATIVE_MENU_OWNS_SHORTCUTS). We skip when
// focus is in a text field or code editor so the webview keeps its native editing
// keys (the WebKitGTK swallow trap), and only claim our specific combos.
function onGlobalKeydown(e) {
  const t = e.target
  if (t && t.closest && t.closest('input, textarea, [contenteditable], .cm-editor, .monaco-editor')) {
    return
  }
  const mod = e.ctrlKey || e.metaKey
  let id = null
  if (mod && !e.altKey) {
    const k = e.key.toLowerCase()
    if (k === 'n' && !e.shiftKey) id = 'file:connect'
    else if (k === 'l' && e.shiftKey) id = 'file:sql'
    else if (k === 'l' && !e.shiftKey) id = 'file:intellishell'
    else if (k === 'p' && !e.shiftKey) id = 'edit:preferences'
    else if (k === 'b' && !e.shiftKey) id = 'coll:vqb'
    else if (k === 'r' && !e.shiftKey) id = 'view:refresh'
  } else if (!mod && !e.altKey && !e.shiftKey) {
    if (e.key === 'F4') id = 'coll:aggregation'
    else if (e.key === 'F10') id = 'coll:open_tab'
  }
  if (id) {
    e.preventDefault()
    handleMenuAction(id)
  }
}

function onManagerConnect(id) {
  showConnectionManager.value = false
  expandConnectionId.value = id
}

async function handleContextAction(action) {
  const saved = contextMenu.value
  contextMenu.value = null

  // Tab context menu (right-click on a tab) routes to its own handler.
  if (saved.type === 'tab') {
    handleTabAction(action, saved.nodeData.tabId)
    return
  }

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

  // Add View… (database node) and Add View Here… (collection node — prefills the
  // source with the clicked collection). Both create a view in the same database.
  if (action === 'Add View…' || action === 'Add View Here…') {
    addViewTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName }
    newViewName.value = ''
    newViewSource.value = action === 'Add View Here…' ? (saved.nodeData.collName || '') : ''
    newViewPipeline.value = ''
    addViewError.value = null
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
    selectedIndex.value = null
    pendingDropIndex.value = null
    resetIndexForm()
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

  if (action === 'Server Status' && saved.type === 'connection') {
    serverStatusTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
    }
    return
  }

  if (action === 'Current Operations' && saved.type === 'connection') {
    currentOpsTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
    }
    return
  }

  if (action === 'Database Statistics' && saved.type === 'database') {
    dbStatsTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
    }
    return
  }

  if (action === 'Add / Edit Validator…' && saved.type === 'collection') {
    validatorTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    return
  }

  if (action === 'View Schema' && saved.type === 'collection') {
    schemaTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    return
  }

  const serverInfoKinds = {
    'Build Info': 'build',
    'Host Info': 'host',
    'Replica Set Status': 'replica',
  }
  if (serverInfoKinds[action] && saved.type === 'connection') {
    serverInfoTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      kind: serverInfoKinds[action],
      title: action,
    }
    return
  }

  if (action === 'Collection Stats' && saved.type === 'collection') {
    statsTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    return
  }

  if (action === 'Duplicate Collection…' && saved.type === 'collection') {
    duplicateCollectionTarget.value = {
      connId: saved.nodeData.connId,
      dbName: saved.nodeData.dbName,
      collName: saved.nodeData.collName,
    }
    duplicateCollectionName.value = saved.nodeData.collName + '_copy'
    duplicateCollectionError.value = null
    return
  }

  if (action === 'GridFS…' && saved.type === 'database') {
    gridfsTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
    }
    return
  }

  if (action === 'Search in…' && saved.type === 'database') {
    searchTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
    }
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
    addCollectionError.value = errMessage(e)
  } finally {
    addCollectionSaving.value = false
  }
}

async function confirmAddView() {
  const target = addViewTarget.value
  const name = newViewName.value.trim()
  const source = newViewSource.value.trim()
  if (!target || !name || !source) return
  // Validate the (optional) pipeline up front so a typo surfaces before the round-trip.
  const pp = parsePipeline(newViewPipeline.value)
  if (!pp.ok) { addViewError.value = pp.error; return }
  addViewSaving.value = true
  addViewError.value = null
  try {
    await invoke('create_view', {
      id: target.connId,
      database: target.dbName,
      name: name,
      viewOn: source,
      pipeline: pp.ejson,
    })
    await connectionTreeRef.value.refreshConn(target.connId)
    showToast(`View "${name}" created`)
    addViewTarget.value = null
  } catch (e) {
    addViewError.value = errMessage(e)
  } finally {
    addViewSaving.value = false
  }
}

// The Validator modal owns its own fetch/save; we just confirm the result.
function onValidatorSaved(collName) {
  showToast(`Validator saved for "${collName}"`)
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
    dropDatabaseError.value = errMessage(e)
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
    dropCollectionError.value = errMessage(e)
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
    renameCollectionError.value = errMessage(e)
  } finally {
    renameCollectionSaving.value = false
  }
}

async function confirmDuplicateCollection() {
  const target = duplicateCollectionTarget.value
  const name = duplicateCollectionName.value.trim()
  if (!target || !name || name === target.collName) return
  duplicateCollectionSaving.value = true
  duplicateCollectionError.value = null
  try {
    const count = await invoke('duplicate_collection', {
      id: target.connId,
      database: target.dbName,
      source: target.collName,
      target: name,
    })
    await connectionTreeRef.value.refreshConn(target.connId)
    showToast(`Copied ${count} document${count === 1 ? '' : 's'} to "${name}"`)
    duplicateCollectionTarget.value = null
  } catch (e) {
    duplicateCollectionError.value = errMessage(e)
  } finally {
    duplicateCollectionSaving.value = false
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
    addDatabaseError.value = errMessage(e)
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
    // Re-point the selection at the reloaded index object (a fresh list replaces
    // the old references); clear it if that index no longer exists.
    if (selectedIndex.value) {
      selectedIndex.value = indexesList.value.find(i => i.name === selectedIndex.value.name) || null
    }
  } catch (e) {
    indexesError.value = errMessage(e)
    indexesList.value = []
  } finally {
    indexesLoading.value = false
  }
}

// Reset the create/edit form back to a blank create.
function resetIndexForm() {
  newIndexKeys.value = ''
  newIndexName.value = ''
  newIndexUnique.value = false
  indexFormMode.value = 'create'
  indexEditOriginalName.value = ''
}

// Closes the Indexes dialog and clears its selection/form so the Index menu
// disables again (and any half-typed edit is discarded).
function closeIndexesModal() {
  indexesTarget.value = null
  selectedIndex.value = null
  pendingDropIndex.value = null
  resetIndexForm()
}

async function confirmCreateIndex() {
  const target = indexesTarget.value
  const keys = newIndexKeys.value.trim()
  if (!target || !keys) return
  const editing = indexFormMode.value === 'edit' && !!indexEditOriginalName.value
  indexCreating.value = true
  indexesError.value = null
  try {
    // MongoDB has no in-place index edit, so an edit drops the original first and
    // recreates it from the (possibly changed) form values.
    if (editing) {
      await invoke('drop_index', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
        name: indexEditOriginalName.value,
      })
    }
    await invoke('create_index', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
      keys: keys,
      unique: newIndexUnique.value,
      name: newIndexName.value.trim(),
    })
    resetIndexForm()
    await loadIndexes()
    showToast(editing ? 'Index updated' : 'Index created')
  } catch (e) {
    indexesError.value = errMessage(e)
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
    indexesError.value = errMessage(e)
    pendingDropIndex.value = null
  }
}

// --- Index menu actions (operate on the selected row in the Indexes dialog) ---

// The selected index, or null with a nudge if somehow invoked without one. The
// Index-menu gate guarantees a selection, so this is just defensive.
function requireSelectedIndex() {
  if (!indexesTarget.value || !selectedIndex.value) {
    showToast('Select an index first')
    return null
  }
  return selectedIndex.value
}

// Edit Index…: pre-fill the create form with the selected index as a starting
// point and switch it to edit mode (save = drop-and-recreate).
function startEditIndex() {
  const idx = requireSelectedIndex()
  if (!idx) return
  if (isProtectedIndex(idx.name)) {
    showToast('The _id index cannot be edited')
    return
  }
  newIndexKeys.value = indexKeyLabel(idx) ? JSON.stringify(idx.key) : ''
  newIndexName.value = idx.name || ''
  newIndexUnique.value = !!idx.unique
  indexFormMode.value = 'edit'
  indexEditOriginalName.value = idx.name
}

// View Details: show the full spec (read-only) plus usage stats when available.
async function openIndexDetails() {
  const idx = requireSelectedIndex()
  if (!idx) return
  const target = indexesTarget.value
  indexDetailsTarget.value = idx
  indexDetailsStats.value = null
  indexDetailsLoading.value = true
  try {
    const all = await invoke('index_stats', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
    })
    indexDetailsStats.value = all.find(s => s.name === idx.name) || null
  } catch (e) {
    // $indexStats can be unsupported (older server, non-replicated deployment);
    // the spec is still shown, just without usage numbers.
    indexDetailsStats.value = null
  } finally {
    indexDetailsLoading.value = false
  }
}

// $indexStats.accesses.since is a BSON date, which crosses the wire as relaxed
// Extended JSON (a string, or a { $date } wrapper). Render whichever we get as a
// plain string rather than "[object Object]".
function formatIndexSince(value) {
  if (value == null) return '—'
  if (typeof value === 'object') {
    const inner = value.$date
    if (inner == null) return JSON.stringify(value)
    return typeof inner === 'object' ? (inner.$numberLong ?? JSON.stringify(inner)) : inner
  }
  return value
}

// Copy Index: put the full index definition on the clipboard as pretty JSON.
function copyIndex() {
  const idx = requireSelectedIndex()
  if (!idx) return
  navigator.clipboard.writeText(indexSpecJson(idx))
  showToast('Index copied')
}

// Drop Index: open the type-to-confirm dialog; never for the _id_ index.
function openDropIndexConfirm() {
  const idx = requireSelectedIndex()
  if (!idx) return
  if (isProtectedIndex(idx.name)) {
    showToast('The _id index cannot be dropped')
    return
  }
  dropIndexTarget.value = { name: idx.name }
  dropIndexConfirmText.value = ''
  dropIndexError.value = null
}

async function confirmDropIndex() {
  const target = indexesTarget.value
  const drop = dropIndexTarget.value
  if (!target || !drop) return
  if (dropIndexConfirmText.value !== drop.name) return
  dropIndexBusy.value = true
  dropIndexError.value = null
  try {
    await invoke('drop_index', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
      name: drop.name,
    })
    dropIndexTarget.value = null
    await loadIndexes()
    showToast(`Index "${drop.name}" dropped`)
  } catch (e) {
    dropIndexError.value = errMessage(e)
  } finally {
    dropIndexBusy.value = false
  }
}

// Hide / Unhide Index: toggle the planner-visibility flag without a rebuild.
async function setIndexHidden(hidden) {
  const idx = requireSelectedIndex()
  if (!idx) return
  if (isProtectedIndex(idx.name)) {
    showToast('The _id index cannot be hidden')
    return
  }
  const target = indexesTarget.value
  const name = idx.name
  indexesError.value = null
  try {
    await invoke('set_index_hidden', {
      id: target.connId,
      database: target.dbName,
      collection: target.collName,
      name: name,
      hidden: hidden,
    })
    await loadIndexes()
    showToast(hidden ? `Index "${name}" hidden` : `Index "${name}" unhidden`)
  } catch (e) {
    indexesError.value = errMessage(e)
  }
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
    showToast('Export failed: ' + errMessage(e))
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
    showToast('Export failed: ' + errMessage(e))
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
    showToast('Import failed: ' + errMessage(e))
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
    showToast('Import failed: ' + errMessage(e))
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
    filter: '', projection: '', sort: '', skip: 0, limit: defaultQueryLimit.value,
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
    runQuery(id, { filter: '{}', projection: '{}', sort: '{}', skip: 0, limit: defaultQueryLimit.value })
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

// Help → Quickstart: focus the existing Quickstart tab, or open one if it was closed.
function openQuickstart() {
  const existing = tabs.value.find(t => t.kind === 'quickstart')
  if (existing) {
    activateTab(existing.id)
    return
  }
  const id = 't' + Date.now()
  tabs.value.push({ id: id, kind: 'quickstart', title: 'Quickstart' })
  activateTab(id)
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

// ── tab context menu ───────────────────────────────────────
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
      runError: null, elapsedMs: null, drillPath: [], hasRun: false, selectedRow: -1,
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
    results: [], hasRun: false, isRunning: false, runError: null,
    selectedRow: -1, elapsedMs: null,
  }
  tabs.value.push(dup)
  activeTabId.value = id
  runRestoredTab(dup)   // re-run from the cloned query state (find mode only)
}

// ── rename tab dialog ──────────────────────────────────────
const renameTabTarget = ref(null)   // id of the tab being renamed
const renameTabValue = ref('')
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

// ── query execution ────────────────────────────────────────
// A unique tag stamped on each query op (as its `comment`) so a cancel can find
// and kill exactly that operation server-side.
function newRunId() {
  return 'q' + Date.now() + '-' + Math.random().toString(36).slice(2, 8)
}

// Best-effort cancel of a tab's in-flight query: ask the server to kill the op
// tagged with this run's id. The awaited find/aggregate then rejects, which the
// run handlers render as a calm "cancelled" state (because tab.cancelled is set).
async function cancelQuery(tabId) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (!tab || !tab.isRunning || !tab.runId) return
  tab.cancelled = true
  try {
    const killed = await invoke('kill_query', { id: tab.connectionId, comment: tab.runId })
    showToast(killed > 0 ? 'Query cancelled' : 'Query already finished')
  } catch (e) {
    tab.cancelled = false
    showToast('Cancel not permitted on this server: ' + errMessage(e))
  }
}

async function runQuery(tabId, params) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (!tab) return
  tab.isRunning = true
  tab.runError = null
  tab.runErrorCode = null
  tab.cancelled = false
  const runId = newRunId()
  tab.runId = runId
  const t0 = Date.now()
  const { addToHistory = true, ...queryParams } = params
  try {
    tab.results = await invoke('find_documents', {
      id:         tab.connectionId,
      database:   tab.dbName,
      collection: tab.collectionName,
      ...queryParams,
      comment:    runId,
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
    // A deliberate cancel makes the killed op error — show a calm state, not a scary one.
    if (tab.cancelled) {
      tab.runError = 'Query cancelled.'
      tab.runErrorCode = null
    } else {
      tab.runError = errMessage(e)
      tab.runErrorCode = errCode(e)
    }
  } finally {
    tab.isRunning = false
  }
}

async function runAggregate(tabId, params) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (!tab) return
  tab.isRunning = true
  tab.runError = null
  tab.runErrorCode = null
  tab.cancelled = false
  const runId = newRunId()
  tab.runId = runId
  const t0 = Date.now()
  try {
    tab.results = await invoke('run_aggregate', {
      id:         tab.connectionId,
      database:   tab.dbName,
      collection: tab.collectionName,
      ...params,
      comment:    runId,
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
    if (tab.cancelled) {
      tab.runError = 'Query cancelled.'
      tab.runErrorCode = null
    } else {
      tab.runError = errMessage(e)
      tab.runErrorCode = errCode(e)
    }
  } finally {
    tab.isRunning = false
  }
}
</script>

<template>
  <div class="app-layout">
    <!-- The menu bar is the native OS menu (installed from src-tauri/src/menu.rs);
         see handleMenuAction for how its clicks are routed back into the app. -->

    <!-- Toolbar -->
    <div class="toolbar" v-show="!toolbarHidden">
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
        @select-node="treeSelection = $event"
        @connections-changed="treeConnectionCount = $event"
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
        :doc-menu-request="docMenuRequest"
        :history-request="historyRequest"
        @activate-tab="activateTab"
        @close-tab="closeTab"
        @tab-context="onTabContext"
        @run-query="runQuery"
        @run-aggregate="runAggregate"
        @cancel-query="cancelQuery"
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
      @toast="showToast"
    />

    <!-- Server Status modal -->
    <ServerStatusModal
      v-if="serverStatusTarget"
      :target="serverStatusTarget"
      @close="serverStatusTarget = null"
    />

    <DatabaseStatsModal
      v-if="dbStatsTarget"
      :target="dbStatsTarget"
      @close="dbStatsTarget = null"
    />

    <CurrentOpsModal
      v-if="currentOpsTarget"
      :target="currentOpsTarget"
      @close="currentOpsTarget = null"
    />

    <ValidatorModal
      v-if="validatorTarget"
      :target="validatorTarget"
      @saved="onValidatorSaved"
      @close="validatorTarget = null"
    />

    <!-- Schema (View Schema) modal -->
    <SchemaModal
      v-if="schemaTarget"
      :target="schemaTarget"
      @close="schemaTarget = null"
    />

    <!-- SQL → MQL translator -->
    <SqlModal
      v-if="showSqlModal"
      @close="showSqlModal = false"
    />

    <!-- Data Masking modal -->
    <MaskingModal
      v-if="maskingTarget"
      :target="maskingTarget"
      @toast="showToast"
      @close="maskingTarget = null"
    />

    <!-- Collection Stats modal -->
    <StatsModal
      v-if="statsTarget"
      :target="statsTarget"
      @close="statsTarget = null"
    />

    <!-- Build / Host / Replica Set info modal -->
    <ServerInfoModal
      v-if="serverInfoTarget"
      :target="serverInfoTarget"
      @close="serverInfoTarget = null"
    />

    <!-- SQL Migration modal -->
    <MigrationModal
      v-if="migrationTarget"
      :target="migrationTarget"
      @close="migrationTarget = null"
    />

    <!-- Global Search modal -->
    <SearchModal
      v-if="searchTarget"
      :target="searchTarget"
      @close="searchTarget = null"
    />

    <!-- GridFS modal -->
    <GridFsModal
      v-if="gridfsTarget"
      :target="gridfsTarget"
      @toast="showToast"
      @close="gridfsTarget = null"
    />

    <!-- Data Compare modal -->
    <CompareModal
      v-if="compareTarget"
      :target="compareTarget"
      @close="compareTarget = null"
    />

    <!-- Keyboard Shortcuts reference -->
    <ShortcutsModal
      v-if="showShortcuts"
      @close="showShortcuts = false"
    />

    <!-- About -->
    <AboutModal
      v-if="showAbout"
      @close="showAbout = false"
    />

    <!-- Preferences -->
    <PreferencesModal
      v-if="showPreferences"
      :default-query-limit="defaultQueryLimit"
      :theme="theme"
      @close="showPreferences = false"
      @saved="defaultQueryLimit = $event.defaultQueryLimit; applyTheme($event.theme)"
      @open-shortcuts="showPreferences = false; showShortcuts = true"
    />

    <!-- SSH host-key trust prompt / changed-key warning -->
    <SshHostKeyModal
      :prompt="sshHostKeyPrompt"
      :changed="sshHostKeyChanged"
      @trust="onHostKeyTrust"
      @cancel="onHostKeyCancel"
      @forget="onHostKeyForget"
      @dismiss="sshHostKeyChanged = null"
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

    <!-- Add View modal -->
    <div v-if="addViewTarget" class="del-overlay" @mousedown.self="addViewTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add View</div>
          <button class="close-btn" @click="addViewTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newViewName"
            class="prompt-input"
            placeholder="View name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <input
            v-model="newViewSource"
            class="prompt-input"
            placeholder="Source collection (viewOn)"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
          />
          <textarea
            v-model="newViewPipeline"
            class="prompt-input pipeline-input"
            placeholder="Aggregation pipeline (optional), e.g. [ { &quot;$match&quot;: { &quot;active&quot;: true } } ]"
            spellcheck="false"
          ></textarea>
          <div v-if="addViewError" class="del-error">{{ addViewError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addViewTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newViewName.trim() || !newViewSource.trim() || addViewSaving" @click="confirmAddView">
            {{ addViewSaving ? 'Creating…' : 'Create' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rename Tab modal -->
    <div v-if="renameTabTarget" class="del-overlay" @mousedown.self="renameTabTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Rename Tab</div>
          <button class="close-btn" @click="renameTabTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="renameTabValue"
            class="prompt-input"
            placeholder="Tab name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmRenameTab"
            @keydown.escape="renameTabTarget = null"
          />
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="renameTabTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!renameTabValue.trim()" @click="confirmRenameTab">Rename</button>
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

    <!-- Duplicate Collection prompt -->
    <div v-if="duplicateCollectionTarget" class="del-overlay" @mousedown.self="duplicateCollectionTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Duplicate Collection</div>
          <button class="close-btn" @click="duplicateCollectionTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="duplicateCollectionName"
            class="prompt-input"
            placeholder="New collection name"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmDuplicateCollection"
          />
          <div v-if="duplicateCollectionError" class="del-error">{{ duplicateCollectionError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="duplicateCollectionTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!duplicateCollectionName.trim() || duplicateCollectionName.trim() === duplicateCollectionTarget.collName || duplicateCollectionSaving" @click="confirmDuplicateCollection">
            {{ duplicateCollectionSaving ? 'Duplicating…' : 'Duplicate' }}
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
    <div v-if="indexesTarget" class="del-overlay" @mousedown.self="closeIndexesModal()">
      <div class="del-dialog idx-dialog">
        <div class="del-title">
          <div class="t">Indexes — {{ indexesTarget.collName }}</div>
          <button class="close-btn" @click="closeIndexesModal()">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <div v-if="indexesLoading" class="idx-msg">Loading indexes…</div>
          <table v-else-if="indexesList.length" class="idx-table">
            <thead>
              <tr><th>Name</th><th>Keys</th><th>Unique</th><th>Hidden</th><th></th></tr>
            </thead>
            <tbody>
              <tr
                v-for="idx in indexesList"
                :key="idx.name"
                class="idx-row"
                :class="{ selected: selectedIndex && selectedIndex.name === idx.name }"
                @click="selectedIndex = idx"
              >
                <td class="idx-name">{{ idx.name }}</td>
                <td class="idx-keys">{{ indexKeyLabel(idx) }}</td>
                <td>{{ idx.unique ? 'Yes' : '—' }}</td>
                <td>{{ isIndexHidden(idx) ? 'Yes' : '—' }}</td>
                <td class="idx-actions">
                  <button
                    v-if="idx.name !== '_id_'"
                    class="btn"
                    :class="{ danger: pendingDropIndex === idx.name }"
                    @click.stop="dropIndex(idx.name)"
                  >{{ pendingDropIndex === idx.name ? 'Confirm' : 'Drop' }}</button>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="idx-msg">No indexes.</div>
          <div class="idx-hint">Select an index row to enable the Index menu.</div>

          <div class="idx-create">
            <div class="idx-create-title">{{ indexFormMode === 'edit' ? 'Edit index' : 'Create index' }}</div>
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
            <button v-if="indexFormMode === 'edit'" class="btn idx-cancel-edit" @click="resetIndexForm()">
              Cancel edit
            </button>
          </div>

          <div v-if="indexesError" class="del-error">{{ indexesError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="closeIndexesModal()">Close</button>
          <button class="btn primary" :disabled="!newIndexKeys.trim() || indexCreating" @click="confirmCreateIndex">
            {{ indexCreating ? (indexFormMode === 'edit' ? 'Saving…' : 'Creating…') : (indexFormMode === 'edit' ? 'Save changes' : 'Create index') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Index: View Details (read-only) -->
    <div v-if="indexDetailsTarget" class="del-overlay" @mousedown.self="indexDetailsTarget = null">
      <div class="del-dialog idx-dialog">
        <div class="del-title">
          <div class="t">Index Details — {{ indexDetailsTarget.name }}</div>
          <button class="close-btn" @click="indexDetailsTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <div class="idx-detail-section">Definition</div>
          <pre class="idx-detail-json">{{ indexSpecJson(indexDetailsTarget) }}</pre>
          <div class="idx-detail-section">Usage</div>
          <div v-if="indexDetailsLoading" class="idx-msg">Loading usage…</div>
          <table v-else-if="indexDetailsStats" class="idx-detail-stats">
            <tr><td>Operations</td><td>{{ indexDetailsStats.accesses?.ops ?? '—' }}</td></tr>
            <tr><td>Tracking since</td><td>{{ formatIndexSince(indexDetailsStats.accesses?.since) }}</td></tr>
          </table>
          <div v-else class="idx-msg">Usage statistics unavailable.</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="indexDetailsTarget = null">Close</button>
        </div>
      </div>
    </div>

    <!-- Index: Drop confirmation (type the name to confirm) -->
    <div v-if="dropIndexTarget" class="del-overlay" @mousedown.self="dropIndexTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Drop Index</div>
          <button class="close-btn" @click="dropIndexTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <p>This permanently drops the index
            <code>{{ dropIndexTarget.name }}</code>. Queries that relied on it may slow down.
            This cannot be undone.</p>
          <p class="cc-prompt">Type <code>{{ dropIndexTarget.name }}</code> to confirm:</p>
          <input
            class="prompt-input"
            v-model="dropIndexConfirmText"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmDropIndex"
          />
          <div v-if="dropIndexError" class="del-error">{{ dropIndexError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="dropIndexTarget = null">Cancel</button>
          <button
            class="btn danger"
            :disabled="dropIndexBusy || dropIndexConfirmText !== dropIndexTarget.name"
            @click="confirmDropIndex"
          >{{ dropIndexBusy ? 'Dropping…' : 'Drop Index' }}</button>
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
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 14px;
  position: relative;
  -webkit-app-region: drag;
}
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
  background: var(--bg-hover);
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
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.del-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
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
.del-error { font-size: 12px; color: var(--danger-text); margin-top: 6px; }
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
.pipeline-input {
  height: auto;
  min-height: 76px;
  padding: 8px 10px;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  resize: vertical;
}
/* Space stacked inputs in multi-field dialogs (e.g. Add View); single-input
   dialogs like Add Collection are unaffected. */
.del-body .prompt-input + .prompt-input { margin-top: 10px; }

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
.idx-cancel-edit { margin-top: 10px; }
.idx-row { cursor: pointer; }
.idx-row:hover { background: var(--bg-hover); }
.idx-row.selected { background: var(--accent-soft, var(--bg-hover)); }
.idx-hint { margin-top: 8px; font-size: 11.5px; color: var(--text-faint); }
.cc-prompt { margin-top: 12px; font-size: 12.5px; color: var(--text-dim); }
.cc-prompt code, .del-body code { font-family: var(--mono); }
.idx-detail-section { font-size: 12px; color: var(--text-faint); margin: 14px 0 6px; }
.idx-detail-section:first-child { margin-top: 0; }
.idx-detail-json {
  margin: 0;
  padding: 10px;
  max-height: 220px;
  overflow: auto;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  font-family: var(--mono);
  font-size: 12px;
  color: var(--text);
  white-space: pre;
}
.idx-detail-stats { width: 100%; border-collapse: collapse; font-size: 12.5px; }
.idx-detail-stats td { padding: 4px 8px; border-bottom: 1px solid var(--border-soft); }
.idx-detail-stats td:first-child { color: var(--text-faint); width: 40%; }
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
.btn.danger { background: var(--danger); color: #fff; }
.btn.danger:hover:not(:disabled) { background: var(--danger-hover); }
</style>
