<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { listen } from '@tauri-apps/api/event'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  activeCollectionKey: String,
  expandId: String,
  width: { type: Number, default: 320 },
  tagOverrides: { type: Object, default: () => ({}) },
  contextActiveNodeKey: { type: String, default: null },
})
const emit = defineEmits(['select-collection', 'expanded', 'context-menu'])

const connections = ref([])
const expandedConns = ref({})      // connId → boolean
const loadingConns = ref({})       // connId → boolean
const connDatabases = ref({})      // connId → DatabaseInfo[]
const connErrors = ref({})         // connId → string
const expandedDbs = ref({})        // "connId/dbName" → boolean
const selectedKey = ref(null)      // collection row highlighted by a single click
const searchText = ref('')
const sidebarEl = ref(null)        // root element, used to detect outside clicks

// A single click anywhere outside the sidebar (e.g. in the QueryWorkspace) clears
// the single-click collection highlight. Clicks inside the sidebar are handled by
// the per-row handlers, so they're ignored here.
function clearSelectionOnOutsideClick(e) {
  if (sidebarEl.value && !sidebarEl.value.contains(e.target)) {
    selectedKey.value = null
  }
}

onMounted(async () => {
  // The sidebar shows only the connections that are open; the full saved list
  // lives in the Connection Manager. A connection's `open` flag is persisted, so
  // only the ones that were open before a restart come back.
  const all = await invoke('list_connections')
  connections.value = all.filter(c => c.open)
  await listen('connection-saved', (e) => {
    if (!connections.value.some(c => c.id === e.payload.id)) {
      connections.value.push(e.payload)
    }
  })
  await listen('connection-deleted', (e) => {
    disconnectConn(e.payload.id, { persist: false })
  })
  document.addEventListener('click', clearSelectionOnOutsideClick)
})

onUnmounted(() => {
  document.removeEventListener('click', clearSelectionOnOutsideClick)
})

async function toggleConnection(conn) {
  // Clicking any other row clears a lingering single-click collection highlight.
  selectedKey.value = null
  const id = conn.id
  const wasOpen = expandedConns.value[id]
  expandedConns.value[id] = !wasOpen

  if (!wasOpen && !connDatabases.value[id]) {
    loadingConns.value[id] = true
    connErrors.value[id] = null
    try {
      connDatabases.value[id] = await invoke('list_databases', { id: id })
    } catch (e) {
      connErrors.value[id] = errMessage(e)
      expandedConns.value[id] = false
    } finally {
      loadingConns.value[id] = false
    }
  }
}

function toggleDatabase(connId, dbName) {
  // Clicking any other row clears a lingering single-click collection highlight.
  selectedKey.value = null
  const key = `${connId}/${dbName}`
  expandedDbs.value[key] = !expandedDbs.value[key]
}

// Single click only selects (highlights) the row; double click opens it. This
// mirrors Studio-3T and lets the same collection be opened in several tabs.
function highlightCollection(conn, db, collName) {
  selectedKey.value = collectionKey(conn.id, db.name, collName)
}

function openCollection(conn, db, collName) {
  // Opening makes the row the active collection, so its highlight comes from
  // `activeCollectionKey`. Clear the single-click selection set by the click
  // that preceded this double-click, otherwise it lingers as a stale highlight
  // after the active tab moves to another collection.
  selectedKey.value = null
  emit('select-collection', {
    connectionId: conn.id,
    connectionName: conn.name,
    dbName: db.name,
    collectionName: collName,
  })
}

function collectionKey(connId, dbName, collName) {
  return `${connId}/${dbName}/${collName}`
}

watch(() => props.expandId, async (id) => {
  if (!id) return
  let conn = connections.value.find(c => c.id === id)
  if (!conn) {
    // Opening a connection that isn't in the sidebar yet: fetch just its config,
    // mark it open (persisted), and add only it — don't reload the whole list.
    const all = await invoke('list_connections')
    conn = all.find(c => c.id === id)
    if (conn) {
      await invoke('set_connection_open', { id: id, open: true })
      connections.value.push(conn)
    }
  }
  if (conn && !expandedConns.value[id]) {
    toggleConnection(conn)
  }
  emit('expanded')
})

// When a collection becomes the active one (e.g. switching tabs in the
// workspace), expand the sidebar down to it so the highlighted row is visible.
// Only the connId and dbName are needed; a collection name may contain slashes,
// so split on the first two separators only.
watch(() => props.activeCollectionKey, async (key) => {
  if (!key) return
  const slash1 = key.indexOf('/')
  const slash2 = key.indexOf('/', slash1 + 1)
  if (slash1 === -1 || slash2 === -1) return
  const connId = key.slice(0, slash1)
  const dbName = key.slice(slash1 + 1, slash2)

  const conn = connections.value.find(c => c.id === connId)
  if (!conn) return  // not a connection the sidebar currently shows

  if (!expandedConns.value[connId]) {
    await toggleConnection(conn)
  }
  expandedDbs.value[`${connId}/${dbName}`] = true
})

import { computed } from 'vue'
const filtered = computed(() => {
  if (!searchText.value) return connections.value
  const q = searchText.value.toLowerCase()
  return connections.value.filter(c => c.name.toLowerCase().includes(q))
})

function effectiveTag(conn) {
  return props.tagOverrides[conn.id] !== undefined ? props.tagOverrides[conn.id] : conn.tag
}

// A connection's live state, derived from what the tree already tracks:
//   error     → the last list_databases failed
//   loading   → databases are being fetched
//   connected → databases loaded successfully (we've talked to the server)
//   idle      → in the sidebar but not opened yet
function connStatus(conn) {
  const id = conn.id
  if (connErrors.value[id]) return 'error'
  if (loadingConns.value[id]) return 'loading'
  if (connDatabases.value[id]) return 'connected'
  return 'idle'
}

const STATUS_LABEL = {
  error:     'Connection error',
  loading:   'Connecting…',
  connected: 'Connected',
  idle:      'Not connected',
}

function onNodeContext(e, type, label, nodeData) {
  emit('context-menu', { type: type, x: e.clientX, y: e.clientY, label: label, nodeData: nodeData })
}

function disconnectConn(connId, { persist = true } = {}) {
  connections.value = connections.value.filter(c => c.id !== connId)
  delete expandedConns.value[connId]
  delete loadingConns.value[connId]
  delete connDatabases.value[connId]
  delete connErrors.value[connId]
  for (const key of Object.keys(expandedDbs.value)) {
    if (key.startsWith(connId + '/')) {
      delete expandedDbs.value[key]
    }
  }
  // Persist the closed state so it doesn't re-open after restart. Skipped when the
  // connection was deleted (the record is already gone from storage).
  if (persist) {
    invoke('set_connection_open', { id: connId, open: false })
  }
}

async function refreshConn(connId) {
  if (!expandedConns.value[connId]) return
  delete connDatabases.value[connId]
  loadingConns.value[connId] = true
  connErrors.value[connId] = null
  try {
    connDatabases.value[connId] = await invoke('list_databases', { id: connId })
  } catch (e) {
    connErrors.value[connId] = errMessage(e)
    expandedConns.value[connId] = false
  } finally {
    loadingConns.value[connId] = false
  }
}

function getConnections() {
  return connections.value
}

defineExpose({ disconnectConn, refreshConn, getConnections })
</script>

<template>
  <div class="sidebar" ref="sidebarEl" :style="{ width: props.width + 'px' }">
    <!-- Search row -->
    <div class="side-search">
      <div class="search-box">
        <BaseIcon name="search" :size="14" style="color:var(--text-faint);flex:none" />
        <input v-model="searchText" placeholder="Search open connections (⌘F)" />
      </div>
      <button class="icon-btn sm" title="Font size">
        <BaseIcon name="textType" :size="15" />
      </button>
    </div>

    <!-- Tree -->
    <!-- Clicking empty space in the tree clears a single-click collection highlight. -->
    <div class="tree" @click.self="selectedKey = null">
      <div v-if="filtered.length === 0" class="tree-empty">
        No connections. Use File → Connect.
      </div>

      <template v-for="conn in filtered" :key="conn.id">
        <!-- Connection root -->
        <div
          class="tnode"
          :class="{
            sel: activeCollectionKey?.startsWith(conn.id),
            'ctx-sel': props.contextActiveNodeKey === conn.id,
            prod: effectiveTag(conn) === 'red',
            bold: effectiveTag(conn) === 'red',
          }"
          style="padding-left: 6px"
          @click="toggleConnection(conn)"
          @contextmenu.prevent="onNodeContext($event, 'connection', conn.name, { connId: conn.id, connName: conn.name })"
        >
          <span class="tw">
            <BaseIcon :name="expandedConns[conn.id] ? 'caretDown' : 'caret'" :size="12" />
          </span>
          <span class="ti"><BaseIcon name="connect" :size="15" /></span>
          <span class="tt">{{ conn.name }}</span>
          <span
            class="status-dot"
            :class="connStatus(conn)"
            :title="STATUS_LABEL[connStatus(conn)]"
          ></span>
        </div>

        <!-- Loading indicator -->
        <div v-if="loadingConns[conn.id]" class="tnode" style="padding-left: 36px">
          <span class="mini-spin"></span>
          <span class="tt" style="color:var(--text-faint);font-size:11.5px">Loading…</span>
        </div>

        <!-- Error -->
        <div v-if="connErrors[conn.id]" class="tnode err-node" style="padding-left: 36px">
          <span class="err-msg">{{ connErrors[conn.id] }}</span>
          <span class="err-retry" @click.stop="toggleConnection(conn)">Retry</span>
        </div>

        <!-- Databases -->
        <template v-if="expandedConns[conn.id] && connDatabases[conn.id]">
          <template v-for="db in connDatabases[conn.id]" :key="db.name">
            <!-- Database row -->
            <div
              class="tnode"
              :class="{
                prod: effectiveTag(conn) === 'red',
                locked: !db.accessible,
                'ctx-sel': props.contextActiveNodeKey === conn.id + '/' + db.name,
              }"
              style="padding-left: 21px"
              @click="db.accessible ? toggleDatabase(conn.id, db.name) : (selectedKey = null)"
              @contextmenu.prevent="onNodeContext($event, 'database', db.name, { connId: conn.id, dbName: db.name })"
            >
              <span class="tw">
                <BaseIcon v-if="!db.accessible" name="lock" :size="12" />
                <BaseIcon v-else :name="expandedDbs[`${conn.id}/${db.name}`] ? 'caretDown' : 'caret'" :size="12" />
              </span>
              <span class="ti"><BaseIcon name="dbSmall" :size="15" /></span>
              <span class="tt">{{ db.name }}</span>
              <span v-if="db.accessible && db.collections.length" class="cnt">({{ db.collections.length }})</span>
            </div>

            <!-- Collections -->
            <template v-if="expandedDbs[`${conn.id}/${db.name}`]">
              <div
                v-for="coll in db.collections"
                :key="coll"
                class="tnode"
                :class="{
                  sel: activeCollectionKey === collectionKey(conn.id, db.name, coll)
                    || selectedKey === collectionKey(conn.id, db.name, coll),
                  'ctx-sel': props.contextActiveNodeKey === collectionKey(conn.id, db.name, coll),
                  prod: effectiveTag(conn) === 'red',
                }"
                style="padding-left: 51px"
                @click="highlightCollection(conn, db, coll)"
                @dblclick="openCollection(conn, db, coll)"
                @contextmenu.prevent="onNodeContext($event, 'collection', coll, { connId: conn.id, connName: conn.name, dbName: db.name, collName: coll })"
              >
                <span class="tw empty"><BaseIcon name="caret" :size="12" /></span>
                <span class="ti"><BaseIcon name="collSmall" :size="15" /></span>
                <span class="tt">{{ coll }}</span>
              </div>
            </template>
          </template>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  flex: none;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.side-search {
  padding: 8px;
  display: flex;
  gap: 6px;
  align-items: center;
  flex-shrink: 0;
}

.search-box {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 6px 9px;
}

.search-box input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-size: 12.5px;
}

.search-box input::placeholder { color: var(--text-faint); }

.icon-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-dim);
  padding: 5px;
  display: grid;
  place-items: center;
}
.icon-btn:hover { background: var(--bg-hover); color: var(--text); }
.icon-btn.sm { padding: 4px; }

.tree {
  flex: 1;
  overflow-y: auto;
  padding: 2px 0 12px;
}

.tree-empty {
  padding: 16px 12px;
  font-size: 12px;
  color: var(--text-faint);
  text-align: center;
}

.tnode {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px 3px 0;
  font-size: 12.5px;
  cursor: default;
  white-space: nowrap;
  user-select: none;
}

.tnode:hover  { background: var(--bg-hover); }
.tnode.sel    { background: var(--bg-active); }
.tnode.ctx-sel { background: var(--bg-hover); }

.tw {
  width: 16px;
  display: grid;
  place-items: center;
  color: var(--text-faint);
  flex: none;
}
.tw.empty { visibility: hidden; }

.ti { flex: none; color: var(--text-dim); }
.tt { overflow: hidden; text-overflow: ellipsis; }

.cnt { color: var(--text-faint); font-size: 11.5px; margin-left: 4px; }

/* Per-connection status dot, pushed to the right edge of the row. */
.status-dot {
  flex: none;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  margin-left: auto;
  margin-right: 8px;
  background: var(--text-faint);
}
.status-dot.connected { background: var(--green); }
.status-dot.error     { background: var(--prod); }
.status-dot.idle      { background: var(--text-faint); opacity: .45; }
.status-dot.loading {
  background: var(--warn);
  animation: status-pulse 1s ease-in-out infinite;
}
@keyframes status-pulse { 0%, 100% { opacity: 1; } 50% { opacity: .3; } }

.err-node { align-items: flex-start; cursor: default; flex-direction: column; gap: 2px; }
.err-msg { color: #e07070; font-size: 11.5px; white-space: pre-wrap; word-break: break-word; line-height: 1.5; padding: 2px 0; }
.err-retry { color: var(--accent); font-size: 11.5px; cursor: pointer; }
.err-retry:hover { text-decoration: underline; }
.mini-spin {
  width: 11px;
  height: 11px;
  margin-right: 7px;
  border-radius: 50%;
  border: 1.5px solid var(--border);
  border-top-color: var(--accent);
  animation: tree-spin 0.7s linear infinite;
  flex: none;
}
@keyframes tree-spin { to { transform: rotate(360deg); } }

.tnode.prod .tt,
.tnode.prod .ti { color: var(--prod); }
.tnode.prod.bold .tt { font-weight: 700; }

.tnode.locked { cursor: default; }
.tnode.locked .tt,
.tnode.locked .ti,
.tnode.locked .tw { color: var(--text-faint); }
</style>
