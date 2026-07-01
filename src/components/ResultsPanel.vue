<script setup>
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { parseField } from '../utils/queryParser'
import BaseIcon from './BaseIcon.vue'
import DocumentModal from './DocumentModal.vue'
import VisualQueryBuilder from './VisualQueryBuilder.vue'
import ResultTable from './ResultTable.vue'
import TreeView from './TreeView.vue'
import StateMessage from './StateMessage.vue'
import { mongoStringify, syntaxHighlight } from '../utils/mongoFormat'

const props = defineProps({
  activeTab:   { type: Object,  required: true },
  isAggregate: { type: Boolean, default: false },
  runValid:    { type: Boolean, default: true },
  rtab:        { type: String,  default: 'Result' },
  vqbOpen:     { type: Boolean, default: false },
  tabs:        { type: Array,   required: true },
  activeTabId: { type: String,  required: true },
})

// `run` re-runs the active tab in its current mode (the toolbar refresh button).
// `requery` re-runs the find query with an explicit history flag (pagination, CRUD
// refresh). Both delegate to the parent, which owns the parse + run pipeline.
const emit = defineEmits(['run', 'requery', 'select-rtab', 'open-vqb', 'close-vqb', 'toast', 'cancel'])

const viewMode     = ref('table')
const viewMenu     = ref(false)
const pageSizeMenu = ref(false)

// Drag-to-VQB signals originate in the grid (ResultTable) and are forwarded to
// VisualQueryBuilder, which sits beside the grid here. ResultTable owns the gesture;
// these plain refs just relay its latest field / section / drop to the VQB props.
const draggedField    = ref('')
const dragOverSection = ref(null)
const vqbDrop         = ref(null)

// ── VQB panel resize ──────────────────────────────────────
// Mirrors the connection sidebar (App.vue): a thin resizer bar between the grid
// and the panel, dragged with the mouse. The panel is on the right, so dragging
// right narrows it. Width is persisted so it survives toggling and restarts.
function loadVqbWidth() {
  const saved = parseInt(localStorage.getItem('vqbWidth'), 10)
  if (Number.isFinite(saved)) return Math.max(280, Math.min(760, saved))
  return 360
}
const vqbWidth     = ref(loadVqbWidth())
const vqbResizing  = ref(false)

function startVqbResize(e) {
  e.preventDefault()
  const startX = e.clientX
  const startW = vqbWidth.value
  vqbResizing.value = true
  const onMove = (ev) => {
    vqbWidth.value = Math.max(280, Math.min(760, startW + (startX - ev.clientX)))
  }
  const onUp = () => {
    vqbResizing.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    localStorage.setItem('vqbWidth', String(vqbWidth.value))
  }
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}


// ── pagination ─────────────────────────────────────────
const PAGE_SIZES = [10, 25, 50, 100, 200]

function goFirst() {
  const tab = props.activeTab
  if (!tab) return
  tab.skip = 0
  emit('requery', false)
}

function goPrev() {
  const tab = props.activeTab
  if (!tab) return
  tab.skip = Math.max(0, (tab.skip || 0) - (tab.limit || 50))
  emit('requery', false)
}

function goNext() {
  const tab = props.activeTab
  if (!tab) return
  tab.skip = (tab.skip || 0) + (tab.limit || 50)
  emit('requery', false)
}

// Count the documents matching the tab's current filter. The result is cached on
// the tab together with the filter it was counted for, so the "of N" total is
// only shown while it still matches the active filter (see rangeText).
async function fetchCount(tab) {
  // Convert the tab's shell-syntax filter to canonical Extended JSON before sending,
  // exactly as the run-query path does — the backend's parser is strict and rejects
  // shell conveniences like unquoted keys.
  const parsed = parseField(tab.filter || '')
  if (!parsed.ok) throw new Error(parsed.error)
  const filter = parsed.ejson
  const total = await invoke('count_documents', {
    id:         tab.connectionId,
    database:   tab.dbName,
    collection: tab.collectionName,
    filter:     filter,
  })
  tab.total = total
  tab.totalFilter = filter
  return total
}

async function goLast() {
  const tab = props.activeTab
  if (!tab) return
  try {
    const total = await fetchCount(tab)
    const limit = tab.limit || 50
    // Land on the page whose first row is the last full page boundary.
    tab.skip = total === 0 ? 0 : Math.floor((total - 1) / limit) * limit
    emit('requery', false)
  } catch (e) {
    emit('toast', 'Count failed: ' + errMessage(e))
  }
}

async function countDocuments() {
  const tab = props.activeTab
  if (!tab || isCountDisabled.value) return
  try {
    const total = await fetchCount(tab)
    emit('toast', `${total.toLocaleString()} document${total !== 1 ? 's' : ''} match this query`)
  } catch (e) {
    emit('toast', 'Count failed: ' + errMessage(e))
  }
}

function setPageSize(size) {
  const tab = props.activeTab
  if (!tab) return
  tab.limit = size
  tab.skip = 0
  pageSizeMenu.value = false
  emit('requery', true)
}

// ── copy document (toolbar button) ─────────────────────
function copySelectedDocument() {
  const tab = props.activeTab
  if (!tab || tab.selectedRow < 0) return
  navigator.clipboard.writeText(JSON.stringify(tab.results[tab.selectedRow], null, 2))
}

// ── drill into nested object cells ─────────────────────
// field-name path navigated into, e.g. ['bank_account', 'account']. Owned here
// (not in ResultTable) so it survives switching to the JSON / Explain view and back,
// and so the run-reset below has a stable place to live.
const drillPath = ref([])

// The parent's run pipeline no longer clears the drill path directly; reset it on the
// rising edge of isRunning so every fresh run (refresh, pagination, query bar) starts
// at the top level. The grid shows its loading skeleton while isRunning, so the reset
// is never visible mid-flight.
watch(() => props.activeTab && props.activeTab.isRunning, (running, prev) => {
  if (running && !prev) drillPath.value = []
})

// Reset the drill path when switching tabs so the new collection opens at top level.
watch(() => props.activeTab?.id, () => { drillPath.value = [] })

// ── document CRUD ──────────────────────────────────────
const showDocModal     = ref(false)
const docModalMode     = ref('insert')
const showDeleteConfirm = ref(false)
const crudError        = ref(null)
const crudSaving       = ref(false)

function openInsert() {
  docModalMode.value = 'insert'
  crudError.value = null
  showDocModal.value = true
}

function openEdit() {
  docModalMode.value = 'edit'
  crudError.value = null
  showDocModal.value = true
}

function buildIdFilter(doc) {
  return JSON.stringify({ _id: doc._id })
}

async function onDocSave(jsonStr) {
  crudSaving.value = true
  crudError.value = null
  const tab = props.activeTab
  try {
    if (docModalMode.value === 'insert') {
      await invoke('insert_document', {
        id: tab.connectionId,
        database: tab.dbName,
        collection: tab.collectionName,
        document: jsonStr,
      })
    } else {
      const original = tab.results[tab.selectedRow]
      await invoke('replace_document', {
        id: tab.connectionId,
        database: tab.dbName,
        collection: tab.collectionName,
        idFilter: buildIdFilter(original),
        document: jsonStr,
      })
    }
    showDocModal.value = false
    emit('requery', true)
  } catch (e) {
    crudError.value = errMessage(e)
  } finally {
    crudSaving.value = false
  }
}

async function onDeleteConfirm() {
  const tab = props.activeTab
  const original = tab.results[tab.selectedRow]
  crudError.value = null
  try {
    await invoke('delete_document', {
      id: tab.connectionId,
      database: tab.dbName,
      collection: tab.collectionName,
      idFilter: buildIdFilter(original),
    })
    showDeleteConfirm.value = false
    tab.selectedRow = -1
    emit('requery', true)
  } catch (e) {
    crudError.value = errMessage(e)
  }
}

// Best-effort headline metrics pulled from the explain document; the full plan
// is always shown below as formatted JSON.
const explainSummary = computed(() => {
  const r = props.activeTab && props.activeTab.explainResult
  if (!r) return null
  const stats = r.executionStats || {}
  const winning = (r.queryPlanner && r.queryPlanner.winningPlan) || {}
  const stage = (stats.executionStages && stats.executionStages.stage) || winning.stage || '—'
  const fmt = (v) => (v === undefined || v === null ? '—' : v)
  return {
    stage:        stage,
    nReturned:    fmt(stats.nReturned),
    docsExamined: fmt(stats.totalDocsExamined),
    keysExamined: fmt(stats.totalKeysExamined),
    timeMs:       fmt(stats.executionTimeMillis),
  }
})

// ── paging range / count ──────────────────────────────
// "<from> to <to>" of the current page, skip-aware; appends "of <N>" only when a
// count has been taken for the still-current filter.
const rangeText = computed(() => {
  const tab = props.activeTab
  const len = tab?.results?.length ?? 0
  if (!len) return '-- to --'
  const skip = tab.skip || 0
  const base = `${skip + 1} to ${skip + len}`
  // Compare in canonical Extended JSON so the stored count (see fetchCount) matches
  // the active filter regardless of shell-syntax/whitespace differences.
  const parsed = parseField(tab.filter || '')
  const curFilter = parsed.ok ? parsed.ejson : null
  if (tab.total != null && curFilter != null && tab.totalFilter === curFilter) {
    return `${base} of ${tab.total.toLocaleString()}`
  }
  return base
})

// Count applies to a find filter; aggregate pipelines have no single filter.
const isCountDisabled = computed(() =>
  props.isAggregate || !props.activeTab || props.activeTab.kind !== 'collection'
)

// ── query code ─────────────────────────────────────────
const queryCode = computed(() => {
  const tab = props.activeTab
  if (!tab || tab.kind !== 'collection') return ''
  if (tab.mode === 'aggregate') {
    return `db.${tab.collectionName}.aggregate(${tab.pipeline?.trim() || '[]'})`
  }
  const filter = tab.filter?.trim() || '{}'
  const projection = tab.projection?.trim() || ''
  const sort = tab.sort?.trim() || ''
  const skip = tab.skip || 0
  const limit = tab.limit || 50
  let cmd = `db.${tab.collectionName}.find(${filter}`
  if (projection) cmd += `, ${projection}`
  cmd += ')'
  if (sort) cmd += `.sort(${sort})`
  if (skip) cmd += `.skip(${skip})`
  cmd += `.limit(${limit})`
  return cmd
})
</script>

<template>
  <div class="results">
    <div class="result-content">
    <!-- Result sub-tabs -->
    <div class="rtabs">
      <button
        v-for="t in ['Result', 'Query Code', 'Explain']"
        :key="t"
        class="rtab"
        :class="{ active: rtab === t }"
        @click="emit('select-rtab', t)"
      >{{ t }}</button>
    </div>

    <!-- Result toolbar -->
    <div class="rtoolbar" v-if="rtab === 'Result'">
      <button class="icon-btn" @click="emit('run')" :disabled="activeTab.isRunning || !runValid">
        <BaseIcon name="refresh" :size="16" />
      </button>
      <button v-if="activeTab.isRunning" class="cancel-btn" @click="emit('cancel')" title="Cancel the running query">
        <BaseIcon name="close" :size="13" /> Cancel
      </button>
      <button class="icon-btn"
        :disabled="isAggregate || !activeTab.hasRun || (activeTab.skip || 0) === 0 || activeTab.isRunning"
        @click="goFirst"><BaseIcon name="first" :size="16" /></button>
      <button class="icon-btn"
        :disabled="isAggregate || !activeTab.hasRun || (activeTab.skip || 0) === 0 || activeTab.isRunning"
        @click="goPrev"><BaseIcon name="prev" :size="16" /></button>
      <button class="icon-btn"
        :disabled="isAggregate || !activeTab.hasRun || (activeTab.results?.length ?? 0) < (activeTab.limit || 50) || activeTab.isRunning"
        @click="goNext"><BaseIcon name="next" :size="16" /></button>
      <button class="icon-btn"
        :disabled="isAggregate || !activeTab.hasRun || (activeTab.results?.length ?? 0) < (activeTab.limit || 50) || activeTab.isRunning"
        @click="goLast"><BaseIcon name="last" :size="16" /></button>
      <div class="page-size-wrap">
        <span class="page-size" @click="pageSizeMenu = !pageSizeMenu">
          {{ activeTab.limit || 50 }} <BaseIcon name="caretDown" :size="12" />
        </span>
        <div v-if="pageSizeMenu" class="page-size-menu">
          <div
            v-for="sz in PAGE_SIZES"
            :key="sz"
            class="psm-item"
            :class="{ on: (activeTab.limit || 50) === sz }"
            @click="setPageSize(sz)"
          >{{ sz }}</div>
        </div>
      </div>
      <span class="docs-range">
        Documents {{ rangeText }}
      </span>
      <button class="icon-btn" disabled><BaseIcon name="lock" :size="16" /></button>
      <button class="icon-btn"
        :disabled="!activeTab.hasRun || activeTab.isRunning"
        @click="openInsert"><BaseIcon name="plus" :size="16" /></button>
      <button class="icon-btn"
        :disabled="activeTab.selectedRow < 0"
        @click="copySelectedDocument"><BaseIcon name="copy" :size="16" /></button>
      <button class="icon-btn"
        :disabled="activeTab.selectedRow < 0"
        @click="openEdit"><BaseIcon name="edit" :size="16" /></button>
      <button class="icon-btn"
        :disabled="activeTab.selectedRow < 0"
        @click="showDeleteConfirm = true; crudError = null"><BaseIcon name="trash" :size="16" />
      </button>
      <span class="rtoolbar-spacer"></span>

      <!-- View mode selector -->
      <div class="view-select-wrap">
        <span class="view-select" @click="viewMenu = !viewMenu">
          {{ { table: 'Table View', json: 'JSON View', tree: 'Tree View' }[viewMode] }}
          <BaseIcon name="caretDown" :size="12" />
        </span>
        <div v-if="viewMenu" class="view-menu">
          <div
            v-for="[k, label] in [['table','Table View'],['json','JSON View'],['tree','Tree View']]"
            :key="k"
            class="view-menu-item"
            :class="{ on: viewMode === k }"
            @click="viewMode = k; viewMenu = false"
          >
            <BaseIcon v-if="viewMode === k" name="check" :size="13" />
            <span>{{ label }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Result-tab states: error / loading / empty (shared placeholder) -->
    <StateMessage
      v-if="rtab === 'Result' && activeTab.runError"
      mode="error"
      :message="activeTab.runError"
      :code="activeTab.runErrorCode"
      retryable
      @retry="emit('run')"
    />
    <StateMessage
      v-else-if="rtab === 'Result' && activeTab.isRunning"
      mode="loading"
      label="Running query…"
    />
    <StateMessage
      v-else-if="rtab === 'Result' && activeTab.hasRun && !activeTab.results?.length"
      mode="empty"
    />

    <!-- Table view -->
    <ResultTable
      v-else-if="rtab === 'Result' && viewMode === 'table'"
      :active-tab="activeTab"
      :vqb-open="vqbOpen"
      v-model:drillPath="drillPath"
      @dragged-field="draggedField = $event"
      @drag-over-section="dragOverSection = $event"
      @vqb-drop="vqbDrop = $event"
      @open-vqb="emit('open-vqb')"
      @close-vqb="emit('close-vqb')"
      @crud-error="crudError = $event"
    />

    <!-- JSON view -->
    <div v-else-if="rtab === 'Result' && viewMode === 'json'" class="json-view">
      <div v-if="!activeTab.results?.length" style="padding:32px;color:var(--text-faint);font-size:12px">No documents</div>
      <div v-else class="json-doc" v-for="(doc, i) in activeTab.results" :key="i" v-html="syntaxHighlight(mongoStringify(doc))"></div>
    </div>

    <!-- Tree view -->
    <div v-else-if="rtab === 'Result' && viewMode === 'tree'" class="tree-view">
      <div v-if="!activeTab.results?.length" style="padding:32px;color:var(--text-faint);font-size:12px">No documents</div>
      <template v-else>
        <div class="tree-head">
          <span class="th-key">Key</span>
          <span class="th-val">Value</span>
          <span class="th-type">Type</span>
        </div>
        <div class="tree-body">
          <TreeView
            v-for="(doc, i) in activeTab.results"
            :key="i"
            :label="`(${i + 1})`"
            :value="doc"
            :depth="0"
          />
        </div>
      </template>
    </div>

    <!-- Query Code sub-tab -->
    <div v-else-if="rtab === 'Query Code'" class="qcode-view">
      <pre class="qcode-pre"><span class="qcode-prompt">&gt;</span> {{ queryCode }}</pre>
    </div>

    <!-- Explain sub-tab -->
    <div v-else-if="rtab === 'Explain'" class="explain-view">
      <div v-if="activeTab.explainRunning" class="explain-msg">Running explain…</div>
      <div v-else-if="activeTab.explainError" class="run-error">{{ activeTab.explainError }}</div>
      <template v-else-if="activeTab.explainResult">
        <div class="explain-summary" v-if="explainSummary">
          <span class="es-item"><span class="es-k">Stage</span><span class="es-v">{{ explainSummary.stage }}</span></span>
          <span class="es-item"><span class="es-k">Returned</span><span class="es-v">{{ explainSummary.nReturned }}</span></span>
          <span class="es-item"><span class="es-k">Docs examined</span><span class="es-v">{{ explainSummary.docsExamined }}</span></span>
          <span class="es-item"><span class="es-k">Keys examined</span><span class="es-v">{{ explainSummary.keysExamined }}</span></span>
          <span class="es-item"><span class="es-k">Time</span><span class="es-v">{{ explainSummary.timeMs }} ms</span></span>
        </div>
        <div class="json-doc" v-html="syntaxHighlight(mongoStringify(activeTab.explainResult))"></div>
      </template>
      <div v-else class="explain-msg">Run a query, then this tab shows its execution plan.</div>
    </div>

    <!-- Other sub-tabs placeholder -->
    <div v-else class="empty-rows" style="padding:32px;color:var(--text-faint);font-size:12px;display:flex;align-items:center;justify-content:center">
      {{ rtab }} — coming soon
    </div>

    <!-- Footer -->
    <div class="rfooter">
      <span>{{ activeTab.selectedRow >= 0 ? '1 document selected' : '0 documents selected' }}</span>
      <span class="spacer"></span>
      <span class="fitem"
        :class="{ clickable: !isCountDisabled, faded: isCountDisabled }"
        @click="countDocuments"><BaseIcon name="count" :size="14" /> Count Documents</span>
      <span class="fitem" v-if="activeTab.elapsedMs != null">
        <BaseIcon name="clock" :size="14" />
        {{ (activeTab.elapsedMs / 1000).toFixed(3) }}s
      </span>
    </div>
    </div><!-- /result-content -->
    <div
      v-if="vqbOpen"
      class="resizer"
      :class="{ dragging: vqbResizing }"
      title="Drag to resize"
      @mousedown="startVqbResize"
    >
      <span class="resizer-grip"></span>
    </div>
    <VisualQueryBuilder
      v-if="vqbOpen"
      :tabs="tabs"
      :active-tab-id="activeTabId"
      :width="vqbWidth"
      :dragged-field="draggedField"
      :vqb-drop="vqbDrop"
      :drag-over-section="dragOverSection"
      @run="emit('run')"
    />
  </div>

  <!-- Document insert / edit modal -->
  <DocumentModal
    v-if="showDocModal"
    :mode="docModalMode"
    :initial-doc="docModalMode === 'edit' ? activeTab?.results[activeTab.selectedRow] : null"
    @close="showDocModal = false"
    @save="onDocSave"
  />

  <!-- Delete confirmation -->
  <div v-if="showDeleteConfirm" class="del-overlay" @mousedown.self="showDeleteConfirm = false">
    <div class="del-dialog">
      <div class="del-title">
        <div class="t">Delete Document</div>
        <button class="close-btn" @click="showDeleteConfirm = false">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>
      <div class="del-body">
        <p>Are you sure you want to delete this document? This cannot be undone.</p>
        <div v-if="crudError" class="del-error">{{ crudError }}</div>
      </div>
      <div class="del-footer">
        <span class="spacer"></span>
        <button class="btn" @click="showDeleteConfirm = false">Cancel</button>
        <button class="btn danger" @click="onDeleteConfirm">Delete</button>
      </div>
    </div>
  </div>

  <!-- CRUD error banner (for edit/insert errors shown outside the modal) -->
  <div v-if="crudError && !showDocModal && !showDeleteConfirm" class="crud-err-banner">
    {{ crudError }}
    <button @click="crudError = null"><BaseIcon name="close" :size="13" /></button>
  </div>
</template>

<style scoped>
/* Results */
.results { flex: 1; display: flex; flex-direction: row; min-height: 0; }

/* ── VQB resizer ── (matches the connection sidebar resizer in App.vue) */
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
.result-content { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden; }
.rtabs { display: flex; align-items: stretch; border-bottom: 1px solid var(--border); flex: none; }
.rtab {
  padding: 6px 16px;
  font-size: 12.5px;
  color: var(--text-dim);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
}
.rtab.active { color: var(--text); border-bottom-color: var(--accent); }

.rtoolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.icon-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-dim);
  padding: 4px;
  display: grid;
  place-items: center;
}
.icon-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
.icon-btn:disabled { opacity: .4; }
.cancel-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 10px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--bg-toolbar);
  color: var(--danger-text);
  font-size: 12px;
  cursor: pointer;
}
.cancel-btn:hover { background: var(--bg-hover); }
.page-size {
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  padding: 3px 6px;
  font-size: 12px;
  color: var(--text);
}
.docs-range { font-size: 12px; color: var(--text-dim); margin: 0 4px; }
.rtoolbar-spacer { flex: 1; }

/* view mode */
.view-select-wrap { position: relative; }
.view-select {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  padding: 4px 9px;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
}
.view-menu {
  position: absolute;
  right: 0;
  top: 30px;
  width: 160px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 14px 34px rgba(0,0,0,.55);
  z-index: 20;
  padding: 4px;
}
.view-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: 5px;
  font-size: 12.5px;
  color: var(--text-dim);
  cursor: pointer;
}
.view-menu-item:hover { background: var(--bg-hover); color: var(--text); }
.view-menu-item.on { color: var(--text); }
.view-menu-item:not(.on) span { margin-left: 21px; }

/* Striped placeholder background, shared by the "coming soon" sub-tab. */
.empty-rows {
  min-height: 2000px;
  position: relative;
  background:
    repeating-linear-gradient(to bottom, transparent 0 24px, var(--grid-line) 24px 25px),
    repeating-linear-gradient(to bottom, var(--bg-row) 0 25px, var(--bg-row-alt) 25px 50px);
}

/* JSON view */
.json-view { flex: 1; overflow: auto; padding: 12px 16px; }

/* Tree view */
.tree-view { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: auto; background: var(--bg-window); }
.tree-head {
  display: grid;
  grid-template-columns: minmax(220px, 1.4fr) minmax(160px, 2fr) 110px;
  position: sticky;
  top: 0;
  z-index: 2;
  height: 26px;
  align-items: center;
  background: var(--bg-toolbar);
  color: var(--text-dim);
  font-weight: 600;
  font-size: 11px;
  border-bottom: 1px solid var(--border);
}
.tree-head span { padding: 0 8px; border-right: 1px solid var(--border); height: 100%; display: flex; align-items: center; }
.tree-head .th-type { border-right: none; }
.json-doc {
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.2;
  color: var(--text);
  white-space: pre;
  border-left: 2px solid var(--border-soft);
  padding: 8px 0 8px 14px;
  margin-bottom: 10px;
  cursor: text;
  -webkit-user-select: text;
  user-select: text;
}
/* syntax highlight token classes */
/* the global `*` reset in theme.css sets user-select:none directly on these
   spans, which overrides the inherited user-select:text from .json-doc —
   re-enable it here or selection/copy only picks up the punctuation between tokens */
.json-doc :deep(span) { -webkit-user-select: text; user-select: text; }
.json-doc :deep(.jk)  { color: var(--cell-key); }
.json-doc :deep(.jop) { color: var(--cell-op); }
.json-doc :deep(.js)  { color: var(--cell-str); }
.json-doc :deep(.jn)  { color: var(--cell-num); }
.json-doc :deep(.jb)  { color: var(--cell-num); }
.json-doc :deep(.jl)  { color: var(--text-faint); }
.json-doc :deep(.joid) { color: var(--link); }

.explain-view { flex: 1; overflow: auto; padding: 12px 16px; }
.explain-msg { padding: 32px; color: var(--text-faint); font-size: 12px; display: flex; align-items: center; justify-content: center; }
.explain-summary { display: flex; flex-wrap: wrap; gap: 8px 18px; padding: 10px 12px; margin-bottom: 12px; background: var(--panel-2, rgba(255,255,255,.03)); border: 1px solid var(--border-soft); border-radius: 6px; }
.es-item { display: flex; flex-direction: column; gap: 2px; }
.es-k { font-size: 10.5px; text-transform: uppercase; letter-spacing: .4px; color: var(--text-faint); }
.es-v { font-family: var(--mono); font-size: 13px; color: var(--text); }

/* page size dropdown */
.page-size-wrap { position: relative; }
.page-size { cursor: pointer; }
.page-size-menu {
  position: absolute;
  top: 28px;
  left: 0;
  width: 70px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 10px 28px rgba(0,0,0,.55);
  z-index: 20;
  padding: 4px;
}
.psm-item {
  padding: 6px 10px;
  border-radius: 5px;
  font-size: 12px;
  color: var(--text-dim);
  cursor: pointer;
  text-align: right;
}
.psm-item:hover { background: var(--bg-hover); color: var(--text); }
.psm-item.on    { color: var(--accent); font-weight: 600; }

/* Query Code sub-tab */
.qcode-view { flex: 1; overflow: auto; padding: 16px 20px; }
.qcode-pre {
  font-family: var(--mono);
  font-size: 13px;
  line-height: 1.7;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-all;
  -webkit-user-select: text;
  user-select: text;
}
.qcode-prompt { color: var(--text-faint); margin-right: 8px; }

/* Footer */
.rfooter {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 6px 12px;
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-dim);
  flex: none;
  background: var(--bg-panel);
}
.spacer { flex: 1; }
.fitem { display: flex; align-items: center; gap: 6px; }
.fitem.clickable { cursor: pointer; }
.fitem.clickable:hover { color: var(--text); }
.fitem.faded { opacity: .4; cursor: default; }
.run-error { padding: 10px 14px; color: var(--danger-text); font-size: 12px; }

/* Delete confirm dialog */
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
.del-footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
}
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
.btn.danger { background: var(--danger); color: #fff; }
.btn.danger:hover { background: var(--danger-hover); }

/* CRUD error banner */
.crud-err-banner {
  position: fixed;
  bottom: 48px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--danger-bg);
  border: 1px solid var(--danger);
  color: var(--danger-text);
  border-radius: 6px;
  padding: 8px 14px;
  font-size: 12.5px;
  display: flex;
  align-items: center;
  gap: 10px;
  z-index: 70;
  max-width: 560px;
}
.crud-err-banner button {
  background: none;
  border: none;
  color: var(--danger-text);
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  flex: none;
}
</style>
