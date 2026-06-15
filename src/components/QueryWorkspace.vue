<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import BaseIcon from './BaseIcon.vue'
import DocumentModal from './DocumentModal.vue'

const props = defineProps({
  tabs:        { type: Array,   required: true },
  activeTabId: { type: String,  required: true },
  vqbOpen:     { type: Boolean, default: false },
})
const emit = defineEmits(['activate-tab', 'close-tab', 'run-query', 'toggle-vqb'])

const activeTab = computed(() => props.tabs.find(t => t.id === props.activeTabId))

// per-tab local state for result sub-tab and view mode
const rtab         = ref('Result')
const viewMode     = ref('table')
const viewMenu     = ref(false)
const pageSizeMenu = ref(false)

// ── query helpers ──────────────────────────────────────────
function toStrictJson(raw) {
  const s = (raw || '').trim()
  if (!s || s === '{}') return '{}'
  return s.replace(/([{,]\s*)([a-zA-Z_$][a-zA-Z0-9_$.]*)\s*:/g, '$1"$2":')
}

function runQuery() {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return
  emit('run-query', tab.id, {
    filter:     toStrictJson(tab.filter),
    projection: toStrictJson(tab.projection),
    sort:       toStrictJson(tab.sort),
    skip:       tab.skip || 0,
    limit:      tab.limit || 50,
  })
}

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

const TYPE_ICON = { id: 'typeId', str: 'typeStr', num: 'typeNum', date: 'typeDate', bool: 'typeBool', null: 'typeNull', obj: 'typeObj' }
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
  const rest = [...seen].filter(k => k !== '_id').sort()
  return seen.has('_id') ? ['_id', ...rest] : rest
}

function fillerCount(results) {
  return Math.max(0, 24 - (results?.length || 0))
}

// ── JSON syntax highlighter ────────────────────────────
function syntaxHighlight(json) {
  return json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        if (match[0] === '"') {
          if (/:$/.test(match)) {
            return match[1] === '$'
              ? `<span class="jop">${match}</span>`
              : `<span class="jk">${match}</span>`
          }
          return `<span class="js">${match}</span>`
        }
        if (match === 'true' || match === 'false') return `<span class="jb">${match}</span>`
        if (match === 'null') return `<span class="jl">${match}</span>`
        return `<span class="jn">${match}</span>`
      }
    )
}

// ── pagination ─────────────────────────────────────────
const PAGE_SIZES = [10, 25, 50, 100, 200, 500]

function goFirst() {
  const tab = activeTab.value
  if (!tab) return
  tab.skip = 0
  runQuery()
}

function goPrev() {
  const tab = activeTab.value
  if (!tab) return
  tab.skip = Math.max(0, (tab.skip || 0) - (tab.limit || 50))
  runQuery()
}

function goNext() {
  const tab = activeTab.value
  if (!tab) return
  tab.skip = (tab.skip || 0) + (tab.limit || 50)
  runQuery()
}

function setPageSize(size) {
  const tab = activeTab.value
  if (!tab) return
  tab.limit = size
  tab.skip = 0
  pageSizeMenu.value = false
  runQuery()
}

// ── copy document ──────────────────────────────────────
function copySelectedDocument() {
  const tab = activeTab.value
  if (!tab || tab.selectedRow < 0) return
  navigator.clipboard.writeText(JSON.stringify(tab.results[tab.selectedRow], null, 2))
}

// ── column resize ──────────────────────────────────────
const tableRef   = ref(null)
const colWidths  = ref({})   // col name → px; empty = auto layout

// Reset widths when switching tabs so we re-measure on the new results
watch(() => activeTab.value?.id, () => { colWidths.value = {} })

let resizeCol = null
let resizeStartX = 0
let resizeStartWidth = 0

function startResize(e, col) {
  e.preventDefault()
  e.stopPropagation()
  // Capture natural browser-measured widths on the very first drag
  if (Object.keys(colWidths.value).length === 0) {
    const cols = columns(activeTab.value?.results || [])
    const ths  = tableRef.value?.querySelectorAll('thead th:not(.col-filler):not(.rownum)') || []
    const snap = {}
    ths.forEach((th, i) => { if (cols[i]) snap[cols[i]] = th.offsetWidth })
    colWidths.value = snap
  }
  resizeCol        = col
  resizeStartX     = e.clientX
  resizeStartWidth = colWidths.value[col] || 80
  document.body.style.cursor     = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup',   stopResize)
}

function onResizeMove(e) {
  if (resizeCol === null) return
  colWidths.value[resizeCol] = Math.max(40, resizeStartWidth + (e.clientX - resizeStartX))
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

  // Ensure fixed-layout mode before setting a single column width
  const cols = columns(activeTab.value?.results || [])
  if (Object.keys(colWidths.value).length === 0) {
    const ths = tableRef.value.querySelectorAll('thead th:not(.col-filler):not(.rownum)')
    const snap = {}
    ths.forEach((th, i) => { if (cols[i]) snap[cols[i]] = th.offsetWidth })
    colWidths.value = snap
  }

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

watch(() => activeTab.value?.id, () => { selectedCol.value = null; cellCtx.value = null })

function selectRow(rowIdx) {
  activeTab.value.selectedRow = rowIdx
  selectedCol.value = null
  cellCtx.value = null
}

function selectCell(rowIdx, col) {
  activeTab.value.selectedRow = rowIdx
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
  const tab = activeTab.value
  if (!tab || tab.selectedRow < 0 || !selectedCol.value) return
  const val = tab.results[tab.selectedRow]?.[selectedCol.value]
  navigator.clipboard.writeText(cellCopyValue(selectedCol.value, val))
}

function openCellCtx(e, rowIdx, col) {
  e.preventDefault()
  selectCell(rowIdx, col)
  cellCtx.value = { x: e.clientX, y: e.clientY, row: rowIdx, col: col }
}

function cellCtxPick(action) {
  const tab = activeTab.value
  const val = tab?.results[cellCtx.value?.row]?.[cellCtx.value?.col]
  if (action === 'copy-value') {
    navigator.clipboard.writeText(cellCopyValue(cellCtx.value.col, val))
  } else if (action === 'copy-json') {
    navigator.clipboard.writeText(JSON.stringify(val, null, 2))
  } else if (action === 'copy-doc') {
    navigator.clipboard.writeText(JSON.stringify(tab.results[cellCtx.value.row], null, 2))
  }
  cellCtx.value = null
}

function handleKeydown(e) {
  const tab = activeTab.value
  if (!tab) return

  if (e.key === 'Escape' && cellCtx.value) { cellCtx.value = null; return }

  if (tab.selectedRow < 0) return

  const cols   = columns(tab.results || [])
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
  } else if (e.key === 'ArrowDown' && rowIdx < (tab.results?.length ?? 0) - 1) {
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
  const tab = activeTab.value
  try {
    if (docModalMode.value === 'insert') {
      await invoke('insert_document', {
        id: tab.connectionId,
        uri: tab.uri,
        database: tab.dbName,
        collection: tab.collectionName,
        document: jsonStr,
      })
    } else {
      const original = tab.results[tab.selectedRow]
      await invoke('replace_document', {
        id: tab.connectionId,
        uri: tab.uri,
        database: tab.dbName,
        collection: tab.collectionName,
        idFilter: buildIdFilter(original),
        document: jsonStr,
      })
    }
    showDocModal.value = false
    runQuery()
  } catch (e) {
    crudError.value = String(e)
  } finally {
    crudSaving.value = false
  }
}

async function onDeleteConfirm() {
  const tab = activeTab.value
  const original = tab.results[tab.selectedRow]
  crudError.value = null
  try {
    await invoke('delete_document', {
      id: tab.connectionId,
      uri: tab.uri,
      database: tab.dbName,
      collection: tab.collectionName,
      idFilter: buildIdFilter(original),
    })
    showDeleteConfirm.value = false
    tab.selectedRow = -1
    runQuery()
  } catch (e) {
    crudError.value = String(e)
  }
}

// ── query code ─────────────────────────────────────────
const queryCode = computed(() => {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return ''
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
  <div class="work">
    <!-- Tabs -->
    <div class="tabs">
      <button
        v-for="t in tabs"
        :key="t.id"
        class="tab"
        :class="{ active: t.id === activeTabId }"
        @click="emit('activate-tab', t.id)"
      >
        <span>{{ t.title }}</span>
        <span class="x" @click.stop="emit('close-tab', t.id)">
          <BaseIcon name="close" :size="12" />
        </span>
      </button>
    </div>

    <!-- Quickstart pane -->
    <template v-if="!activeTab || activeTab.kind === 'quickstart'">
      <div class="quickstart">
        <h1>Welcome to Studio-4T</h1>
        <p>The cross-database workspace. MongoDB today — PostgreSQL, MySQL and more on the roadmap.</p>
        <div class="qs-grid">
          <div v-for="[ic, title, desc] in [
            ['connect',   'Connect to a database',  'Open the Connection Manager and pick a server.'],
            ['shell',     'Open IntelliShell',       'Autocompleting query console with inline results.'],
            ['aggregate', 'Build an aggregation',   'Visual pipeline editor with live stage previews.'],
            ['import',    'Import / Export',         'Move data between collections, files and engines.'],
          ]" :key="title" class="qs-card">
            <BaseIcon :name="ic" :size="24" style="color:var(--accent);flex-shrink:0" />
            <div>
              <div class="qs-card-title">{{ title }}</div>
              <div class="qs-card-desc">{{ desc }}</div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Collection workspace -->
    <template v-else-if="activeTab.kind === 'collection'">
      <!-- Breadcrumb -->
      <div class="crumbs">
        <BaseIcon name="connect" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.connectionName }}</span>
        <BaseIcon name="caret"  :size="11" class="sep" />
        <BaseIcon name="dbSmall" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.dbName }}</span>
        <BaseIcon name="caret"  :size="11" class="sep" />
        <BaseIcon name="collSmall" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.collectionName }}</span>
      </div>

      <!-- Query bar -->
      <div class="qbar">
        <button class="qbtn run" @click="runQuery" :disabled="activeTab.isRunning">
          <BaseIcon name="run" :size="16" class="ic" />
          {{ activeTab.isRunning ? 'Running…' : 'Run' }}
          <BaseIcon name="caretDown" :size="11" class="drop" />
        </button>
        <button class="qbtn" disabled><BaseIcon name="load"    :size="16" class="ic" /> Load query   <BaseIcon name="caretDown" :size="11" class="drop" /></button>
        <button class="qbtn" disabled><BaseIcon name="save"    :size="16" class="ic" /> Save query   <BaseIcon name="caretDown" :size="11" class="drop" /></button>
        <button class="qbtn" disabled><BaseIcon name="history" :size="16" class="ic" /> Query history</button>
        <button class="qbtn" disabled><BaseIcon name="anchor"  :size="16" class="ic" /> Set default query <BaseIcon name="caretDown" :size="11" class="drop" /></button>
        <button class="qbtn" disabled><BaseIcon name="copy"    :size="16" class="ic" /> Copy</button>
        <button class="qbtn" disabled><BaseIcon name="paste"   :size="16" class="ic" /> Paste</button>
        <span class="qbar-spacer"></span>
        <button class="vqb-toggle" :class="{ on: vqbOpen }" @click="emit('toggle-vqb')">
          <BaseIcon name="aggregate" :size="15" /> Visual Query Builder
        </button>
      </div>

      <!-- Query fields grid -->
      <div class="qfields">
        <span class="qlabel">Query</span>
        <div class="qinput">
          <input
            class="qval"
            v-model="activeTab.filter"
            placeholder="{}"
            @keydown.enter.ctrl="runQuery"
            @keydown.enter.meta="runQuery"
          />
          <span class="qicons">
            <BaseIcon name="brush" :size="15" @click="activeTab.filter = ''" style="cursor:pointer" />
          </span>
        </div>
        <span class="qlabel">Sort</span>
        <div class="qinput">
          <input class="qval" v-model="activeTab.sort" placeholder="{}" />
        </div>
        <span></span>

        <span class="qlabel">Projection</span>
        <div class="qinput">
          <input class="qval" v-model="activeTab.projection" placeholder="{}" />
        </div>
        <div class="num-cluster">
          <span class="qlabel">Limit</span>
          <div class="numbox">
            <input
              :value="activeTab.limit || 50"
              placeholder="50"
              inputmode="numeric"
              @input="activeTab.limit = Math.max(1, parseInt($event.target.value) || 1)"
            />
            <div class="num-steppers">
              <button tabindex="-1" @click="activeTab.limit = Math.max(1, (activeTab.limit || 50) + 1)">
                <BaseIcon name="caret" :size="9" style="transform: rotate(-90deg)" />
              </button>
              <button tabindex="-1" @click="activeTab.limit = Math.max(1, (activeTab.limit || 50) - 1)">
                <BaseIcon name="caret" :size="9" style="transform: rotate(90deg)" />
              </button>
            </div>
          </div>
          <span class="qlabel">Skip</span>
          <div class="numbox">
            <input
              :value="activeTab.skip || 0"
              placeholder="0"
              inputmode="numeric"
              @input="activeTab.skip = Math.max(0, parseInt($event.target.value) || 0)"
            />
            <div class="num-steppers">
              <button tabindex="-1" @click="activeTab.skip = Math.max(0, (activeTab.skip || 0) + 1)">
                <BaseIcon name="caret" :size="9" style="transform: rotate(-90deg)" />
              </button>
              <button tabindex="-1" @click="activeTab.skip = Math.max(0, (activeTab.skip || 0) - 1)">
                <BaseIcon name="caret" :size="9" style="transform: rotate(90deg)" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Results -->
      <div class="results">
        <!-- Result sub-tabs -->
        <div class="rtabs">
          <button
            v-for="t in ['Result', 'Query Code', 'Explain']"
            :key="t"
            class="rtab"
            :class="{ active: rtab === t }"
            @click="rtab = t"
          >{{ t }}</button>
        </div>

        <!-- Result toolbar -->
        <div class="rtoolbar" v-if="rtab === 'Result'">
          <button class="icon-btn" @click="runQuery" :disabled="activeTab.isRunning">
            <BaseIcon name="refresh" :size="16" />
          </button>
          <button class="icon-btn"
            :disabled="!activeTab.hasRun || (activeTab.skip || 0) === 0 || activeTab.isRunning"
            @click="goFirst"><BaseIcon name="first" :size="16" /></button>
          <button class="icon-btn"
            :disabled="!activeTab.hasRun || (activeTab.skip || 0) === 0 || activeTab.isRunning"
            @click="goPrev"><BaseIcon name="prev" :size="16" /></button>
          <button class="icon-btn"
            :disabled="!activeTab.hasRun || (activeTab.results?.length ?? 0) < (activeTab.limit || 50) || activeTab.isRunning"
            @click="goNext"><BaseIcon name="next" :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="last" :size="16" /></button>
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
            Documents {{ activeTab.results?.length ? `1 to ${activeTab.results.length}` : '-- to --' }}
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

        <!-- Error state -->
        <div v-if="activeTab.runError" class="run-error">{{ activeTab.runError }}</div>

        <!-- Table view -->
        <div v-else-if="rtab === 'Result' && viewMode === 'table'" class="grid-wrap">
          <div class="fieldpath">
            <span>{{ activeTab.collectionName }}</span>
          </div>
          <template v-if="!activeTab.hasRun || activeTab.isRunning">
            <table class="grid">
              <thead><tr>
                <th class="rownum"></th>
                <th style="min-width:320px">{Document id}</th>
              </tr></thead>
            </table>
            <div class="empty-rows"><div class="empty-rows-gutter"></div></div>
          </template>
          <template v-else-if="activeTab.results?.length === 0">
            <table class="grid">
              <thead><tr>
                <th class="rownum"></th>
                <th style="min-width:320px">{Document id}</th>
              </tr></thead>
            </table>
            <div class="empty-rows"><div class="empty-rows-gutter"></div></div>
          </template>
          <template v-else>
            <table
              class="grid"
              :class="{ 'fixed-cols': Object.keys(colWidths).length > 0 }"
              ref="tableRef"
            >
              <thead>
                <tr>
                  <th class="rownum"></th>
                  <th
                    v-for="col in columns(activeTab.results)"
                    :key="col"
                    :style="colWidths[col] ? { width: colWidths[col] + 'px' } : {}"
                  >
                    {{ col === '_id' ? '{Document id}' : col }}
                    <div class="col-resize-handle" @mousedown="startResize($event, col)" @dblclick.stop="autoFitColumn($event, col)"></div>
                  </th>
                  <th class="col-filler"></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, i) in activeTab.results"
                  :key="i"
                  :class="{ selrow: activeTab.selectedRow === i }"
                  @click="selectRow(i)"
                >
                  <td class="rownum">{{ i + 1 }}</td>
                  <td
                    v-for="col in columns(activeTab.results)"
                    :key="col"
                    :class="{ selcell: activeTab.selectedRow === i && selectedCol === col }"
                    @click.stop="selectCell(i, col)"
                    @contextmenu="openCellCtx($event, i, col)"
                  >
                    <span class="tcell" :class="'t-' + guessType(col, row[col])">
                      <span class="ticon">
                        <BaseIcon :name="TYPE_ICON[guessType(col, row[col])] || 'typeStr'" :size="14" />
                      </span>
                      <span class="tval" :class="TYPE_CLASS[guessType(col, row[col])]">
                        {{ formatCell(col, row[col]) }}
                      </span>
                    </span>
                  </td>
                  <td class="col-filler"></td>
                </tr>
                <tr
                  v-for="f in fillerCount(activeTab.results)"
                  :key="'f' + f"
                  class="filler"
                >
                  <td class="rownum"></td>
                  <td v-for="col in columns(activeTab.results)" :key="col"></td>
                  <td class="col-filler"></td>
                </tr>
              </tbody>
            </table>
          </template>
        </div>

        <!-- JSON view -->
        <div v-else-if="rtab === 'Result' && viewMode === 'json'" class="json-view">
          <div v-if="!activeTab.results?.length" style="padding:32px;color:var(--text-faint);font-size:12px">No documents</div>
          <div v-else class="json-doc" v-for="(doc, i) in activeTab.results" :key="i" v-html="syntaxHighlight(JSON.stringify(doc, null, 2))"></div>
        </div>

        <!-- Query Code sub-tab -->
        <div v-else-if="rtab === 'Query Code'" class="qcode-view">
          <pre class="qcode-pre"><span class="qcode-prompt">&gt;</span> {{ queryCode }}</pre>
        </div>

        <!-- Other sub-tabs placeholder -->
        <div v-else class="empty-rows" style="padding:32px;color:var(--text-faint);font-size:12px;display:flex;align-items:center;justify-content:center">
          {{ rtab }} — coming soon
        </div>

        <!-- Footer -->
        <div class="rfooter">
          <span>{{ activeTab.selectedRow >= 0 ? '1 document selected' : '0 documents selected' }}</span>
          <span class="spacer"></span>
          <span class="fitem"><BaseIcon name="count" :size="14" /> Count Documents</span>
          <span class="fitem" v-if="activeTab.elapsedMs != null">
            <BaseIcon name="clock" :size="14" />
            {{ (activeTab.elapsedMs / 1000).toFixed(3) }}s
          </span>
        </div>
      </div>
    </template>
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
.work { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

/* Tabs */
.tabs {
  display: flex;
  align-items: stretch;
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
  height: 32px;
  flex: none;
  padding-left: 6px;
  overflow-x: auto;
}
.tab {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  font-size: 12.5px;
  color: var(--text-dim);
  border-right: 1px solid var(--border);
  border-top: none;
  border-bottom: 2px solid transparent;
  border-left: none;
  background: none;
  max-width: 220px;
  white-space: nowrap;
  flex-shrink: 0;
}
.tab.active { background: var(--bg-window); color: var(--text); border-bottom-color: var(--accent); }
.tab .x { color: var(--text-faint); border-radius: 4px; padding: 1px; display: grid; place-items: center; }
.tab .x:hover { background: var(--bg-hover); color: var(--text); }

/* Quickstart */
.quickstart { flex: 1; overflow: auto; padding: 48px 56px; }
.quickstart h1 { font-size: 24px; font-weight: 600; margin-bottom: 6px; }
.quickstart p  { color: var(--text-dim); font-size: 13.5px; margin-bottom: 28px; }
.qs-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; max-width: 700px; }
.qs-card {
  background: var(--bg-panel);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  padding: 16px;
  display: flex;
  gap: 13px;
}
.qs-card-title { font-size: 13.5px; font-weight: 600; margin-bottom: 4px; }
.qs-card-desc  { font-size: 12px; color: var(--text-dim); }

/* Breadcrumb */
.crumbs {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 14px;
  font-size: 12.5px;
  color: var(--text-dim);
  border-bottom: 1px solid var(--border);
  flex: none;
}
.sep { color: var(--text-faint); }
.c-ic { color: var(--text-faint); }

/* Query bar */
.qbar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 10px;
  border-bottom: 1px solid var(--border);
  flex: none;
  flex-wrap: wrap;
}
.qbtn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 9px;
  border-radius: 6px;
  background: none;
  border: none;
  color: var(--text);
  font-size: 12.5px;
}
.qbtn:hover:not(:disabled) { background: var(--bg-hover); }
.qbtn.run { min-width: 92px; }
.qbtn.run .ic { color: var(--green); }
.qbtn .ic  { color: var(--text-dim); }
.qbtn .drop { color: var(--text-faint); }
.qbtn:disabled { opacity: .5; cursor: default; }
.qbar-spacer { flex: 1; }
.vqb-toggle {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text-dim);
  font-size: 12px;
}
.vqb-toggle.on { color: var(--accent); border-color: var(--accent-soft); }
.vqb-toggle:disabled { opacity: .4; }

/* Query fields */
.qfields {
  padding: 6px 12px 8px;
  display: grid;
  grid-template-columns: auto 1fr auto 1fr auto;
  gap: 6px 12px;
  align-items: center;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.qlabel { font-size: 12.5px; color: var(--text-dim); white-space: nowrap; }
.qinput {
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 5px 10px;
  font-family: var(--mono);
  font-size: 12.5px;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.qinput:focus-within { border-color: var(--accent); }
.qval {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  min-width: 0;
}
.qval::placeholder { color: var(--text-faint); }
.qicons { display: flex; gap: 4px; color: var(--text-faint); flex: none; }

/* Limit + Skip side by side, spanning the right 3 grid columns */
.num-cluster {
  grid-column: 3 / -1;
  display: flex;
  align-items: center;
  gap: 10px;
}

/* numeric stepper (Skip / Limit) */
.numbox {
  display: flex;
  align-items: stretch;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  width: 72px;
  overflow: hidden;
}
.numbox:focus-within { border-color: var(--accent); }
.numbox input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  padding: 5px 0 5px 9px;
}
.numbox input::placeholder { color: var(--text-faint); }
.num-steppers {
  display: flex;
  flex-direction: column;
  flex: none;
  border-left: 1px solid var(--border-soft);
}
.num-steppers button {
  flex: 1;
  width: 17px;
  display: grid;
  place-items: center;
  background: var(--bg-toolbar);
  border: none;
  color: var(--text-dim);
  padding: 0;
}
.num-steppers button:first-child { border-bottom: 1px solid var(--border-soft); }
.num-steppers button:hover { background: var(--bg-hover); color: var(--text); }

/* Results */
.results { flex: 1; display: flex; flex-direction: column; min-height: 0; }
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
  background: #2a2c30;
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

/* Grid */
.grid-wrap { flex: 1; overflow: auto; min-height: 0; background: var(--bg-window); }
.fieldpath {
  padding: 7px 12px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  z-index: 3;
}
table.grid {
  border-collapse: collapse;
  width: max-content;
  min-width: 100%;
  font-family: var(--mono);
  font-size: 12px;
}
table.grid th {
  position: sticky;
  top: 34px;
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
table.grid td.selcell { outline: 1px solid var(--accent); outline-offset: -1px; }
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
table.grid.fixed-cols { table-layout: fixed; }
th.col-filler, td.col-filler { border-right: none; }
/* In auto layout the filler needs a width hint to absorb leftover space;
   in fixed layout it's the only unspecified column so it gets remaining space naturally. */
table.grid:not(.fixed-cols) .col-filler { width: 100%; }

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
  width: 5px;
  height: 100%;
  cursor: col-resize;
  z-index: 1;
}
.col-resize-handle:hover { background: var(--accent); opacity: .5; }

.tcell { display: inline-flex; align-items: center; gap: 6px; vertical-align: middle; }
.ticon { color: var(--text-faint); display: grid; place-items: center; flex: none; }
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
  background: var(--bg-panel-2);
  border-right: 1px solid var(--border-soft);
}

/* JSON view */
.json-view { flex: 1; overflow: auto; padding: 12px 16px; }
.json-doc {
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.65;
  color: var(--text);
  white-space: pre;
  border-left: 2px solid var(--border-soft);
  padding: 8px 0 8px 14px;
  margin-bottom: 10px;
  -webkit-user-select: text;
  user-select: text;
}
/* syntax highlight token classes */
.json-doc :deep(.jk)  { color: var(--cell-key); }
.json-doc :deep(.jop) { color: var(--cell-op); }
.json-doc :deep(.js)  { color: var(--cell-str); }
.json-doc :deep(.jn)  { color: var(--cell-num); }
.json-doc :deep(.jb)  { color: var(--cell-num); }
.json-doc :deep(.jl)  { color: var(--text-faint); }

/* page size dropdown */
.page-size-wrap { position: relative; }
.page-size { cursor: pointer; }
.page-size-menu {
  position: absolute;
  top: 28px;
  left: 0;
  width: 70px;
  background: #2a2c30;
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
.run-error { padding: 10px 14px; color: #e07070; font-size: 12px; }

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
.btn.danger { background: #c0392b; color: #fff; }
.btn.danger:hover { background: #a93226; }

/* CRUD error banner */
.crud-err-banner {
  position: fixed;
  bottom: 48px;
  left: 50%;
  transform: translateX(-50%);
  background: #3a1a1a;
  border: 1px solid #c0392b;
  color: #e07070;
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
  color: #e07070;
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  flex: none;
}
</style>
