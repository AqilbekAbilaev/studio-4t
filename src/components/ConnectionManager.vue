<script setup>
import { ref, computed, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, emit as tauriEmit } from '@tauri-apps/api/event'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { errMessage } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import NewConnection from './NewConnection.vue'
import ContextMenu from './ContextMenu.vue'

const emit = defineEmits(['close', 'connect', 'toast'])

const connections = ref([])
const selectedId = ref(null)
const filterText = ref('')
const showOnStartup     = ref(false)
const showNewConnection = ref(false)
const showEditConnection = ref(false)

// --- Folders (Connection Manager grouping) ---
const folders          = ref([])
const expandedFolders  = ref([])      // folder ids currently expanded (default: all)
const renamingFolderId = ref(null)
const renameText       = ref('')
const pendingDeleteId  = ref(null)    // folder id armed for a confirming second click
const ctxMenu          = ref(null)    // { x, y, connId } for the move-to-folder context menu

const TAG_COLORS = {
  red: '#e07a6b', blue: '#3b82f6', green: '#4caf78', purple: '#b07ddb',
}

onMounted(async () => {
  connections.value = await invoke('list_connections')
  folders.value = await invoke('list_folders')
  expandedFolders.value = folders.value.map(f => f.id)  // start expanded
  listen('connection-saved', (e) => {
    if (!connections.value.find(c => c.id === e.payload.id))
      connections.value.push(e.payload)
  })
})

const filtered = computed(() => {
  const q = filterText.value.toLowerCase()
  if (!q) return connections.value
  return connections.value.filter(c =>
    c.name.toLowerCase().includes(q) || parseDbServer(c).toLowerCase().includes(q)
  )
})

function tagColor(tag) { return TAG_COLORS[tag] ?? null }

function parseDbServer(conn) {
  const hosts = conn.hosts ?? []
  if (!hosts.length) return '—'
  const first = hosts[0]
  if (conn.connection_type === 'srv') return first.host
  const label = `${first.host}:${first.port}`
  return hosts.length > 1 ? `${label} +${hosts.length - 1}` : label
}

function parseSecurity(conn) {
  if (!conn.username) return null
  const db = conn.auth_db || 'admin'
  return `${conn.username} @ ${db}`
}

function formatNow() {
  return new Date().toLocaleString('en-GB', {
    day: '2-digit', month: 'short', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  }).replace(',', '')
}

function newConnection() {
  showNewConnection.value = true
}

function editSelected() {
  if (!selectedId.value) return
  showEditConnection.value = true
}

function onConnectionSaved(conn) {
  if (!connections.value.find(c => c.id === conn.id)) {
    connections.value.push(conn)
  }
  showNewConnection.value = false
}

function onConnectionUpdated(conn) {
  const idx = connections.value.findIndex(c => c.id === conn.id)
  if (idx !== -1) connections.value.splice(idx, 1, conn)
  showEditConnection.value = false
}

async function deleteSelected() {
  if (!selectedId.value) return
  const deletedId = selectedId.value
  await invoke('delete_connection', { id: deletedId })
  connections.value = connections.value.filter(c => c.id !== deletedId)
  selectedId.value = null
  // Tell the sidebar to drop it too if it was open (mirrors connection-saved).
  await tauriEmit('connection-deleted', { id: deletedId })
}

async function connectSelected() {
  if (!selectedId.value) return
  const now = formatNow()
  try {
    await invoke('update_last_accessed', { id: selectedId.value, timestamp: now })
    const conn = connections.value.find(c => c.id === selectedId.value)
    if (conn) conn.last_accessed = now
  } catch {}
  emit('connect', selectedId.value)
}

async function duplicateSelected() {
  if (!selectedId.value) return
  try {
    const copy = await invoke('duplicate_connection', { id: selectedId.value })
    connections.value.push(copy)
    selectedId.value = copy.id
    emit('toast', `Duplicated as "${copy.name}"`)
  } catch (e) {
    emit('toast', 'Duplicate failed: ' + errMessage(e))
  }
}

async function copyUri() {
  if (!selectedId.value) return
  try {
    const uri = await invoke('connection_uri', { id: selectedId.value })
    await navigator.clipboard.writeText(uri)
    emit('toast', 'Connection URI copied (password excluded)')
  } catch (e) {
    emit('toast', 'To URI failed: ' + errMessage(e))
  }
}

async function exportConnections() {
  let path
  try {
    path = await saveDialog({
      defaultPath: 'connections.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
  } catch (e) {
    emit('toast', 'Export failed: ' + errMessage(e))
    return
  }
  if (!path) return  // user cancelled
  try {
    const count = await invoke('export_connections', { path: path })
    emit('toast', `Exported ${count} connection${count !== 1 ? 's' : ''} (passwords excluded)`)
  } catch (e) {
    emit('toast', 'Export failed: ' + errMessage(e))
  }
}

async function importConnections() {
  let path
  try {
    path = await openDialog({
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
  } catch (e) {
    emit('toast', 'Import failed: ' + errMessage(e))
    return
  }
  if (!path) return  // user cancelled
  try {
    const count = await invoke('import_connections', { path: String(path) })
    connections.value = await invoke('list_connections')
    emit('toast', `Imported ${count} connection${count !== 1 ? 's' : ''} — re-enter passwords to connect`)
  } catch (e) {
    emit('toast', 'Import failed: ' + errMessage(e))
  }
}

// --- Folder grouping ---
const isFiltering = computed(() => filterText.value.trim().length > 0)

const validFolderIds = computed(() => new Set(folders.value.map(f => f.id)))

// Connections grouped by their folder id (only valid, existing folders count).
const connsByFolder = computed(() => {
  const m = new Map()
  for (const c of connections.value) {
    if (c.folder_id && validFolderIds.value.has(c.folder_id)) {
      if (!m.has(c.folder_id)) m.set(c.folder_id, [])
      m.get(c.folder_id).push(c)
    }
  }
  return m
})

// Connections at the root: no folder, or a folder that no longer exists.
const rootConns = computed(() =>
  connections.value.filter(c => !c.folder_id || !validFolderIds.value.has(c.folder_id))
)

function isExpanded(id) { return expandedFolders.value.includes(id) }

function toggleFolder(id) {
  pendingDeleteId.value = null
  const i = expandedFolders.value.indexOf(id)
  if (i === -1) expandedFolders.value.push(id)
  else expandedFolders.value.splice(i, 1)
}

// An ordered flat list of rows to render: folder headers, their connections
// (when expanded), then the root connections. While filtering, a flat list of
// matches with no folder headers.
const displayRows = computed(() => {
  if (isFiltering.value) {
    return filtered.value.map(c => ({ type: 'conn', conn: c, indent: false }))
  }
  const rows = []
  for (const f of folders.value) {
    const kids = connsByFolder.value.get(f.id) ?? []
    rows.push({ type: 'folder', folder: f, count: kids.length })
    if (isExpanded(f.id)) {
      if (kids.length === 0) rows.push({ type: 'empty', key: 'empty-' + f.id })
      else for (const c of kids) rows.push({ type: 'conn', conn: c, indent: true })
    }
  }
  for (const c of rootConns.value) rows.push({ type: 'conn', conn: c, indent: false })
  return rows
})

// Create a folder with a unique default name and expand it.
async function createUniqueFolder() {
  const base = 'New Folder'
  const existing = new Set(folders.value.map(f => f.name))
  let name = base
  let n = 2
  while (existing.has(name)) name = `${base} ${n++}`
  const folder = await invoke('create_folder', { name: name })
  folders.value.push(folder)
  expandedFolders.value.push(folder.id)
  return folder
}

async function newFolder() {
  // Create, then drop straight into inline rename.
  try {
    startRenameFolder(await createUniqueFolder())
  } catch (e) {
    emit('toast', 'Create folder failed: ' + errMessage(e))
  }
}

function startRenameFolder(f) {
  renamingFolderId.value = f.id
  renameText.value = f.name
  nextTick(() => {
    const el = document.querySelector('.folder-rename-input')
    if (el) { el.focus(); el.select() }
  })
}

async function commitRenameFolder(f) {
  const name = renameText.value.trim()
  renamingFolderId.value = null
  if (!name || name === f.name) return
  try {
    await invoke('rename_folder', { id: f.id, name: name })
    const target = folders.value.find(x => x.id === f.id)
    if (target) target.name = name
  } catch (e) {
    emit('toast', 'Rename failed: ' + errMessage(e))
  }
}

function cancelRenameFolder() {
  renamingFolderId.value = null
}

async function deleteFolder(f) {
  // Two-click confirm: first click arms, second click deletes.
  if (pendingDeleteId.value !== f.id) {
    pendingDeleteId.value = f.id
    return
  }
  pendingDeleteId.value = null
  try {
    await invoke('delete_folder', { id: f.id })
    folders.value = folders.value.filter(x => x.id !== f.id)
    // Connections inside were moved back to root server-side; reload to reflect.
    connections.value = await invoke('list_connections')
  } catch (e) {
    emit('toast', 'Delete folder failed: ' + errMessage(e))
  }
}

function openMoveMenu(event, conn) {
  selectedId.value = conn.id
  pendingDeleteId.value = null
  ctxMenu.value = { x: event.clientX, y: event.clientY, connId: conn.id }
}

// The items for the reused ContextMenu: remove-from-folder (when applicable),
// the folder list (current folder checked), then a New Folder action.
const moveMenuModel = computed(() => {
  if (!ctxMenu.value) return null
  const conn = connections.value.find(c => c.id === ctxMenu.value.connId)
  const currentId = conn && validFolderIds.value.has(conn.folder_id) ? conn.folder_id : null
  const items = []
  if (currentId) {
    items.push({ label: 'Remove from Folder', icon: 'close', value: 'root' })
    items.push({ sep: true })
  }
  for (const f of folders.value) {
    items.push({
      label: f.name,
      icon: 'folder',
      value: 'f:' + f.id,
      shortcut: f.id === currentId ? '✓' : undefined,
    })
  }
  if (folders.value.length) items.push({ sep: true })
  items.push({ label: 'New Folder…', icon: 'plus', value: 'new' })
  return { x: ctxMenu.value.x, y: ctxMenu.value.y, items: items }
})

async function onMovePick(value) {
  const connId = ctxMenu.value && ctxMenu.value.connId
  ctxMenu.value = null
  if (!connId) return
  if (value === 'new') {
    try {
      const folder = await createUniqueFolder()
      await applyMove(connId, folder.id)
      startRenameFolder(folder)
    } catch (e) {
      emit('toast', 'Create folder failed: ' + errMessage(e))
    }
    return
  }
  const folderId = value === 'root' ? null : value.slice(2)  // strip "f:"
  await applyMove(connId, folderId)
}

async function applyMove(connId, folderId) {
  try {
    await invoke('move_connection_to_folder', { id: connId, folderId: folderId })
    const c = connections.value.find(x => x.id === connId)
    if (c) c.folder_id = folderId
    if (folderId && !isExpanded(folderId)) expandedFolders.value.push(folderId)
  } catch (e) {
    emit('toast', 'Move failed: ' + errMessage(e))
  }
}

const CM_TOOLS = [
  { name: 'newConn',   label: 'New Connection', action: newConnection },
  { name: 'folder',    label: 'New Folder', action: newFolder },
  { sep: true },
  { name: 'edit',      label: 'Edit',   action: editSelected,   needsSel: true },
  { name: 'trash',     label: 'Delete', action: deleteSelected, needsSel: true },
  { name: 'duplicate', label: 'Duplicate', action: duplicateSelected, needsSel: true },
  { sep: true },
  { name: 'import',    label: 'Import', action: importConnections },
  { name: 'export',    label: 'Export', action: exportConnections },
  { name: 'uri',       label: 'To URI', action: copyUri, needsSel: true },
]
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">

      <!-- Title bar -->
      <div class="dlg-title">
        <div class="t">Connection Manager</div>
        <button class="dlg-close" title="Close" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <!-- Toolbar -->
      <div class="cm-toolbar">
        <template v-for="(t, i) in CM_TOOLS" :key="i">
          <div v-if="t.sep" class="tb-sep"></div>
          <button
            v-else
            class="tbtn"
            :class="{ 'tbtn-off': !t.action || (t.needsSel && !selectedId) }"
            :title="t.label"
            @click="t.action && (!t.needsSel || selectedId) && t.action()"
          >
            <span class="ic"><BaseIcon :name="t.name" :size="22" /></span>
            <span class="lbl">{{ t.label }}</span>
          </button>
        </template>
      </div>

      <!-- Filter -->
      <div class="cm-filter">
        <input
          class="fbox"
          v-model="filterText"
          placeholder="Click here to filter connections"
        />
        <span class="matches">{{ filtered.length }} matches</span>
      </div>

      <!-- Grid -->
      <div class="cm-grid">
        <table class="cmt">
          <thead>
            <tr>
              <th style="width:30%">Name</th>
              <th style="width:20%">DB Server</th>
              <th style="width:28%">Security</th>
              <th style="width:16%">Last Accessed</th>
              <th>Shortcut</th>
            </tr>
          </thead>
          <tbody>
            <template
              v-for="row in displayRows"
              :key="row.type === 'conn' ? row.conn.id : (row.key || ('folder-' + row.folder.id))"
            >
              <!-- Folder header -->
              <tr v-if="row.type === 'folder'" class="folder-row" @click="toggleFolder(row.folder.id)">
                <td colspan="5">
                  <div class="folder-head">
                    <BaseIcon
                      :name="isExpanded(row.folder.id) ? 'caretDown' : 'caret'"
                      :size="11"
                      class="folder-caret"
                    />
                    <BaseIcon name="folder" :size="15" class="folder-ic" />
                    <input
                      v-if="renamingFolderId === row.folder.id"
                      class="folder-rename-input"
                      v-model="renameText"
                      @click.stop
                      @keydown.enter="commitRenameFolder(row.folder)"
                      @keydown.esc="cancelRenameFolder"
                      @blur="commitRenameFolder(row.folder)"
                    />
                    <span
                      v-else
                      class="folder-name"
                      @dblclick.stop="startRenameFolder(row.folder)"
                    >{{ row.folder.name }}</span>
                    <span class="folder-count">{{ row.count }}</span>
                    <span class="folder-actions" @click.stop>
                      <button class="fbtn" title="Rename" @click="startRenameFolder(row.folder)">
                        <BaseIcon name="edit" :size="14" />
                      </button>
                      <button
                        class="fbtn"
                        :class="{ 'fbtn-danger': pendingDeleteId === row.folder.id }"
                        :title="pendingDeleteId === row.folder.id ? 'Click again to delete' : 'Delete folder'"
                        @click="deleteFolder(row.folder)"
                      >
                        <BaseIcon name="trash" :size="14" />
                      </button>
                    </span>
                  </div>
                </td>
              </tr>

              <!-- Empty folder hint -->
              <tr v-else-if="row.type === 'empty'" class="folder-empty-row">
                <td colspan="5">Empty — right-click a connection and choose “Move to folder”.</td>
              </tr>

              <!-- Connection row -->
              <tr
                v-else
                :class="{ sel: row.conn.id === selectedId }"
                @click="selectedId = row.conn.id; pendingDeleteId = null"
                @dblclick="editSelected"
                @contextmenu.prevent="openMoveMenu($event, row.conn)"
              >
                <td>
                  <span class="cm-name" :class="{ 'cm-indent': row.indent }">
                    <span
                      class="cm-tag"
                      :style="tagColor(row.conn.tag)
                        ? { background: tagColor(row.conn.tag) }
                        : { background: 'transparent', border: '1px solid var(--border-soft)' }"
                    >
                      <BaseIcon
                        name="dbSmall"
                        :size="12"
                        :style="tagColor(row.conn.tag) ? { color: '#fff' } : { color: 'var(--text-faint)' }"
                      />
                    </span>
                    {{ row.conn.name }}
                  </span>
                </td>
                <td>{{ parseDbServer(row.conn) }}</td>
                <td>
                  <span v-if="parseSecurity(row.conn)" class="cm-key">
                    <BaseIcon name="lock" :size="13" />
                    {{ parseSecurity(row.conn) }}
                  </span>
                  <span v-else class="muted">—</span>
                </td>
                <td><span class="muted">{{ row.conn.last_accessed ?? '—' }}</span></td>
                <td></td>
              </tr>
            </template>

            <tr v-if="displayRows.length === 0">
              <td colspan="5" style="text-align:center;padding:24px;color:var(--text-faint)">
                No connections found.
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Footer -->
      <div class="cm-footer">
        <label class="chk-line">
          <span class="cb" :class="{ on: showOnStartup }" @click="showOnStartup = !showOnStartup">
            <BaseIcon v-if="showOnStartup" name="check" :size="12" />
          </span>
          Show on startup
        </label>
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Close</button>
        <button class="btn primary" :disabled="!selectedId" @click="connectSelected">Connect</button>
      </div>

    </div>
  </div>

  <!-- Move-to-folder context menu (reuses the app's ContextMenu) -->
  <ContextMenu
    v-if="moveMenuModel"
    :menu="moveMenuModel"
    @close="ctxMenu = null"
    @pick="onMovePick"
  />

  <!-- New Connection modal -->
  <NewConnection
    v-if="showNewConnection"
    @close="showNewConnection = false"
    @saved="onConnectionSaved"
  />

  <!-- Edit Connection modal -->
  <NewConnection
    v-if="showEditConnection"
    :edit-conn="connections.find(c => c.id === selectedId)"
    @close="showEditConnection = false"
    @updated="onConnectionUpdated"
  />
</template>

<style scoped>
/* ---- overlay & dialog ---- */
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 50;
}

.dialog {
  width: 1180px;
  max-width: 94vw;
  height: 660px;
  max-height: 92vh;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.6), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ---- title bar ---- */
.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 14px;
  position: relative;
}
.dlg-title .t {
  position: absolute;
  left: 0; right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}
.dlg-close {
  margin-left: auto;
  z-index: 1;
  display: grid;
  place-items: center;
  width: 24px; height: 24px;
  border: none;
  background: none;
  border-radius: 6px;
  color: var(--text-dim);
  cursor: pointer;
}
.dlg-close:hover { background: var(--bg-hover); color: var(--text); }

/* ---- toolbar ---- */
.cm-toolbar {
  display: flex;
  align-items: stretch;
  gap: 2px;
  padding: 8px;
  border-bottom: 1px solid var(--border);
  flex: none;
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
}
.tbtn:hover { background: var(--bg-hover); }
.tbtn .ic { color: var(--text-dim); }
.tbtn:hover .ic { color: var(--text); }
.tbtn .lbl { font-size: 10.5px; color: var(--text-dim); white-space: nowrap; }
.tbtn-off { opacity: .38; cursor: default; }
.tbtn-off:hover { background: none; }
.tb-sep {
  width: 1px;
  background: var(--border-soft);
  margin: 6px 4px;
}

/* ---- filter row ---- */
.cm-filter {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.fbox {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 7px 11px;
  font-size: 12.5px;
  color: var(--text);
  outline: none;
}
.fbox:focus { border-color: var(--accent); }
.matches { font-size: 12.5px; color: var(--text-dim); white-space: nowrap; }

/* ---- table ---- */
.cm-grid {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

table.cmt {
  border-collapse: collapse;
  width: 100%;
  font-size: 13px;
}
table.cmt th {
  position: sticky;
  top: 0;
  background: var(--bg-panel-2);
  color: var(--text);
  text-align: left;
  font-weight: 600;
  padding: 7px 12px;
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  z-index: 2;
  white-space: nowrap;
}
table.cmt td {
  padding: 8px 12px;
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  color: var(--text);
  white-space: nowrap;
}
table.cmt tbody tr { cursor: default; user-select: none; }
table.cmt tbody tr:hover:not(.sel) { background: var(--bg-hover); }
table.cmt tr.sel td { background: var(--accent); color: #fff; }
table.cmt tr.sel .muted { color: rgba(255,255,255,.8); }

.cm-name {
  display: flex;
  align-items: center;
  gap: 9px;
}
.cm-tag {
  width: 16px;
  height: 16px;
  border-radius: 3px;
  display: grid;
  place-items: center;
  flex: none;
}
.cm-key {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-dim);
}
table.cmt tr.sel .cm-key { color: rgba(255,255,255,.85); }
.muted { color: var(--text-dim); }

/* ---- folders ---- */
.folder-row { cursor: default; user-select: none; }
.folder-row:hover td { background: var(--bg-hover); }
.folder-row td { background: var(--bg-panel-2); }
.folder-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.folder-caret { color: var(--text-dim); flex: none; }
.folder-ic { color: var(--text-dim); flex: none; }
.folder-name { font-weight: 600; color: var(--text); }
.folder-count {
  font-size: 11.5px;
  color: var(--text-dim);
  background: var(--bg-input);
  border-radius: 9px;
  padding: 1px 8px;
}
.folder-rename-input {
  background: var(--bg-input);
  border: 1px solid var(--accent);
  border-radius: 5px;
  padding: 2px 7px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  outline: none;
  min-width: 160px;
}
.folder-actions {
  margin-left: auto;
  display: flex;
  gap: 2px;
  opacity: 0;
}
.folder-row:hover .folder-actions { opacity: 1; }
.fbtn {
  display: grid;
  place-items: center;
  width: 26px; height: 24px;
  border: none;
  background: none;
  border-radius: 5px;
  color: var(--text-dim);
  cursor: pointer;
}
.fbtn:hover { background: var(--bg-window); color: var(--text); }
.fbtn-danger, .fbtn-danger:hover { color: #fff; background: #e05a4d; }

.cm-indent { padding-left: 22px; }

.folder-empty-row td {
  padding: 6px 12px 6px 44px;
  color: var(--text-faint);
  font-size: 12px;
  font-style: italic;
}

/* ---- footer ---- */
.cm-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  flex: none;
}
.spacer { flex: 1; }

.chk-line {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: var(--text-dim);
  cursor: pointer;
  user-select: none;
}
.cb {
  width: 17px; height: 17px;
  border-radius: 4px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  display: grid;
  place-items: center;
  flex: none;
}
.cb.on { background: var(--accent); border-color: var(--accent); color: #fff; }

.btn {
  padding: 8px 20px;
  border-radius: 7px;
  font-size: 13px;
  border: 1px solid var(--border-soft);
  background: var(--bg-toolbar);
  color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:hover { background: var(--accent-soft); }
.btn:disabled { opacity: .4; cursor: default; }
</style>
