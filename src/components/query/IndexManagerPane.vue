<script setup>
import { computed, ref, inject, onMounted, onUnmounted } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'
import IndexAddDialog from './IndexAddDialog.vue'
import {
  isProtectedIndex, isIndexHidden, indexKeyLabel, indexType, indexProperties,
} from '../../utils/indexSpec'

// The Index Manager tab (kind: 'indexes'). It's a thin view over the shared
// useIndexes composable — the same state that drives the native Index menu — so
// the toolbar buttons and the menu act on the one selected index. Provided
// app-wide by App.vue as appModals.indexes.
const props = defineProps({
  activeTab: { type: Object, required: true },
})

const bundle = inject('appModals')
const idx = bundle.indexes
const showToast = bundle.handlers.showToast

const {
  indexesList, indexesLoading, indexesError, selectedIndex,
  indexSizes, indexUsage, indexTotalSize,
  indexFormOpen, indexFormMode, indexFormSeed, indexCreating,
  loadIndexes, openIndexes, closeIndexesModal, openCreateIndex, closeIndexForm, submitIndex,
  startEditIndex, openIndexDetails, copyIndex, openDropIndexConfirm, setIndexHidden,
} = idx

// Load this tab's collection on mount; clear the shared selection/target on unmount
// so the Index menu disables again when a non-index tab becomes active.
onMounted(() => {
  openIndexes({
    connId: props.activeTab.connId,
    dbName: props.activeTab.dbName,
    collName: props.activeTab.collName,
  })
})
onUnmounted(() => { closeIndexesModal() })

// --- toolbar enablement ---
const hasSel      = computed(() => !!selectedIndex.value)
const selProtected = computed(() => !!selectedIndex.value && isProtectedIndex(selectedIndex.value.name))
const selHidden   = computed(() => !!selectedIndex.value && isIndexHidden(selectedIndex.value))

function selectRow(index) { selectedIndex.value = index }

// Hide / Unhide is one toggle button whose label follows the selected index.
function toggleHidden() { setIndexHidden(!selHidden.value) }

// Paste: create an index from a JSON spec on the clipboard (the shape produced by
// "Copy index"). Pre-fills the Add-index form so the user confirms before writing.
async function pasteIndex() {
  let text = ''
  try { text = await navigator.clipboard.readText() } catch (e) { text = '' }
  if (!text.trim()) { showToast('Clipboard is empty'); return }
  let spec
  try { spec = JSON.parse(text) } catch (e) { showToast('Clipboard is not a valid index spec'); return }
  if (!spec || typeof spec.key !== 'object') { showToast('Clipboard is not an index spec'); return }
  // Seed the Add dialog with the pasted spec (drop the name so it doesn't collide).
  const seed = Object.assign({}, spec)
  delete seed.name
  openCreateIndex(seed)
}

// --- row rendering ---
const expanded = ref({})   // index name -> whether its key spec is expanded
function toggleExpand(name) { expanded.value[name] = !expanded.value[name] }

function typeOf(index)   { return indexType(index) }
function propsOf(index)  { const p = indexProperties(index); return p.length ? p.join(', ') : '—' }
function sizeOf(index)   { return fmtBytes(indexSizes.value[index.name]) }
function usageOf(index)  { const u = indexUsage.value[index.name]; return u == null ? 'n/a' : u }

// Binary byte sizes, matching how collStats reports index sizes (e.g. "124.0 KiB").
function fmtBytes(bytes) {
  if (bytes == null) return 'n/a'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KiB', 'MiB', 'GiB', 'TiB']
  let value = bytes / 1024
  let i = 0
  while (value >= 1024 && i < units.length - 1) { value /= 1024; i++ }
  return `${value.toFixed(1)} ${units[i]}`
}
</script>

<template>
  <div class="idxm">
    <!-- Breadcrumb -->
    <div class="crumbs">
      <BaseIcon name="connect" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.connName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="dbSmall" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.dbName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="collSmall" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.collName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="anchor" :size="15" class="c-ic" />
      <span class="crumb">Indexes</span>
    </div>

    <!-- Toolbar -->
    <div class="idx-toolbar">
      <button class="tb" @click="loadIndexes()"><BaseIcon name="refresh" :size="16" /><span>Refresh</span></button>
      <button class="tb" @click="openCreateIndex()"><BaseIcon name="plus" :size="16" /><span>Add index</span></button>
      <span class="tb-sep"></span>
      <button class="tb" :disabled="!hasSel || selProtected" @click="openDropIndexConfirm()"><BaseIcon name="trash" :size="16" /><span>Drop index</span></button>
      <button class="tb" :disabled="!hasSel || selProtected" @click="startEditIndex()"><BaseIcon name="edit" :size="16" /><span>Edit index</span></button>
      <button class="tb" :disabled="!hasSel" @click="openIndexDetails()"><BaseIcon name="eye" :size="16" /><span>View details</span></button>
      <button class="tb" :disabled="!hasSel || selProtected" @click="toggleHidden()"><BaseIcon :name="selHidden ? 'eye' : 'eyeOff'" :size="16" /><span>{{ selHidden ? 'Unhide index' : 'Hide index' }}</span></button>
      <span class="tb-sep"></span>
      <button class="tb" :disabled="!hasSel" @click="copyIndex()"><BaseIcon name="copy" :size="16" /><span>Copy</span></button>
      <button class="tb" @click="pasteIndex()"><BaseIcon name="paste" :size="16" /><span>Paste</span></button>
    </div>

    <!-- Index list -->
    <div class="idx-body">
      <div v-if="indexesLoading" class="idx-msg">Loading indexes…</div>
      <div v-else-if="indexesError" class="idx-msg idx-err">{{ indexesError }}</div>
      <table v-else class="idx-table">
        <thead>
          <tr>
            <th class="col-name">Name</th>
            <th class="col-type">Type</th>
            <th class="col-props">Properties</th>
            <th class="col-size">Size</th>
            <th class="col-usage">Usage</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="index in indexesList" :key="index.name">
            <tr
              class="idx-row"
              :class="{ selected: selectedIndex && selectedIndex.name === index.name }"
              @click="selectRow(index)"
            >
              <td class="col-name">
                <span class="name-inner">
                  <button class="caret" :class="{ open: expanded[index.name] }" @click.stop="toggleExpand(index.name)">
                    <BaseIcon name="caret" :size="11" />
                  </button>
                  {{ index.name }}
                </span>
              </td>
              <td class="col-type">{{ typeOf(index) }}</td>
              <td class="col-props">{{ propsOf(index) }}</td>
              <td class="col-size">{{ sizeOf(index) }}</td>
              <td class="col-usage">{{ usageOf(index) }}</td>
            </tr>
            <tr v-if="expanded[index.name]" class="idx-detail">
              <td colspan="5"><span class="dt-label">Fields:</span> {{ indexKeyLabel(index) || '—' }}</td>
            </tr>
          </template>
          <tr v-if="!indexesList.length"><td colspan="5" class="idx-empty">No indexes.</td></tr>
        </tbody>
      </table>
    </div>

    <!-- Status bar -->
    <div class="idx-status">
      <span>{{ indexesList.length }} {{ indexesList.length === 1 ? 'Index' : 'Indexes' }}</span>
      <span class="spacer"></span>
      <span v-if="indexTotalSize != null">{{ fmtBytes(indexTotalSize) }}</span>
    </div>

    <!-- Add / Edit index dialog -->
    <IndexAddDialog
      v-if="indexFormOpen"
      :mode="indexFormMode"
      :seed="indexFormSeed"
      :busy="indexCreating"
      :error="indexesError"
      @submit="submitIndex"
      @cancel="closeIndexForm"
    />
  </div>
</template>

<style scoped>
.idxm { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

/* Breadcrumb (mirrors the collection tab) */
.crumbs {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 14px; font-size: 12.5px; color: var(--text-dim);
  border-bottom: 1px solid var(--border); flex: none;
}
.sep { color: var(--text-faint); }
.c-ic { color: var(--text-faint); }

/* Toolbar */
.idx-toolbar {
  display: flex; align-items: center; gap: 2px;
  padding: 5px 8px; background: var(--bg-toolbar);
  border-bottom: 1px solid var(--border); flex: none;
}
.tb {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 4px 8px; border: none; background: transparent;
  color: var(--text); font-size: 12.5px; border-radius: var(--radius);
  cursor: pointer;
}
.tb:hover:not(:disabled) { background: var(--bg-hover); }
.tb:disabled { color: var(--text-faint); cursor: default; }
.tb-sep { width: 1px; align-self: stretch; margin: 3px 6px; background: var(--border); }

/* Table */
.idx-body { flex: 1; overflow: auto; min-height: 0; }
.idx-table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
.idx-table thead th {
  position: sticky; top: 0; z-index: 1;
  text-align: left; font-weight: 600; color: var(--text-dim);
  background: var(--bg-panel); padding: 6px 10px;
  border-bottom: 1px solid var(--border); border-right: 1px solid var(--border-soft);
}
.idx-row td {
  padding: 5px 10px; color: var(--text); vertical-align: middle;
  border-bottom: 1px solid var(--grid-line); border-right: 1px solid var(--border-soft);
  white-space: nowrap;
}
.idx-row { cursor: pointer; }
.idx-row:hover { background: var(--bg-hover); }
.idx-row.selected { background: var(--accent); color: #fff; }
.idx-row.selected td { color: #fff; }
.col-name { white-space: nowrap; }
.name-inner { display: inline-flex; align-items: center; gap: 4px; vertical-align: middle; }
.caret {
  border: none; background: transparent; padding: 0; cursor: pointer;
  color: var(--text-faint); display: inline-flex; transition: transform .12s;
}
.caret.open { transform: rotate(90deg); }
.idx-row.selected .caret { color: #fff; }
.idx-detail td {
  padding: 4px 10px 6px 30px; font-size: 12px; color: var(--text-dim);
  background: var(--bg-row-alt); border-bottom: 1px solid var(--grid-line);
}
.dt-label { color: var(--text-faint); margin-right: 4px; }
.idx-empty { padding: 14px 10px; color: var(--text-dim); }
.idx-msg { padding: 14px; color: var(--text-dim); font-size: 12.5px; }
.idx-err { color: var(--danger-text); }

/* Status bar */
.idx-status {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 12px; font-size: 12px; color: var(--text-dim);
  background: var(--bg-toolbar); border-top: 1px solid var(--border); flex: none;
}
.idx-status .spacer { flex: 1; }

</style>
