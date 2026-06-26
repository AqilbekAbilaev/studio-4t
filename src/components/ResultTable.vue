<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  activeTab: { type: Object,  required: true },
  vqbOpen:   { type: Boolean, default: false },
  drillPath: { type: Array,   default: () => [] },  // field-name path navigated into
})

// The drag-to-VQB outputs (`vqb-drop`, `dragged-field`, `drag-over-section`) are
// consumed by VisualQueryBuilder, which lives beside this grid in ResultsPanel, so they
// bubble up rather than being held here. `update:drillPath` keeps drill state (owned by
// ResultsPanel so it survives view switches and the run-reset) in sync via v-model.
const emit = defineEmits(['dragged-field', 'drag-over-section', 'vqb-drop', 'crud-error', 'update:drillPath'])

function onThClick(col) {
  if (!props.vqbOpen) return
  emit('dragged-field', col)
  nextTick(() => { emit('dragged-field', '') })
}

// ── drag a result cell → Visual Query Builder ──────────────
// HTML5 drag-and-drop doesn't fire drop events reliably inside Tauri's WKWebView,
// so dragging is done with raw mouse events. A drag only starts once the pointer
// moves past a small threshold, so a plain click still selects the cell. On drop
// we hit-test the pointer against the VQB sections (tagged with data-vqb-drop)
// and hand the field + section to VisualQueryBuilder via the vqbDrop prop.
const DRAG_THRESHOLD  = 5
const dragging        = ref(false)
const dragGhost       = ref({ x: 0, y: 0, label: '' })

let dragField         = ''
let dragStartX        = 0
let dragStartY        = 0
let suppressNextClick = false

function sectionAtPoint(x, y) {
  const el = document.elementFromPoint(x, y)
  const zone = el && el.closest('[data-vqb-drop]')
  return zone ? zone.getAttribute('data-vqb-drop') : null
}

function onCellMouseDown(e, col) {
  if (!props.vqbOpen || e.button !== 0) return
  if (e.target.tagName === 'INPUT') return  // inline cell editor is active
  // Suppress the browser's native press-drag selection gesture, which otherwise
  // auto-scrolls the grid sideways as the pointer moves toward the VQB panel.
  // Click and dblclick still fire, so cell selection / editing is unaffected.
  e.preventDefault()
  suppressNextClick = false
  dragField  = col
  dragStartX = e.clientX
  dragStartY = e.clientY
  dragging.value = false
  document.addEventListener('mousemove', onDragMove)
  document.addEventListener('mouseup',   onDragUp)
}

function onDragMove(e) {
  if (!dragging.value) {
    if (Math.hypot(e.clientX - dragStartX, e.clientY - dragStartY) < DRAG_THRESHOLD) return
    dragging.value = true
    document.body.style.cursor = 'grabbing'
  }
  dragGhost.value = { x: e.clientX, y: e.clientY, label: dragField }
  emit('drag-over-section', sectionAtPoint(e.clientX, e.clientY))
}

function onDragUp(e) {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup',   onDragUp)
  document.body.style.cursor = ''
  if (dragging.value) {
    const section = sectionAtPoint(e.clientX, e.clientY)
    if (section) emit('vqb-drop', { field: dragField, section: section, nonce: Date.now() })
    suppressNextClick = true  // swallow the click that fires after a real drag
  }
  dragging.value = false
  emit('drag-over-section', null)
  dragField = ''
}

function onCellClick(rowIdx, col) {
  if (suppressNextClick) { suppressNextClick = false; return }
  selectCell(rowIdx, col)
}

onUnmounted(() => {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup',   onDragUp)
})

// ── table view helpers ─────────────────────────────────────
function guessType(key, val) {
  if (key === '_id' || (val && typeof val === 'object' && '$oid' in val)) return 'id'
  if (val && typeof val === 'object' && '$date' in val) return 'date'
  if (typeof val === 'number') return 'num'
  if (typeof val === 'boolean') return 'bool'
  if (val === null || val === undefined) return 'null'
  if (Array.isArray(val) || (typeof val === 'object')) return 'obj'
  return 'str'
}

const TYPE_CLASS = { id: 'cell-oid', str: 'cell-str', num: 'cell-num', date: '', bool: 'cell-num', null: 'cell-faint', obj: 'cell-faint' }

function formatCell(key, val) {
  if (val === null || val === undefined) return ''
  if (typeof val === 'string') return val
  if (typeof val === 'number' || typeof val === 'boolean') return String(val)
  if (Array.isArray(val)) return `Array(${val.length})`
  if (typeof val === 'object') {
    if ('$oid' in val) return val.$oid
    if ('$date' in val) {
      const d = val.$date
      if (typeof d === 'string') return d
      if (typeof d === 'object' && '$numberLong' in d) return new Date(parseInt(d.$numberLong)).toISOString()
    }
    if ('$numberLong' in val) return val.$numberLong
    if ('$numberDecimal' in val) return val.$numberDecimal
    return '{…}'
  }
  return JSON.stringify(val)
}

function columns(results) {
  if (!results?.length) return []
  const seen = new Set()
  for (const doc of results) for (const k of Object.keys(doc)) seen.add(k)
  const allNumeric = [...seen].every(k => /^\d+$/.test(k))
  if (allNumeric) return [...seen].sort((a, b) => Number(a) - Number(b))
  const rest = [...seen].filter(k => k !== '_id').sort()
  return seen.has('_id') ? ['_id', ...rest] : rest
}

// Filler rows pad the grid below real documents so the row stripes/borders
// reach the bottom of the viewport instead of stopping after a fixed count —
// recomputed from the actual container height so it still covers tall windows.
const gridWrapRef  = ref(null)
const FILLER_ROW_HEIGHT = 25
const minFillRows  = ref(24)
let gridResizeObserver = null

function updateMinFillRows() {
  if (!gridWrapRef.value) return
  minFillRows.value = Math.max(24, Math.ceil(gridWrapRef.value.clientHeight / FILLER_ROW_HEIGHT))
}

watch(gridWrapRef, (el, prevEl) => {
  if (prevEl) gridResizeObserver?.unobserve(prevEl)
  if (el) {
    if (!gridResizeObserver) gridResizeObserver = new ResizeObserver(updateMinFillRows)
    gridResizeObserver.observe(el)
    updateMinFillRows()
  }
}, { flush: 'post' })

onUnmounted(() => { gridResizeObserver?.disconnect() })

function fillerCount(results) {
  return Math.max(0, minFillRows.value - (results?.length || 0))
}

// ── column resize ──────────────────────────────────────
const tableRef   = ref(null)
const colWidths  = ref({})   // col name → px; empty = auto layout

let resizeCol = null
let resizeStartX = 0
let resizeStartWidth = 0

function startResize(e, col) {
  e.preventDefault()
  e.stopPropagation()
  // Measure only the column being dragged so we never snap all columns at once
  const cols     = gridColumns.value
  const nthChild = cols.indexOf(col) + 2
  const th       = tableRef.value?.querySelector(`thead th:nth-child(${nthChild})`)
  resizeCol        = col
  resizeStartX     = e.clientX
  resizeStartWidth = th ? th.offsetWidth : (colWidths.value[col] || 80)
  document.body.style.cursor     = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup',   stopResize)
}

function onResizeMove(e) {
  if (resizeCol === null) return
  colWidths.value[resizeCol] = Math.max(40, resizeStartWidth + (e.clientX - resizeStartX))
  // WebKit caches a sticky header cell's geometry and won't recompute its pinned
  // box just because its width changed — the line lags until something else
  // forces layout. Nudge a reflow once the new width has been applied to the DOM,
  // without touching `position`, so the header never jumps from its pinned spot.
  nextTick(() => { if (tableRef.value) void tableRef.value.offsetHeight })
}

function stopResize() {
  resizeCol = null
  document.body.style.cursor     = ''
  document.body.style.userSelect = ''
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup',   stopResize)
}

function autoFitColumn(e, col) {
  e.stopPropagation()
  if (!tableRef.value) return

  const cols = gridColumns.value
  // +2: child 1 is the rownum column, data columns start at child 2
  const nthChild = cols.indexOf(col) + 2
  if (nthChild < 2) return

  // Header: measure label text with a throwaway element that inherits the th's computed font.
  // Can't use th.scrollWidth — in fixed layout it equals offsetWidth when cell > content.
  const th = tableRef.value.querySelector(`thead th:nth-child(${nthChild})`)
  let maxW = 40
  if (th) {
    const probe = document.createElement('span')
    probe.style.cssText = `position:absolute;visibility:hidden;white-space:nowrap;font:${getComputedStyle(th).font}`
    probe.textContent = col === '_id' ? '{Document id}' : col
    document.body.appendChild(probe)
    maxW = probe.offsetWidth + 24  // 12px left + 12px right padding from th CSS
    document.body.removeChild(probe)
  }

  // Body cells: .tcell is display:inline-flex so its offsetWidth = intrinsic content size,
  // independent of how wide or narrow the parent td currently is.
  tableRef.value.querySelectorAll(`tbody tr:not(.filler) td:nth-child(${nthChild}) .tcell`).forEach(tcell => {
    maxW = Math.max(maxW, tcell.offsetWidth + 24)  // 12px left + 12px right padding from td CSS
  })

  colWidths.value[col] = Math.ceil(maxW)
}

// ── row / cell selection ──────────────────────────────
const selectedCol = ref(null)  // tracked only for right-click context menu copy
const cellCtx     = ref(null)  // { x, y, row, col } | null — right-click menu
const inlineEdit  = ref(null)  // { rowIdx, col, raw } | null — in-place primitive edit

const vFocus = { mounted(el) { el.focus(); el.select() } }

// Reset transient selection / widths when switching tabs so we re-measure on the new
// results. (Drill path is owned by ResultsPanel and reset there.)
watch(() => props.activeTab?.id, () => {
  selectedCol.value = null
  cellCtx.value = null
  colWidths.value = {}
})

function getAtPath(doc, path) {
  let cur = doc
  for (const key of path) {
    if (cur === null || typeof cur !== 'object') return undefined
    cur = cur[key]
  }
  return cur
}

// the "documents" the grid currently renders: either the real result set, or
// (once drilled) every document's value at the drilled path — one row per
// original document is kept, so documents missing that path just render blank
// instead of collapsing the grid down to a single row
// Cached once per render. The template reads these many times (the column list is
// referenced once per row), so computing them as plain functions made rendering a
// 200-document result O(rows²); memoizing keeps the draw fast.
const gridDocs = computed(() => {
  const tab = props.activeTab
  if (!tab) return []
  if (!props.drillPath.length) return tab.results || []
  return (tab.results || []).map((doc) => {
    const val = getAtPath(doc, props.drillPath) ?? {}
    if (Array.isArray(val)) {
      const obj = {}
      val.forEach((el, idx) => { obj[String(idx)] = el })
      return obj
    }
    return val
  })
})

const gridColumns = computed(() => columns(gridDocs.value))

function isDrillable(col, val) {
  return guessType(col, val) === 'obj'
}

function openCellDrill(rowIdx, col) {
  const tab = props.activeTab
  if (!tab) return
  const val = gridDocs.value[rowIdx]?.[col]
  if (!isDrillable(col, val)) return
  emit('update:drillPath', [...props.drillPath, col])
  selectedCol.value = null
  tab.selectedRow = -1
}

function goToDrillLevel(level) {
  emit('update:drillPath', level < 0 ? [] : props.drillPath.slice(0, level + 1))
  selectedCol.value = null
  if (props.activeTab) props.activeTab.selectedRow = -1
}

function selectRow(rowIdx) {
  props.activeTab.selectedRow = rowIdx
  selectedCol.value = null
  cellCtx.value = null
}

function selectCell(rowIdx, col) {
  props.activeTab.selectedRow = rowIdx
  selectedCol.value = col
  cellCtx.value = null
}

function cellCopyValue(col, val) {
  if (val === null || val === undefined) return ''
  if (typeof val === 'string')  return val
  if (typeof val === 'number' || typeof val === 'boolean') return String(val)
  if (typeof val === 'object') {
    if ('$oid'  in val) return val.$oid
    if ('$date' in val) {
      const d = val.$date
      if (typeof d === 'string') return d
      if (typeof d === 'object' && '$numberLong' in d) return new Date(parseInt(d.$numberLong)).toISOString()
    }
    if ('$numberLong'    in val) return val.$numberLong
    if ('$numberDecimal' in val) return val.$numberDecimal
  }
  return JSON.stringify(val, null, 2)
}

function copySelectedCell() {
  const tab = props.activeTab
  if (!tab || tab.selectedRow < 0 || !selectedCol.value) return
  const val = gridDocs.value[tab.selectedRow]?.[selectedCol.value]
  navigator.clipboard.writeText(cellCopyValue(selectedCol.value, val))
}

function copySelectedDocument() {
  const tab = props.activeTab
  if (!tab || tab.selectedRow < 0) return
  navigator.clipboard.writeText(JSON.stringify(tab.results[tab.selectedRow], null, 2))
}

function openCellCtx(e, rowIdx, col) {
  e.preventDefault()
  selectCell(rowIdx, col)
  cellCtx.value = { x: e.clientX, y: e.clientY, row: rowIdx, col: col }
}

function cellCtxPick(action) {
  const docs = gridDocs.value
  const val = docs[cellCtx.value?.row]?.[cellCtx.value?.col]
  if (action === 'copy-value') {
    navigator.clipboard.writeText(cellCopyValue(cellCtx.value.col, val))
  } else if (action === 'copy-json') {
    navigator.clipboard.writeText(JSON.stringify(val, null, 2))
  } else if (action === 'copy-doc') {
    navigator.clipboard.writeText(JSON.stringify(docs[cellCtx.value.row], null, 2))
  }
  cellCtx.value = null
}

// ── inline cell editing ────────────────────────────────
function buildIdFilter(doc) {
  return JSON.stringify({ _id: doc._id })
}

function startInlineEdit(rowIdx, col) {
  const tab = props.activeTab
  if (!tab) return
  const val = gridDocs.value[rowIdx]?.[col]
  const type = guessType(col, val)
  if (type === 'obj' || type === 'id' || type === 'date') return
  const raw = val === null || val === undefined ? '' : String(val)
  inlineEdit.value = { rowIdx: rowIdx, col: col, raw: raw }
}

async function commitInlineEdit() {
  const edit = inlineEdit.value
  if (!edit) return
  inlineEdit.value = null
  const tab = props.activeTab
  if (!tab) return
  const docs = gridDocs.value
  const originalVal = docs[edit.rowIdx]?.[edit.col]
  let newVal
  if (typeof originalVal === 'number') {
    const n = Number(edit.raw)
    newVal = isNaN(n) ? edit.raw : n
  } else if (typeof originalVal === 'boolean') {
    newVal = edit.raw === 'true'
  } else {
    newVal = edit.raw
  }
  const rootDoc = JSON.parse(JSON.stringify(tab.results[edit.rowIdx]))
  let cur = rootDoc
  for (const key of props.drillPath) {
    cur = cur[key]
  }
  cur[edit.col] = newVal
  try {
    await invoke('replace_document', {
      id: tab.connectionId,
      database: tab.dbName,
      collection: tab.collectionName,
      idFilter: buildIdFilter(tab.results[edit.rowIdx]),
      document: JSON.stringify(rootDoc),
    })
    const refreshed = await invoke('find_documents', {
      id: tab.connectionId,
      database: tab.dbName,
      collection: tab.collectionName,
      filter: buildIdFilter(tab.results[edit.rowIdx]),
      projection: '{}',
      sort: '{}',
      skip: 0,
      limit: 1,
    })
    if (refreshed.length) {
      tab.results.splice(edit.rowIdx, 1, refreshed[0])
    } else {
      tab.results.splice(edit.rowIdx, 1)
    }
  } catch (e) {
    emit('crud-error', String(e))
  }
}

function cancelInlineEdit() {
  inlineEdit.value = null
}

function handleKeydown(e) {
  // Don't hijack keys while the user is typing in a field (query bar, modals,
  // inline cell editor) — otherwise arrow keys / Ctrl+C drive grid navigation
  // instead of the input.
  const t = e.target
  if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return
  if (inlineEdit.value) return
  const tab = props.activeTab
  if (!tab) return

  if (e.key === 'Escape' && cellCtx.value) { cellCtx.value = null; return }

  if (tab.selectedRow < 0) return

  const docs   = gridDocs.value
  const cols   = gridColumns.value
  const colIdx = cols.indexOf(selectedCol.value)
  const rowIdx = tab.selectedRow

  if ((e.metaKey || e.ctrlKey) && e.key === 'c') {
    e.preventDefault()
    selectedCol.value ? copySelectedCell() : copySelectedDocument()
    return
  }

  if (e.key === 'Escape') {
    selectedCol.value = null
    tab.selectedRow   = -1
    return
  }

  const scrollToCell = () => nextTick(() =>
    tableRef.value?.querySelector('td.selcell')?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  )

  if (e.key === 'ArrowRight' && colIdx < cols.length - 1) {
    e.preventDefault()
    selectedCol.value = cols[colIdx + 1]
    scrollToCell()
  } else if (e.key === 'ArrowLeft' && colIdx > 0) {
    e.preventDefault()
    selectedCol.value = cols[colIdx - 1]
    scrollToCell()
  } else if (e.key === 'ArrowDown' && rowIdx < docs.length - 1) {
    e.preventDefault()
    tab.selectedRow = rowIdx + 1
    scrollToCell()
  } else if (e.key === 'ArrowUp' && rowIdx > 0) {
    e.preventDefault()
    tab.selectedRow = rowIdx - 1
    scrollToCell()
  }
}

onMounted(()  => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleKeydown))

// WebKitGTK (the Linux Tauri webview) lets the grid's compositor layer go "cold"
// while the window is backgrounded, so after switching back it won't repaint on
// interaction until something forces an invalidation — the first scroll is absorbed
// (row-number column flashes blank, snaps back) and rows don't highlight on hover
// until you scroll/click once. Nudge the scroller by a pixel and back on focus
// (with a forced reflow in between) to warm the layer before the user interacts.
// The net scroll position is unchanged.
function repaintGridOnFocus() {
  const el = gridWrapRef.value
  if (!el) return
  requestAnimationFrame(() => {
    const top = el.scrollTop
    el.scrollTop = top + 1
    void el.offsetHeight
    el.scrollTop = top
  })
}
onMounted(()  => window.addEventListener('focus', repaintGridOnFocus))
onUnmounted(() => window.removeEventListener('focus', repaintGridOnFocus))
</script>

<template>
  <div class="grid-outer">
    <div class="fieldpath">
      <span class="fp fp-link" @click="goToDrillLevel(-1)">{{ activeTab.collectionName }}</span>
      <template v-for="(seg, idx) in drillPath" :key="idx">
        <BaseIcon name="caret" :size="11" class="fp-sep" />
        <span class="fp fp-link" @click="goToDrillLevel(idx)">{{ seg }}</span>
      </template>
      <template v-if="selectedCol">
        <BaseIcon name="caret" :size="11" class="fp-sep" />
        <span class="fp">{{ selectedCol }}</span>
      </template>
    </div>
    <div class="grid-wrap" ref="gridWrapRef">
    <div class="grid-scroll">
    <template v-if="!activeTab.hasRun || activeTab.isRunning">
      <table class="grid">
        <thead><tr>
          <th class="rownum"></th>
          <th style="min-width:320px;max-width:320px">{Document id}</th>
          <th class="col-filler"></th>
        </tr></thead>
      </table>
      <div class="empty-rows"><div class="empty-rows-gutter"></div></div>
    </template>
    <template v-else-if="activeTab.results?.length === 0">
      <table class="grid">
        <thead><tr>
          <th class="rownum"></th>
          <th style="min-width:320px;max-width:320px">{Document id}</th>
          <th class="col-filler"></th>
        </tr></thead>
      </table>
      <div class="empty-rows"><div class="empty-rows-gutter"></div></div>
    </template>
    <template v-else>
      <table
        class="grid"
        ref="tableRef"
      >
        <thead>
          <tr>
            <th class="rownum"></th>
            <th
              v-for="col in gridColumns"
              :key="col"
              :style="colWidths[col] ? { minWidth: colWidths[col] + 'px', maxWidth: colWidths[col] + 'px' } : {}"
              @click.stop="onThClick(col)"
            >
              {{ col === '_id' ? '{Document id}' : (/^\d+$/.test(col) ? `[${col}]` : col) }}
              <div class="col-resize-handle" draggable="false" @dragstart.prevent @mousedown="startResize($event, col)" @dblclick.stop="autoFitColumn($event, col)"></div>
            </th>
            <th class="col-filler"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in gridDocs"
            :key="i"
            :class="{ selrow: activeTab.selectedRow === i }"
            @click="selectRow(i)"
          >
            <td class="rownum">{{ i + 1 }}</td>
            <td
              v-for="col in gridColumns"
              :key="col"
              :class="{ selcell: activeTab.selectedRow === i && selectedCol === col, drillable: isDrillable(col, row[col]) }"
              @mousedown="onCellMouseDown($event, col)"
              @click.stop="onCellClick(i, col)"
              @dblclick.stop="isDrillable(col, row[col]) ? openCellDrill(i, col) : startInlineEdit(i, col)"
              @contextmenu="openCellCtx($event, i, col)"
            >
              <template v-if="inlineEdit && inlineEdit.rowIdx === i && inlineEdit.col === col">
                <input
                  class="cell-edit-input"
                  v-model="inlineEdit.raw"
                  v-focus
                  @keydown.enter.stop="commitInlineEdit"
                  @keydown.escape.stop="cancelInlineEdit"
                  @blur="commitInlineEdit"
                />
              </template>
              <span v-else class="tcell" :class="'t-' + guessType(col, row[col])">
                <span class="tval" :class="TYPE_CLASS[guessType(col, row[col])]">
                  {{ formatCell(col, row[col]) }}
                </span>
              </span>
            </td>
            <td class="col-filler"></td>
          </tr>
          <tr
            v-for="f in fillerCount(gridDocs)"
            :key="'f' + f"
            class="filler"
          >
            <td class="rownum"></td>
            <td v-for="col in gridColumns" :key="col"></td>
            <td class="col-filler"></td>
          </tr>
        </tbody>
      </table>
    </template>
    </div>
    </div>
  </div>

  <!-- Cell right-click context menu -->
  <template v-if="cellCtx">
    <div class="cell-ctx-backdrop" @mousedown="cellCtx = null"></div>
    <div class="cell-ctx-menu" :style="{ left: cellCtx.x + 'px', top: cellCtx.y + 'px' }">
      <div class="cell-ctx-item" @click="cellCtxPick('copy-value')">
        <span class="cell-ctx-ic"><BaseIcon name="copy" :size="14" /></span>
        Copy Value
        <span class="cell-ctx-sc">⌘C</span>
      </div>
      <div class="cell-ctx-item" @click="cellCtxPick('copy-json')">
        <span class="cell-ctx-ic"></span>
        Copy as JSON
      </div>
      <div class="cell-ctx-sep"></div>
      <div class="cell-ctx-item" @click="cellCtxPick('copy-doc')">
        <span class="cell-ctx-ic"></span>
        Copy Document
      </div>
    </div>
  </template>

  <!-- Floating label that follows the pointer while dragging a cell into the VQB -->
  <div
    v-if="dragging"
    class="drag-ghost"
    :style="{ left: dragGhost.x + 14 + 'px', top: dragGhost.y + 14 + 'px' }"
  >{{ dragGhost.label }}</div>
</template>

<style scoped>
/* Grid */
.grid-outer { flex: 1; display: flex; flex-direction: column; min-height: 0; background: var(--bg-window); }
.grid-wrap { flex: 1; overflow: auto; min-height: 0; }
.grid-scroll { width: max-content; min-width: 100%; }
.fieldpath {
  height: 34px;
  flex: none;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 12px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
}
.fieldpath .fp-sep { color: var(--text-faint); }
.fieldpath .fp-link { cursor: pointer; }
.fieldpath .fp-link:hover { color: var(--accent); }
table.grid {
  border-collapse: collapse;
  width: max-content;
  min-width: 100%;
  font-family: var(--mono);
  font-size: 12px;
}
table.grid th {
  position: sticky;
  top: 0;
  background: var(--bg-toolbar);
  color: var(--text-dim);
  font-weight: 600;
  text-align: left;
  padding: 5px 12px;
  border-right: 1px solid var(--grid-line);
  border-bottom: 1px solid var(--border-soft);
  white-space: nowrap;
  z-index: 2;
}
table.grid td {
  padding: 4px 12px;
  border-right: 1px solid var(--grid-line);
  border-bottom: 1px solid var(--grid-line);
  color: var(--text);
  white-space: nowrap;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
}
table.grid tr:nth-child(even) td { background: var(--bg-row-alt); }
table.grid tr:hover td { background: var(--bg-hover); }
table.grid tr.selrow td { background: #34373c; box-shadow: inset 0 0 0 9999px rgba(255,255,255,.02); }
table.grid td.selcell { outline: 1px solid var(--accent); outline-offset: -1px; position: relative; z-index: 4; }
table.grid td.drillable { cursor: pointer; }
/* rownum — sticky left gutter column */
table.grid th.rownum,
table.grid td.rownum {
  position: sticky;
  left: 0;
  z-index: 1;
  background: var(--bg-panel-2);
  color: var(--text-faint);
  text-align: right;
  min-width: 46px;
  border-right: 1px solid var(--border-soft);
}
table.grid th.rownum { z-index: 3; }
table.grid tr:hover td.rownum { background: var(--bg-hover); }
table.grid tr.selrow td.rownum { background: #2e3033; }
/* filler rows extend the column grid below real documents */
table.grid tr.filler td { height: 25px; padding: 0; }
table.grid tr.filler:nth-child(even) td { background: var(--bg-row-alt); }
th.col-filler, td.col-filler { border-right: none; width: 100%; }

/* Drag ghost — follows the pointer while dragging a cell into the VQB.
   pointer-events:none is required so elementFromPoint sees the dropzone, not the ghost. */
.drag-ghost {
  position: fixed;
  z-index: 200;
  pointer-events: none;
  background: var(--accent);
  color: #fff;
  font-family: var(--mono);
  font-size: 12px;
  padding: 4px 9px;
  border-radius: 6px;
  box-shadow: 0 6px 18px rgba(0, 0, 0, .45);
  white-space: nowrap;
}

/* Cell context menu */
.cell-ctx-backdrop { position: fixed; inset: 0; z-index: 80; }
.cell-ctx-menu {
  position: fixed;
  z-index: 81;
  min-width: 190px;
  background: #2b2d31;
  border: 1px solid #16171a;
  border-radius: 8px;
  box-shadow: 0 18px 48px rgba(0,0,0,.6);
  padding: 5px;
}
.cell-ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px 6px 8px;
  border-radius: 5px;
  font-size: 13px;
  color: var(--text);
  cursor: default;
}
.cell-ctx-item:hover { background: var(--accent); color: #fff; }
.cell-ctx-ic { width: 18px; flex: none; display: grid; place-items: center; color: var(--text-dim); }
.cell-ctx-item:hover .cell-ctx-ic,
.cell-ctx-item:hover .cell-ctx-sc { color: rgba(255,255,255,.75); }
.cell-ctx-sc { margin-left: auto; color: var(--text-faint); font-size: 12px; letter-spacing: .5px; }
.cell-ctx-sep { height: 1px; background: #3a3c41; margin: 5px 8px; }

.col-resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  width: 12px;
  height: 100%;
  cursor: col-resize;
  z-index: 1;
  transform: translateX(50%);
}

.tcell { display: inline-flex; align-items: center; gap: 6px; vertical-align: middle; }
.cell-oid   { color: var(--link); }
.cell-str   { color: var(--cell-str-green); }
.cell-num   { color: var(--cell-num); }
.cell-faint { color: var(--text-faint); }

.empty-rows {
  min-height: 2000px;
  position: relative;
  background:
    repeating-linear-gradient(to bottom, transparent 0 24px, var(--grid-line) 24px 25px),
    repeating-linear-gradient(to bottom, var(--bg-row) 0 25px, var(--bg-row-alt) 25px 50px);
}
.empty-rows-gutter {
  position: absolute;
  left: 0; top: 0; bottom: 0;
  width: 46px;
  /* Alternate per 25px row like the populated grid's row-number gutter, so the empty
     state doesn't read as one solid near-black bar. */
  background: repeating-linear-gradient(
    to bottom,
    var(--bg-panel-2) 0 25px,
    var(--bg-row-alt) 25px 50px
  );
  border-right: 1px solid var(--border-soft);
}

/* Inline cell editor */
.cell-edit-input {
  width: 100%;
  background: transparent;
  border: none;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12px;
  padding: 0;
  outline: none;
}
</style>
