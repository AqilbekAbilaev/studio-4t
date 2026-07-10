<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { installInputUndo } from './utils/inputUndo'
import { parseField, parsePipeline } from './utils/queryParser'
import { errMessage } from './utils/errors'
import { deriveMenuContext, resolveMenuTarget } from './utils/menuContext'
import { isProtectedIndex, indexKeyLabel, indexSpecJson, isIndexHidden } from './utils/indexSpec'
import { useIndexes } from './composables/useIndexes'
import { useSshHostKey } from './composables/useSshHostKey'
import { useQueryRunner } from './composables/useQueryRunner'
import BaseIcon from './components/BaseIcon.vue'
import ConnectionTree from './components/ConnectionTree.vue'
import QueryWorkspace from './components/QueryWorkspace.vue'
import SplitContainer from './components/SplitContainer.vue'
import ConnectionManager from './components/ConnectionManager.vue'
import ContextMenu from './components/ContextMenu.vue'
import SshHostKeyModal from './components/SshHostKeyModal.vue'
import ServerStatusModal from './components/ServerStatusModal.vue'
import DatabaseStatsModal from './components/DatabaseStatsModal.vue'
import CurrentOpsModal from './components/CurrentOpsModal.vue'
import ProfilerModal from './components/ProfilerModal.vue'
import ValidatorModal from './components/ValidatorModal.vue'
import AboutModal from './components/AboutModal.vue'
import UsersModal from './components/UsersModal.vue'
import RolesModal from './components/RolesModal.vue'
import FunctionsModal from './components/FunctionsModal.vue'
import MapReduceModal from './components/MapReduceModal.vue'
import ServerChartsModal from './components/ServerChartsModal.vue'
import SchemaModal from './components/SchemaModal.vue'
import SqlModal from './components/SqlModal.vue'
import TasksModal from './components/TasksModal.vue'
import MaskingModal from './components/MaskingModal.vue'
import ImportExportWizard from './components/ImportExportWizard.vue'
import ReschemaModal from './components/ReschemaModal.vue'
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
            dbName: t.dbName, code: t.code,
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

onMounted(async () => {
  // WebKitGTK has no native undo/redo for text fields — install our own so Ctrl+Z works.
  installInputUndo()

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

  // Restore persisted database/collection colour tags so they survive a restart.
  // Connection tags come back on each connection (conn.tag) via list_connections.
  try {
    const nodeTags = await invoke('get_node_tags')
    if (nodeTags) tagOverrides.value = { ...nodeTags, ...tagOverrides.value }
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
              paneId: t.paneId || 'p0',
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
        // Restore a saved two-pane split when both panes still point at live tabs.
        if (session.splitOrientation && Array.isArray(session.panes) && session.panes.length === 2
            && session.panes.every(p => tabs.value.some(t => t.id === p.activeTabId))) {
          panes.value = session.panes.map((p, i) => ({ id: 'p' + i, activeTabId: p.activeTabId }))
          splitOrientation.value = session.splitOrientation === 'horizontal' ? 'horizontal' : 'vertical'
          focusedPaneId.value = panes.value.some(p => p.id === session.focusedPaneId) ? session.focusedPaneId : 'p0'
        } else {
          // No split restored — collapse every tab into the single pane so a tab
          // saved as p1 doesn't end up orphaned with no pane to show it.
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
  { id: 't0', kind: 'quickstart', title: 'Quickstart', paneId: 'p0' }
])

// The workspace can be split into two panes. `tabs` above stays the single source
// of truth, but each tab is tagged with the pane that owns it (`paneId`), so a pane
// shows only its own tabs — splitting moves the active tab into the new pane rather
// than duplicating the whole strip. Each pane tracks its active tab; new tabs and
// menu actions target the focused pane. `activeTabId` is a get/set alias for the
// focused pane's active tab, so every existing caller keeps working unchanged.
const panes = ref([{ id: 'p0', activeTabId: 't0' }])
const splitOrientation = ref(null)   // null | 'vertical' | 'horizontal'
const focusedPaneId = ref('p0')
const isSplit = computed(() => panes.value.length > 1)

// The tabs owned by each pane (in their `tabs` order). A tab with no paneId (e.g.
// an older persisted session) belongs to the first pane.
const paneATabs = computed(() => tabs.value.filter(t => (t.paneId || 'p0') === 'p0'))
const paneBTabs = computed(() => tabs.value.filter(t => (t.paneId || 'p0') === 'p1'))

const activeTabId = computed({
  get() {
    const pane = panes.value.find(p => p.id === focusedPaneId.value) || panes.value[0]
    return pane ? pane.activeTabId : null
  },
  set(id) {
    const pane = panes.value.find(p => p.id === focusedPaneId.value) || panes.value[0]
    if (pane) pane.activeTabId = id
  },
})

// Keep each pane's active tab valid: when its tabs change (close, or a move between
// panes), a pane pointing at a tab it no longer owns falls back to its last tab (or
// nothing). Keyed on the pane→tab mapping so both closes and moves re-run it.
watch(() => tabs.value.map(t => (t.paneId || 'p0') + ':' + t.id).join('|'), () => {
  for (const pane of panes.value) {
    const owned = tabs.value.filter(t => (t.paneId || 'p0') === pane.id)
    if (pane.activeTabId == null || !owned.some(t => t.id === pane.activeTabId)) {
      pane.activeTabId = owned.length ? owned[owned.length - 1].id : null
    }
  }
  // A split pane that has lost all its tabs collapses the split — the workspace
  // returns to a single pane showing whichever pane still has tabs.
  if (isSplit.value) {
    const empty = panes.value.find(p => !tabs.value.some(t => (t.paneId || 'p0') === p.id))
    if (empty) collapseToPane(panes.value.find(p => p.id !== empty.id))
  }
})
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
const browserRequest = ref(null)      // File → Load: { nonce } signal to open the saved-query browser
const saveQueryRequest = ref(null)    // File → Save: { nonce } signal to open the save-query form
const showConnectionManager = ref(false)
const serverStatusTarget = ref(null)  // { connId, connName } when the Server Status modal is open
const dbStatsTarget = ref(null)       // { connId, connName, dbName } when the Database Statistics modal is open
const currentOpsTarget = ref(null)    // { connId, connName } when the Current Operations modal is open
const profilerTarget = ref(null)      // { connId, connName, dbName } when the Query Profiler modal is open
const validatorTarget = ref(null)     // { connId, connName, dbName, collName } when the Validator modal is open
const usersTarget = ref(null)         // { connId, connName, dbName } when the Users modal is open
const rolesTarget = ref(null)         // { connId, connName, dbName } when the Roles modal is open
const functionsTarget = ref(null)     // { connId, connName, dbName } when the Stored Functions modal is open
const mapReduceTarget = ref(null)     // { connId, connName, dbName, collName } when the Map-Reduce modal is open
const dbClipboard = ref(null)         // Copy/Paste: { kind: 'collection'|'database', connId, connName, dbName, collName? }
const serverChartsTarget = ref(null)  // { connId, connName } when the Server Status Charts modal is open
const migrationTarget = ref(null)     // { connId, connName, dbName, collName } for the SQL Migration modal
const searchTarget = ref(null)        // { connId, connName, dbName } for the Global Search modal
const gridfsTarget = ref(null)        // { connId, connName, dbName } for the GridFS modal
const gridfsRequest = ref(null)       // { action, nonce } signal to the open GridFS modal
const compareTarget = ref(null)       // { connId, connName, dbName } for the Data Compare modal
const schemaTarget = ref(null)  // { connId, connName, dbName, collName } when the Schema modal is open
const showSqlModal = ref(false)       // SQL → MQL translator modal (top-bar SQL button)
const showTasksModal = ref(false)     // Tasks panel (top-bar Tasks button / File → Open Tasks)
const maskingTarget = ref(null)       // { connId, connName, dbName, collName } for the Data Masking modal
const importWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Import wizard
const exportWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Export wizard
const reschemaTarget = ref(null)      // { connId, connName, dbName, collName } for the Reschema modal
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

const addBucketTarget = ref(null)       // { connId, dbName } | null
const newBucketName   = ref('')
const addBucketError  = ref(null)
const addBucketSaving = ref(false)

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

const {
  indexesTarget,
  indexesList,
  indexesLoading,
  indexesError,
  newIndexKeys,
  newIndexName,
  newIndexUnique,
  indexCreating,
  pendingDropIndex,
  selectedIndex,
  indexFormMode,
  indexEditOriginalName,
  indexDetailsTarget,
  indexDetailsStats,
  indexDetailsLoading,
  dropIndexTarget,
  dropIndexConfirmText,
  dropIndexError,
  dropIndexBusy,
  loadIndexes,
  resetIndexForm,
  closeIndexesModal,
  confirmCreateIndex,
  dropIndex,
  startEditIndex,
  openIndexDetails,
  formatIndexSince,
  copyIndex,
  openDropIndexConfirm,
  confirmDropIndex,
  setIndexHidden,
  openIndexes,
} = useIndexes({ showToast: showToast })

const {
  sshHostKeyPrompt,
  sshHostKeyChanged,
  onHostKeyTrust,
  onHostKeyCancel,
  onHostKeyForget,
} = useSshHostKey()

const { runQuery, runAggregate, cancelQuery, runRestoredTab } = useQueryRunner({ tabs: tabs, showToast: showToast })

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
  if (name === 'tasks') {
    showTasksModal.value = true
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
      const nodeData = {
        connId: tab.connectionId,
        connName: tab.connectionName,
        dbName: tab.dbName,
        collName: tab.collectionName,
      }
      if (name === 'export') openExportWizard(nodeData)
      else openImportWizard(nodeData)
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
  if (name === 'reschema') {
    if (!tab || tab.kind !== 'collection') {
      showToast('Open a collection first')
      return
    }
    reschemaTarget.value = {
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

// After a Reschema apply: a new collection changes the tree, so refresh that
// connection's node. An in-place rewrite leaves the tree structure untouched.
async function onReschemaApplied(result) {
  if (result?.newCollection && result.connId) {
    await connectionTreeRef.value.refreshConn(result.connId)
  }
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

// Help-menu link targets. Default to the project's real GitHub repo (from the git
// remote); retarget as needed once dedicated pages exist.
const HELP_REPO = 'https://github.com/AqilbekAbilaev/studio-4t'
const HELP_URLS = {
  'help:license':         HELP_REPO,
  'help:gallery':         `${HELP_REPO}#readme`,
  'help:whats_new':       `${HELP_REPO}/releases`,
  'help:updates':         `${HELP_REPO}/releases`,
  'help:support':         `${HELP_REPO}/issues`,
  'help:feature_request': `${HELP_REPO}/issues/new`,
  'help:feedback':        `${HELP_REPO}/issues/new`,
  'help:tutorials':       `${HELP_REPO}/wiki`,
  'help:knowledge_base':  `${HELP_REPO}/wiki`,
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
    // Help links open the project's GitHub (issues / releases / repo). Configurable —
    // retarget any URL in HELP_URLS.
    case 'help:license':
    case 'help:gallery':
    case 'help:whats_new':
    case 'help:updates':
    case 'help:support':
    case 'help:feature_request':
    case 'help:feedback':
    case 'help:tutorials':
    case 'help:knowledge_base':
      openUrl(HELP_URLS[id]).catch(() => showToast('Could not open link'))
      return
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
    case 'file:tasks':        showTasksModal.value = true; return
    // File → Load / Save: the saved-query browser and save-query form live in the
    // active collection tab's QueryBar; signal it (no-op with a toast otherwise).
    case 'file:load':
    case 'file:save': {
      const tab = tabs.value.find(t => t.id === activeTabId.value)
      if (!tab || tab.kind !== 'collection') { showToast('Open a collection tab first'); return }
      if (id === 'file:load') browserRequest.value = { nonce: Date.now() }
      else saveQueryRequest.value = { nonce: Date.now() }
      return
    }
    case 'file:search':       handleTool('search', menuTarget('database')); return
    case 'coll:open_tab':     handleTool('collection'); return
    case 'coll:export':       handleTool('export', menuTarget('collection')); return
    case 'coll:import':       handleTool('import', menuTarget('collection')); return
    case 'coll:mask':         handleTool('mask', menuTarget('collection')); return
    case 'coll:compare':      handleTool('compare', menuTarget('database')); return

    // --- server / connection scoped ---
    case 'file:server_status': menuNode('Server Status', 'connection'); return
    case 'file:server_charts': menuNode('Server Status Charts', 'connection'); return
    case 'file:server_build':  menuNode('Build Info', 'connection'); return
    case 'db:database_stats':  menuNode('Database Statistics', 'database'); return
    case 'db:current_ops':     menuNode('Current Operations', 'connection'); return
    case 'db:profiler':        menuNode('Query Profiler', 'database'); return

    // --- database scoped ---
    case 'db:add_collection':  menuNode('Add Collection…', 'database'); return
    case 'file:add_database':
    case 'db:add_database':    menuNode('Add Database…', 'connection'); return
    case 'db:add_view':        menuNode('Add View…', 'database'); return
    case 'coll:add_view':      menuNode('Add View Here…', 'collection'); return
    case 'coll:validator':     menuNode('Add / Edit Validator…', 'collection'); return
    case 'db:export':          menuNode('Export Collections…', 'database'); return
    case 'db:import':          menuNode('Import Collections…', 'database'); return
    case 'db:add_bucket':      menuNode('Add GridFS Bucket…', 'database'); return
    case 'db:manage_users':    menuNode('Manage Users', 'database'); return
    case 'db:manage_roles':    menuNode('Manage Roles', 'database'); return
    case 'db:functions':       menuNode('Stored Functions', 'database'); return
    case 'coll:mapreduce':     menuNode('Open Map-Reduce', 'collection'); return
    // Copy/Paste: copy a collection or database to the app clipboard, then paste it
    // into a target database (same connection). Copy All == Copy Database here.
    case 'coll:copy':          menuNode('Copy Collection', 'collection'); return
    case 'db:copy_database':   menuNode('Copy Database', 'database'); return
    case 'db:copy_all':        menuNode('Copy Database', 'database'); return
    case 'db:paste':
    case 'db:paste_database':  menuNode('Paste Into Database', 'database'); return
    case 'db:drop_database':   menuNode('Drop Database…', 'database'); return
    case 'gridfs:open':        menuNode('GridFS…', 'database'); return
    case 'gridfs:add':
    case 'gridfs:save':
    case 'gridfs:remove':
    case 'gridfs:view_file':
    case 'gridfs:rename':
    case 'gridfs:meta':
    case 'gridfs:copy_bucket':
    case 'gridfs:drop_bucket':
      requestGridfsAction(id); return

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

    // Split the workspace into two panes (or toggle/flip an existing split).
    case 'view:split_v': splitWorkspace('vertical'); return
    case 'view:split_h': splitWorkspace('horizontal'); return

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
    const nd = saved.nodeData
    // Colouring a parent resets its descendants: drop their own tags (this prefix)
    // so they inherit the parent's new colour. Empty for a collection (no children).
    let clearPrefix = null
    if (saved.type === 'connection') {
      // Connection tags live on the connection config (conn.tag). The override
      // gives instant feedback; the command persists it for the next restart.
      tagOverrides.value = { ...tagOverrides.value, [nd.connId]: color }
      try { await invoke('set_connection_tag', { id: nd.connId, color: color }) } catch (_) {}
      clearPrefix = nd.connId + '/'
    } else {
      // Database/collection tags go in the dedicated node-tag store, keyed by the
      // node's tree path so a colour tags only that node, not the whole connection.
      const key = saved.type === 'database'
        ? nd.connId + '/' + nd.dbName
        : nd.connId + '/' + nd.dbName + '/' + nd.collName
      tagOverrides.value = { ...tagOverrides.value, [key]: color }
      try { await invoke('set_node_tag', { key: key, color: color }) } catch (_) {}
      if (saved.type === 'database') clearPrefix = nd.connId + '/' + nd.dbName + '/'
    }
    if (clearPrefix) {
      // Locally drop every descendant override so the tree/tabs re-inherit at once.
      const pruned = {}
      for (const k of Object.keys(tagOverrides.value)) {
        if (!k.startsWith(clearPrefix)) pruned[k] = tagOverrides.value[k]
      }
      tagOverrides.value = pruned
      try { await invoke('clear_node_tags_under', { prefix: clearPrefix }) } catch (_) {}
    }
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

  if (action === 'Add GridFS Bucket…' && saved.type === 'database') {
    addBucketTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName }
    newBucketName.value = ''
    addBucketError.value = null
    return
  }

  if (action === 'Manage Users' && saved.type === 'database') {
    usersTarget.value = { connId: saved.nodeData.connId, connName: saved.nodeData.connName, dbName: saved.nodeData.dbName }
    return
  }

  if (action === 'Manage Roles' && saved.type === 'database') {
    rolesTarget.value = { connId: saved.nodeData.connId, connName: saved.nodeData.connName, dbName: saved.nodeData.dbName }
    return
  }

  if (action === 'Stored Functions' && saved.type === 'database') {
    functionsTarget.value = { connId: saved.nodeData.connId, connName: saved.nodeData.connName, dbName: saved.nodeData.dbName }
    return
  }

  if (action === 'Open Map-Reduce' && saved.type === 'collection') {
    mapReduceTarget.value = {
      connId: saved.nodeData.connId, connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName, collName: saved.nodeData.collName,
    }
    return
  }

  if (action === 'Copy Collection' && saved.type === 'collection') {
    dbClipboard.value = {
      kind: 'collection', connId: saved.nodeData.connId, connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName, collName: saved.nodeData.collName,
    }
    showToast(`Copied collection "${saved.nodeData.collName}"`)
    return
  }

  if (action === 'Copy Database' && saved.type === 'database') {
    dbClipboard.value = {
      kind: 'database', connId: saved.nodeData.connId, connName: saved.nodeData.connName,
      dbName: saved.nodeData.dbName,
    }
    showToast(`Copied database "${saved.nodeData.dbName}"`)
    return
  }

  if (action === 'Paste Into Database' && saved.type === 'database') {
    await pasteClipboard(saved.nodeData)
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
    await openIndexes(saved.nodeData)
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
    openExportWizard(saved.nodeData)
    return
  }

  if (action === 'Import…' && saved.type === 'collection') {
    openImportWizard(saved.nodeData)
    return
  }

  if (action === 'Server Status' && saved.type === 'connection') {
    serverStatusTarget.value = {
      connId: saved.nodeData.connId,
      connName: saved.nodeData.connName,
    }
    return
  }

  if (action === 'Server Status Charts' && saved.type === 'connection') {
    serverChartsTarget.value = { connId: saved.nodeData.connId, connName: saved.nodeData.connName }
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

  if (action === 'Query Profiler' && saved.type === 'database') {
    profilerTarget.value = {
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

  if (action === 'Export Collections…' && saved.type === 'database') {
    await exportDatabase(saved.nodeData)
    return
  }

  if (action === 'Import Collections…' && saved.type === 'database') {
    await importDatabase(saved.nodeData)
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

// Paste the app clipboard (a copied collection or database) into a target database.
// Same-connection only (uses a server-side $out); cross-connection is rejected.
async function pasteClipboard(target) {
  const clip = dbClipboard.value
  if (!clip) { showToast('Nothing to paste — copy a collection or database first'); return }
  if (clip.connId !== target.connId) {
    showToast('Paste is only supported within the same connection')
    return
  }
  try {
    if (clip.kind === 'collection') {
      await invoke('copy_collection', {
        id: clip.connId,
        sourceDatabase: clip.dbName, sourceCollection: clip.collName,
        targetDatabase: target.dbName, targetCollection: clip.collName,
      })
      showToast(`Pasted "${clip.collName}" into ${target.dbName}`)
    } else {
      const dbs = await invoke('list_databases', { id: clip.connId })
      const collections = (dbs.find(d => d.name === clip.dbName)?.collections) || []
      let done = 0
      for (const coll of collections) {
        try {
          await invoke('copy_collection', {
            id: clip.connId,
            sourceDatabase: clip.dbName, sourceCollection: coll,
            targetDatabase: target.dbName, targetCollection: coll,
          })
          done++
        } catch (_) { /* skip a collection that fails; report the rest */ }
      }
      showToast(`Pasted ${done} collection${done !== 1 ? 's' : ''} into ${target.dbName}`)
    }
    await connectionTreeRef.value.refreshConn(target.connId)
  } catch (e) {
    showToast('Paste failed: ' + errMessage(e))
  }
}

// Database → Add GridFS Bucket…: a bucket is the pair of `<name>.files` and
// `<name>.chunks` collections; create both so it appears in the GridFS view.
async function confirmAddBucket() {
  const target = addBucketTarget.value
  const name = newBucketName.value.trim()
  if (!target || !name) return
  addBucketSaving.value = true
  addBucketError.value = null
  try {
    for (const suffix of ['files', 'chunks']) {
      await invoke('create_collection', {
        id: target.connId,
        database: target.dbName,
        name: `${name}.${suffix}`,
      })
    }
    await connectionTreeRef.value.refreshConn(target.connId)
    showToast(`GridFS bucket "${name}" created`)
    addBucketTarget.value = null
  } catch (e) {
    addBucketError.value = errMessage(e)
  } finally {
    addBucketSaving.value = false
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

// Open the stepped Import / Export wizard for a single collection. `nodeData` is
// the sidebar/tab shape ({ connId, connName, dbName, collName }); the wizard maps
// columns→fields with per-field type coercion and shows a live preview before it
// runs. The bulk database-level export/import below stay on the plain commands.
function openExportWizard(nodeData) {
  exportWizardTarget.value = {
    connId: nodeData.connId,
    connName: nodeData.connName,
    dbName: nodeData.dbName,
    collName: nodeData.collName,
  }
}

function openImportWizard(nodeData) {
  importWizardTarget.value = {
    connId: nodeData.connId,
    connName: nodeData.connName,
    dbName: nodeData.dbName,
    collName: nodeData.collName,
  }
}

// After a wizard import, refresh the connection so a newly-populated collection
// shows up in the sidebar.
async function onWizardImported(connId) {
  await connectionTreeRef.value.refreshConn(connId)
}

// Database → Export Collections…: export every collection in the database to a
// chosen folder, one JSON file per collection. Reuses the per-collection command.
async function exportDatabase(nodeData) {
  let dir
  try {
    dir = await openDialog({ directory: true, title: `Export all collections in ${nodeData.dbName}` })
  } catch (e) {
    showToast('Export failed: ' + errMessage(e))
    return
  }
  if (!dir) return  // user cancelled
  let collections = []
  try {
    const dbs = await invoke('list_databases', { id: nodeData.connId })
    collections = (dbs.find(d => d.name === nodeData.dbName)?.collections) || []
  } catch (e) {
    showToast('Export failed: ' + errMessage(e))
    return
  }
  if (!collections.length) { showToast('No collections to export'); return }
  let done = 0
  let failed = 0
  for (const coll of collections) {
    try {
      await invoke('export_collection', {
        id: nodeData.connId,
        database: nodeData.dbName,
        collection: coll,
        path: `${dir}/${coll}.json`,
        format: 'json',
      })
      done++
    } catch (_) {
      failed++
    }
  }
  showToast(`Exported ${done} collection${done !== 1 ? 's' : ''}${failed ? `, ${failed} failed` : ''}`)
}

// Database → Import Collections…: import one or more JSON/CSV files into the
// database, each into a collection named after the file. Reuses the per-file command.
async function importDatabase(nodeData) {
  let paths
  try {
    paths = await openDialog({
      multiple: true,
      filters: [{ name: 'JSON or CSV', extensions: ['json', 'csv'] }],
    })
  } catch (e) {
    showToast('Import failed: ' + errMessage(e))
    return
  }
  if (!paths || !paths.length) return  // user cancelled
  let done = 0
  let failed = 0
  for (const path of paths) {
    const p = String(path)
    const base = p.split(/[\\/]/).pop() || p
    const collection = base.replace(/\.(json|csv)$/i, '')
    const format = p.toLowerCase().endsWith('.csv') ? 'csv' : 'json'
    try {
      await invoke('import_collection', {
        id: nodeData.connId,
        database: nodeData.dbName,
        collection: collection,
        path: p,
        format: format,
      })
      done++
    } catch (_) {
      failed++
    }
  }
  await connectionTreeRef.value.refreshConn(nodeData.connId)
  showToast(`Imported ${done} file${done !== 1 ? 's' : ''}${failed ? `, ${failed} failed` : ''}`)
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
    paneId: focusedPaneId.value,
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
    paneId: focusedPaneId.value,
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

// ── workspace split (two panes over the shared tab list) ────
// Split Vertically / Horizontally from the View menu. Choosing the orientation the
// workspace is already split in collapses it back to one pane; choosing the other
// orientation flips it. A fresh split mirrors the focused pane's active tab so both
// panes start on the same collection.
function splitWorkspace(orientation) {
  if (isSplit.value) {
    if (splitOrientation.value === orientation) {
      unsplitWorkspace()
    } else {
      splitOrientation.value = orientation
    }
    return
  }
  // Splitting moves the active tab into the new pane, so the source pane needs at
  // least two tabs — otherwise the split would immediately leave one pane empty
  // (and empty panes auto-collapse). Guide the user instead of no-op-flickering.
  if (tabs.value.length < 2) {
    showToast('Open another tab to split the workspace')
    return
  }
  // Move the active tab out into the new pane, so the new pane starts with just that
  // one tab and the original keeps the rest (falling back to its last remaining tab).
  const current = panes.value[0]
  const movingId = current.activeTabId
  const moving = tabs.value.find(t => t.id === movingId)
  if (moving) moving.paneId = 'p1'
  const remaining = tabs.value.filter(t => (t.paneId || 'p0') === 'p0')
  current.activeTabId = remaining.length ? remaining[remaining.length - 1].id : null
  panes.value.push({ id: 'p1', activeTabId: movingId })
  splitOrientation.value = orientation
  focusedPaneId.value = 'p1'
}

// Collapse the split back to a single pane, keeping `survivor`'s tabs and active tab.
// Every tab returns to pane 0.
function collapseToPane(survivor) {
  const keep = survivor || panes.value[0]
  for (const t of tabs.value) t.paneId = 'p0'
  panes.value = [{ id: 'p0', activeTabId: keep ? keep.activeTabId : null }]
  splitOrientation.value = null
  focusedPaneId.value = 'p0'
}

// Manual unsplit (View menu / gutter ✕): keep whatever the focused pane was showing.
function unsplitWorkspace() {
  if (!isSplit.value) return
  collapseToPane(panes.value.find(p => p.id === focusedPaneId.value) || panes.value[0])
}

// Per-pane event routing: interacting with a pane focuses it first, so the shared
// menu actions (which read the focused pane through activeTabId) target that pane.
function focusPane(paneId) { focusedPaneId.value = paneId }
function activateTabInPane(paneId, id) {
  focusedPaneId.value = paneId
  const pane = panes.value.find(p => p.id === paneId)
  if (pane) pane.activeTabId = id
  const tab = tabs.value.find(t => t.id === id)
  if (tab && tab._restored) runRestoredTab(tab)
}
function closeTabInPane(paneId, id) {
  focusedPaneId.value = paneId
  closeTab(id)
}
function tabContextInPane(paneId, evt) {
  focusedPaneId.value = paneId
  onTabContext(evt)
}

// GridFS menu actions operate inside the GridFS modal on its selected file/bucket.
// Ensure the modal is open for the resolved database (preserving any existing
// selection when it's already showing that db), then signal the requested action.
async function requestGridfsAction(action) {
  const target = menuTarget('database')
  if (!target || !target.connectionId || !target.dbName) {
    showToast('Open a database first')
    return
  }
  const sameOpen = gridfsTarget.value
    && gridfsTarget.value.connId === target.connectionId
    && gridfsTarget.value.dbName === target.dbName
  if (!sameOpen) {
    gridfsTarget.value = {
      connId: target.connectionId,
      connName: target.connectionName,
      dbName: target.dbName,
    }
  }
  await nextTick()
  gridfsRequest.value = { action: action, nonce: Date.now() }
}

// Help → Quickstart: focus the existing Quickstart tab, or open one if it was closed.
function openQuickstart() {
  const existing = tabs.value.find(t => t.kind === 'quickstart')
  if (existing) {
    activateTab(existing.id)
    return
  }
  const id = 't' + Date.now()
  tabs.value.push({ id: id, kind: 'quickstart', title: 'Quickstart', paneId: focusedPaneId.value })
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
  const before = tabs.value.slice(0, idx)
  tabs.value.splice(idx, 1)
  // Any pane showing the closed tab moves to an adjacent tab it still owns (the
  // nearest preceding one in the same pane, else that pane's first remaining tab).
  for (const pane of panes.value) {
    if (pane.activeTabId !== id) continue
    const precedingInPane = before.filter(t => (t.paneId || 'p0') === pane.id)
    const owned = tabs.value.filter(t => (t.paneId || 'p0') === pane.id)
    const next = precedingInPane.length ? precedingInPane[precedingInPane.length - 1] : owned[0]
    pane.activeTabId = next ? next.id : null
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
      paneId: src.paneId || 'p0',
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
    paneId: src.paneId || 'p0',
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

      <!-- Workspace (single pane) -->
      <QueryWorkspace
        v-if="!isSplit"
        :tabs="paneATabs"
        :active-tab-id="panes[0].activeTabId"
        :tag-overrides="tagOverrides"
        :vqb-open="vqbOpen"
        :clipboard-query="clipboardQuery"
        :doc-menu-request="docMenuRequest"
        :history-request="historyRequest"
        :browser-request="browserRequest"
        :save-query-request="saveQueryRequest"
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

      <!-- Workspace (split into two panes over the shared tab list) -->
      <SplitContainer v-else :orientation="splitOrientation" @unsplit="unsplitWorkspace">
        <template #a>
          <div class="pane-host" :class="{ focused: focusedPaneId === 'p0' }" @mousedown.capture="focusPane('p0')">
            <QueryWorkspace
              :tabs="paneATabs"
              :active-tab-id="panes[0].activeTabId"
              :tag-overrides="tagOverrides"
              :vqb-open="focusedPaneId === 'p0' && vqbOpen"
              :clipboard-query="clipboardQuery"
              :doc-menu-request="focusedPaneId === 'p0' ? docMenuRequest : null"
              :history-request="focusedPaneId === 'p0' ? historyRequest : null"
              :browser-request="focusedPaneId === 'p0' ? browserRequest : null"
              :save-query-request="focusedPaneId === 'p0' ? saveQueryRequest : null"
              @activate-tab="activateTabInPane('p0', $event)"
              @close-tab="closeTabInPane('p0', $event)"
              @tab-context="tabContextInPane('p0', $event)"
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
        </template>
        <template #b>
          <div class="pane-host" :class="{ focused: focusedPaneId === 'p1' }" @mousedown.capture="focusPane('p1')">
            <QueryWorkspace
              :tabs="paneBTabs"
              :active-tab-id="panes[1].activeTabId"
              :tag-overrides="tagOverrides"
              :vqb-open="focusedPaneId === 'p1' && vqbOpen"
              :clipboard-query="clipboardQuery"
              :doc-menu-request="focusedPaneId === 'p1' ? docMenuRequest : null"
              :history-request="focusedPaneId === 'p1' ? historyRequest : null"
              :browser-request="focusedPaneId === 'p1' ? browserRequest : null"
              :save-query-request="focusedPaneId === 'p1' ? saveQueryRequest : null"
              @activate-tab="activateTabInPane('p1', $event)"
              @close-tab="closeTabInPane('p1', $event)"
              @tab-context="tabContextInPane('p1', $event)"
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
        </template>
      </SplitContainer>
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

    <ProfilerModal
      v-if="profilerTarget"
      :target="profilerTarget"
      @close="profilerTarget = null"
    />

    <ValidatorModal
      v-if="validatorTarget"
      :target="validatorTarget"
      @saved="onValidatorSaved"
      @close="validatorTarget = null"
    />

    <UsersModal
      v-if="usersTarget"
      :target="usersTarget"
      @close="usersTarget = null"
    />

    <RolesModal
      v-if="rolesTarget"
      :target="rolesTarget"
      @close="rolesTarget = null"
    />

    <FunctionsModal
      v-if="functionsTarget"
      :target="functionsTarget"
      @close="functionsTarget = null"
    />

    <MapReduceModal
      v-if="mapReduceTarget"
      :target="mapReduceTarget"
      @close="mapReduceTarget = null"
    />

    <ServerChartsModal
      v-if="serverChartsTarget"
      :target="serverChartsTarget"
      @close="serverChartsTarget = null"
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

    <!-- Tasks panel -->
    <TasksModal
      v-if="showTasksModal"
      @toast="showToast"
      @close="showTasksModal = false"
    />

    <!-- Data Masking modal -->
    <MaskingModal
      v-if="maskingTarget"
      :target="maskingTarget"
      @toast="showToast"
      @close="maskingTarget = null"
    />

    <!-- Import / Export field-mapping wizard -->
    <ImportExportWizard
      v-if="importWizardTarget"
      mode="import"
      :target="importWizardTarget"
      @toast="showToast"
      @done="onWizardImported"
      @close="importWizardTarget = null"
    />
    <ImportExportWizard
      v-if="exportWizardTarget"
      mode="export"
      :target="exportWizardTarget"
      @toast="showToast"
      @close="exportWizardTarget = null"
    />

    <!-- Reschema modal -->
    <ReschemaModal
      v-if="reschemaTarget"
      :target="reschemaTarget"
      @toast="showToast"
      @applied="onReschemaApplied"
      @close="reschemaTarget = null"
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
      :menu-request="gridfsRequest"
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

    <!-- Add GridFS Bucket modal -->
    <div v-if="addBucketTarget" class="del-overlay" @mousedown.self="addBucketTarget = null">
      <div class="del-dialog">
        <div class="del-title">
          <div class="t">Add GridFS Bucket</div>
          <button class="close-btn" @click="addBucketTarget = null">
            <BaseIcon name="close" :size="14" />
          </button>
        </div>
        <div class="del-body">
          <input
            v-model="newBucketName"
            class="prompt-input"
            placeholder="Bucket name (e.g. fs)"
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            @keydown.enter="confirmAddBucket"
          />
          <div v-if="addBucketError" class="del-error">{{ addBucketError }}</div>
        </div>
        <div class="del-footer">
          <span class="spacer"></span>
          <button class="btn" @click="addBucketTarget = null">Cancel</button>
          <button class="btn primary" :disabled="!newBucketName.trim() || addBucketSaving" @click="confirmAddBucket">
            {{ addBucketSaving ? 'Creating…' : 'Create' }}
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
            <tbody>
              <tr><td>Operations</td><td>{{ indexDetailsStats.accesses?.ops ?? '—' }}</td></tr>
              <tr><td>Tracking since</td><td>{{ formatIndexSince(indexDetailsStats.accesses?.since) }}</td></tr>
            </tbody>
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

<style src="./App.css" scoped></style>
