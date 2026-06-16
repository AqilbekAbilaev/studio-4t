<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import BaseIcon from './components/BaseIcon.vue'
import ConnectionTree from './components/ConnectionTree.vue'
import QueryWorkspace from './components/QueryWorkspace.vue'
import ConnectionManager from './components/ConnectionManager.vue'
import VisualQueryBuilder from './components/VisualQueryBuilder.vue'
import ContextMenu from './components/ContextMenu.vue'

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
const vqbOpen = ref(false)
const contextMenu = ref(null)
const tagOverrides = ref({})

const addCollectionTarget = ref(null)   // { connId, dbName, uri } | null
const newCollectionName   = ref('')
const addCollectionError  = ref(null)
const addCollectionSaving = ref(false)

const dropDatabaseTarget   = ref(null)  // { connId, dbName, uri } | null
const dropDatabaseError    = ref(null)
const dropDatabaseDeleting = ref(false)

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
      uri: saved.nodeData.uri,
      dbName: saved.nodeData.dbName,
      collectionName: saved.nodeData.collName,
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
    await connectionTreeRef.value.refreshConn(saved.nodeData.connId, saved.nodeData.uri)
    showToast('Refreshed')
    return
  }

  if (action === 'Add Collection…') {
    addCollectionTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName, uri: saved.nodeData.uri }
    newCollectionName.value = ''
    addCollectionError.value = null
    return
  }

  if (action === 'Drop Database…') {
    dropDatabaseTarget.value = { connId: saved.nodeData.connId, dbName: saved.nodeData.dbName, uri: saved.nodeData.uri }
    dropDatabaseError.value = null
    return
  }

  if (action === 'Refresh All') {
    for (const conn of connectionTreeRef.value.getConnections()) {
      await connectionTreeRef.value.refreshConn(conn.id, conn.uri)
    }
    showToast('All connections refreshed')
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
    await invoke('create_collection', { id: target.connId, uri: target.uri, database: target.dbName, name: name })
    await connectionTreeRef.value.refreshConn(target.connId, target.uri)
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
    await invoke('drop_database', { id: target.connId, uri: target.uri, database: target.dbName })
    await connectionTreeRef.value.refreshConn(target.connId, target.uri)
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

// ── tab management ─────────────────────────────────────────
function openCollectionTab({ connectionId, connectionName, uri, dbName, collectionName }) {
  const existing = tabs.value.find(t =>
    t.kind === 'collection' &&
    t.connectionId === connectionId &&
    t.dbName === dbName &&
    t.collectionName === collectionName
  )
  if (existing) { activeTabId.value = existing.id; return }

  const id = 't' + Date.now()
  tabs.value.push({
    id, kind: 'collection',
    title: collectionName,
    connectionId, connectionName, uri, dbName, collectionName,
    filter: '', projection: '', sort: '', skip: 0, limit: 50,
    results: [], hasRun: false, isRunning: false, runError: null,
    selectedRow: -1, elapsedMs: null,
  })
  activeTabId.value = id
  runQuery(id, { filter: '{}', projection: '{}', sort: '{}', skip: 0, limit: 50 })
}

function activateTab(id) { activeTabId.value = id }

function closeTab(id) {
  const idx = tabs.value.findIndex(t => t.id === id)
  if (idx < 0) return
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
  try {
    tab.results = await invoke('find_documents', {
      id:         tab.connectionId,
      uri:        tab.uri,
      database:   tab.dbName,
      collection: tab.collectionName,
      ...params,
    })
    tab.hasRun = true
    tab.elapsedMs = Date.now() - t0
    showToast(`Query returned ${tab.results.length} document${tab.results.length !== 1 ? 's' : ''} in ${(tab.elapsedMs / 1000).toFixed(3)}s`)
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
        @activate-tab="activateTab"
        @close-tab="closeTab"
        @run-query="runQuery"
        @toggle-vqb="vqbOpen = !vqbOpen"
      />
      <VisualQueryBuilder v-if="vqbOpen" />
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
  width: 5px;
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
