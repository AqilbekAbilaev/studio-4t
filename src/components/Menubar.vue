<script setup>
import { ref } from 'vue'
import BaseIcon from './BaseIcon.vue'

// Our own flat, dark, themeable menu bar — same visual language as ContextMenu.
// Studio 3T is only the reference for *which* items exist; the look is ours.
// `context` tells us what's selected so items grey out live, like a real menu:
//   { hasConnection, hasDatabase, hasCollection }
// Item flags: built:false = feature not shipped yet (always greyed); needs =
// the selection an item acts on; danger = destructive (red); sub = has a
// submenu affordance; check/checked = radio-style state.
const props = defineProps({
  context: { type: Object, default: () => ({}) },
})
const emit = defineEmits(['action'])

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)

const MENUS = [
  {
    name: 'File',
    items: [
      { id: 'file:connect', label: 'Connect…', shortcut: 'Ctrl+N' },
      { id: 'file:add_database', label: 'Add Database…', built: false },
      { sep: true },
      { id: 'file:intellishell', label: 'Open IntelliShell', shortcut: 'Ctrl+L', needs: 'database' },
      { id: 'file:sql', label: 'Open SQL', shortcut: 'Shift+Ctrl+L' },
      { id: 'file:tasks', label: 'Open Tasks', built: false },
      { id: 'file:search', label: 'Search in…', needs: 'database' },
      { id: 'file:manage_sql', label: 'Manage SQL Connections', built: false },
      { sep: true },
      { id: 'file:load', label: 'Load', built: false, sub: true },
      { id: 'file:save', label: 'Save', built: false, sub: true },
      { sep: true },
      { id: 'file:server_charts', label: 'Server Status Charts', built: false },
      { id: 'file:server_status', label: 'Server Status', needs: 'connection' },
      { id: 'file:server_build', label: 'Server Build Info', needs: 'connection' },
      { sep: true },
      { id: 'file:exit', label: 'Exit', shortcut: 'Ctrl+Q' },
    ],
  },
  {
    name: 'Edit',
    items: [
      { id: 'edit:copy', label: 'Copy', built: false },
      { id: 'edit:copy_value', label: 'Copy Value', built: false },
      { id: 'edit:copy_field', label: 'Copy Field', built: false },
      { id: 'edit:copy_field_path', label: 'Copy Field Path', built: false },
      { id: 'edit:copy_document', label: 'Copy Document', built: false },
      { id: 'edit:paste_documents', label: 'Paste Document(s)', built: false },
      { sep: true },
      { id: 'edit:preferences', label: 'Preferences…', shortcut: 'Ctrl+P' },
    ],
  },
  {
    name: 'Database',
    items: [
      { id: 'db:add_database', label: 'Add Database…', built: false },
      { id: 'db:copy_database', label: 'Copy Database', built: false },
      { id: 'db:copy_all', label: 'Copy All Collections/Views/Buckets', built: false },
      { id: 'db:paste_database', label: 'Paste Database', built: false },
      { id: 'db:paste', label: 'Paste', built: false },
      { sep: true },
      { id: 'db:export', label: 'Export Collections…', built: false },
      { id: 'db:import', label: 'Import Collections…', built: false },
      { sep: true },
      { id: 'db:drop_database', label: 'Drop Database', needs: 'database', danger: true },
      { sep: true },
      { id: 'db:add_collection', label: 'Add Collection…', needs: 'database' },
      { id: 'db:add_view', label: 'Add View…', built: false },
      { id: 'db:add_bucket', label: 'Add GridFS Bucket…', built: false },
      { sep: true },
      { id: 'db:manage_users', label: 'Manage Users', built: false },
      { id: 'db:manage_roles', label: 'Manage Roles', built: false },
      { id: 'db:functions', label: 'Add / Edit Stored Functions', built: false },
      { sep: true },
      { id: 'db:database_stats', label: 'Database Statistics', built: false },
      { id: 'db:collection_stats', label: 'Collection Statistics', needs: 'collection' },
      { id: 'db:current_ops', label: 'Current Operations', built: false },
    ],
  },
  {
    name: 'Collection',
    items: [
      { id: 'coll:open_tab', label: 'Open Collection Tab', shortcut: 'F10', needs: 'connection' },
      { id: 'coll:aggregation', label: 'Open Aggregation Editor', shortcut: 'F4', needs: 'collection' },
      { id: 'coll:mapreduce', label: 'Open Map-Reduce', built: false },
      { sep: true },
      { id: 'coll:insert_document', label: 'Insert Document…', built: false },
      { id: 'coll:update_dialog', label: 'Update Dialog…', built: false },
      { id: 'coll:delete_dialog', label: 'Delete Dialog…', built: false },
      { id: 'coll:vqb', label: 'Show Visual Query Builder', shortcut: 'Ctrl+B', needs: 'collection' },
      { sep: true },
      { id: 'coll:export', label: 'Export…', needs: 'collection' },
      { id: 'coll:import', label: 'Import…', needs: 'collection' },
      { id: 'coll:copy', label: 'Copy Collection', built: false },
      { sep: true },
      { id: 'coll:add_index', label: 'Add Index…', needs: 'collection' },
      { id: 'coll:validator', label: 'Add / Edit Validator…', built: false },
      { id: 'coll:add_view', label: 'Add View Here…', built: false },
      { id: 'coll:stats', label: 'Collection Stats', needs: 'collection' },
      { id: 'coll:mask', label: 'Mask Collection/View', needs: 'collection' },
      { id: 'coll:schema', label: 'View Schema', needs: 'collection' },
      { id: 'coll:reschema', label: 'Reschema…', built: false },
      { id: 'coll:compare', label: 'Compare To…', needs: 'database' },
      { sep: true },
      { id: 'coll:rename', label: 'Rename Collection…', needs: 'collection' },
      { id: 'coll:duplicate', label: 'Duplicate Collection…', needs: 'collection' },
      { id: 'coll:clear', label: 'Clear Collection', built: false },
      { id: 'coll:drop', label: 'Drop Collection…', needs: 'collection', danger: true },
    ],
  },
  {
    name: 'Index',
    items: [
      { id: 'idx:edit', label: 'Edit Index…', built: false },
      { id: 'idx:view', label: 'View Details', built: false },
      { id: 'idx:copy', label: 'Copy Index', built: false },
      { id: 'idx:drop', label: 'Drop Index', built: false, danger: true },
      { sep: true },
      { id: 'idx:hide', label: 'Hide Index', built: false },
      { id: 'idx:unhide', label: 'Unhide Index', built: false },
    ],
  },
  {
    name: 'Document',
    items: [
      { id: 'doc:edit_value', label: 'Edit Value / Type…', built: false },
      { id: 'doc:remove_field', label: 'Remove Field', built: false },
      { id: 'doc:rename_field', label: 'Rename Field…', built: false },
      { id: 'doc:add_field', label: 'Add Field / Value…', built: false },
      { sep: true },
      { id: 'doc:view_json', label: 'View Document (JSON)…', built: false },
      { id: 'doc:edit_json', label: 'Edit Document (JSON)…', built: false },
      { id: 'doc:delete', label: 'Delete Document', built: false, danger: true },
    ],
  },
  {
    name: 'GridFS',
    items: [
      { id: 'gridfs:open', label: 'Open GridFS View', needs: 'database' },
      { sep: true },
      { id: 'gridfs:view_file', label: 'View File', built: false },
      { id: 'gridfs:rename', label: 'Rename File…', built: false },
      { id: 'gridfs:meta', label: 'Edit Meta Data…', built: false },
      { id: 'gridfs:save', label: 'Save To Disk…', built: false },
      { id: 'gridfs:remove', label: 'Remove File(s)', built: false },
      { id: 'gridfs:add', label: 'Add File(s)…', built: false },
      { sep: true },
      { id: 'gridfs:copy_bucket', label: 'Copy Bucket', built: false },
      { id: 'gridfs:drop_bucket', label: 'Drop Bucket', built: false, danger: true },
    ],
  },
  {
    name: 'View',
    items: [
      { id: 'view:refresh', label: 'Refresh', shortcut: 'Ctrl+R', needs: 'connection' },
      { id: 'view:refresh_document', label: 'Refresh Document', built: false },
      { sep: true },
      { id: 'view:step_column', label: 'Step Into Column', built: false },
      { id: 'view:step_cell', label: 'Step Into Cell', built: false },
      { id: 'view:step_out', label: 'Step Out', built: false },
      { sep: true },
      { id: 'view:tree', label: 'Tree View', built: false, check: true },
      { id: 'view:table', label: 'Table View', built: false, check: true },
      { id: 'view:json', label: 'JSON View', built: false, check: true, checked: true },
      { sep: true },
      { id: 'view:next_tab', label: 'Next Tab', built: false },
      { id: 'view:prev_tab', label: 'Previous Tab', built: false },
      { id: 'view:close_tab', label: 'Close Tab', built: false },
      { id: 'view:close_tab_np', label: 'Close Tab (No Prompt)', built: false },
      { sep: true },
      { id: 'view:split_v', label: 'Split Vertically', built: false },
      { id: 'view:split_h', label: 'Split Horizontally', built: false },
      { id: 'view:history', label: 'History Manager…', built: false },
      { id: 'view:hide_toolbar', label: 'Hide Global Toolbar', built: false },
    ],
  },
  {
    name: 'Help',
    items: [
      { id: 'help:shortcuts', label: 'Keyboard Shortcuts' },
      { sep: true },
      { id: 'help:license', label: 'My License', built: false },
      { id: 'help:about', label: 'About…', built: false },
      { id: 'help:gallery', label: 'Feature Gallery', built: false },
      { id: 'help:quickstart', label: 'Quickstart', built: false },
      { id: 'help:whats_new', label: "What's New", built: false },
      { id: 'help:updates', label: 'Check for Updates…', built: false },
      { sep: true },
      { id: 'help:support', label: 'Contact Support', built: false },
      { id: 'help:feature_request', label: 'Submit a Feature Request', built: false },
      { id: 'help:feedback', label: 'Submit Feedback', built: false },
      { id: 'help:tutorials', label: 'In-app Tutorials', built: false, sub: true },
      { id: 'help:knowledge_base', label: 'Knowledge Base', built: false },
    ],
  },
]

const openMenu = ref(null)

function toggle(name) {
  openMenu.value = openMenu.value === name ? null : name
}
function hover(name) {
  // Once a menu is open, sliding across the bar switches menus (standard UX).
  if (openMenu.value) openMenu.value = name
}
function close() {
  openMenu.value = null
}

function isEnabled(item) {
  if (item.built === false) return false
  if (!item.needs) return true
  const c = props.context || {}
  if (item.needs === 'connection') return !!c.hasConnection
  if (item.needs === 'database') return !!c.hasDatabase
  if (item.needs === 'collection') return !!c.hasCollection
  return true
}

function pick(item) {
  if (item.sub || !isEnabled(item)) return
  emit('action', item.id)
  close()
}

function fmtShortcut(sc) {
  if (!sc) return ''
  if (!isMac) return sc
  return sc
    .replace(/Ctrl/g, '⌘')
    .replace(/Shift/g, '⇧')
    .replace(/Alt/g, '⌥')
    .replace(/\+/g, '')
}

defineExpose({ close })
</script>

<template>
  <div class="menubar">
    <div
      v-for="menu in MENUS"
      :key="menu.name"
      class="mb-top"
      :class="{ open: openMenu === menu.name }"
      @click.stop="toggle(menu.name)"
      @mouseenter="hover(menu.name)"
    >
      <span class="mb-top-label">{{ menu.name }}</span>

      <div v-if="openMenu === menu.name" class="mb-drop" @click.stop>
        <template v-for="(item, i) in menu.items" :key="i">
          <div v-if="item.sep" class="mb-sep"></div>
          <div
            v-else
            class="mb-item"
            :class="{ disabled: !isEnabled(item), danger: item.danger && isEnabled(item) }"
            @click="pick(item)"
          >
            <span class="mb-check">
              <span v-if="item.check && item.checked" class="mb-dot"></span>
            </span>
            <span class="mb-label">{{ item.label }}</span>
            <span v-if="item.shortcut" class="mb-sc">{{ fmtShortcut(item.shortcut) }}</span>
            <span v-if="item.sub" class="mb-caret"><BaseIcon name="caret" :size="12" /></span>
          </div>
        </template>
      </div>
    </div>
  </div>

  <!-- click-away catcher, only while a menu is open -->
  <div v-if="openMenu" class="mb-backdrop" @mousedown="close" @contextmenu.prevent="close"></div>
</template>

<style scoped>
.menubar {
  display: flex;
  align-items: stretch;
  height: 30px;
  padding: 0 4px;
  background: var(--bg-titlebar);
  border-bottom: 1px solid var(--border);
  user-select: none;
  position: relative;
  z-index: 92;
}
.mb-top {
  position: relative;
  display: flex;
  align-items: center;
  padding: 0 10px;
  font-size: 13px;
  color: var(--text);
  cursor: default;
}
.mb-top:hover { background: var(--bg-hover); }
.mb-top.open { background: var(--bg-active); }
.mb-top-label { line-height: 1; }

.mb-drop {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 1px;
  min-width: 240px;
  background: var(--bg-menu);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 18px 48px rgba(0, 0, 0, .6);
  padding: 5px;
  z-index: 93;
}
.mb-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px 6px 8px;
  border-radius: 5px;
  font-size: 13px;
  color: var(--text);
  white-space: nowrap;
  cursor: default;
}
.mb-item:hover { background: var(--accent); color: #fff; }
.mb-item:hover .mb-sc { color: rgba(255, 255, 255, .85); }
.mb-item.danger { color: var(--danger-text); }
.mb-item.danger:hover { background: var(--danger); color: #fff; }
.mb-item.disabled {
  color: var(--text-faint);
  pointer-events: none;
}
.mb-check {
  width: 14px;
  flex: none;
  display: grid;
  place-items: center;
}
.mb-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
}
.mb-label { flex: 1; }
.mb-sc {
  color: var(--text-faint);
  font-size: 12px;
  margin-left: 28px;
  letter-spacing: .5px;
}
.mb-caret { color: var(--text-faint); margin-left: 8px; display: grid; place-items: center; }
.mb-backdrop { position: fixed; inset: 0; z-index: 91; }
</style>
