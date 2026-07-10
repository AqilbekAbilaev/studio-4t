<script setup>
import { ref, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import BaseIcon from './base/BaseIcon.vue'

const props = defineProps({
  activeTab:      { type: Object,  required: true },
  isAggregate:    { type: Boolean, default: false },
  runValid:       { type: Boolean, default: true },
  queryErrorText: { type: String,  default: null },
  vqbOpen:        { type: Boolean, default: false },
  clipboardQuery: { type: Object,  default: null },
  historyRequest: { type: Object,  default: null },
  saveRequest:    { type: Object,  default: null },
})
const emit = defineEmits(['run', 'copy-query', 'paste-query', 'toggle-vqb', 'toast', 'open-browser'])

// autofocus directive for the save-query input
const vFocus = { mounted(el) { el.focus(); el.select() } }

const showSaveForm    = ref(false)
const saveName        = ref('')
const showDefaultMenu = ref(false)
const historyMenu     = ref(false)
const historyEntries  = ref([])
const historyLoading  = ref(false)

function setMode(mode) {
  props.activeTab.mode = mode
}

// Sort spinner next to the Sort field: sets a one-key `_id` sort and runs.
// dir 1 = ascending (oldest first), dir -1 = descending (newest first).
function sortById(dir) {
  props.activeTab.sort = `{ _id: ${dir} }`
  emit('run')
}

// ── query history ──────────────────────────────────────
async function openHistoryMenu() {
  const tab = props.activeTab
  if (!tab || tab.kind !== 'collection') return
  if (historyMenu.value) {
    historyMenu.value = false
    return
  }
  historyLoading.value = true
  historyMenu.value = true
  try {
    historyEntries.value = await invoke('get_query_history', {
      connectionId: tab.connectionId,
      database:     tab.dbName,
      collection:   tab.collectionName,
    })
  } catch (_) {
    historyEntries.value = []
  } finally {
    historyLoading.value = false
  }
}

// View → History Manager: open the query-history menu on request from the native menu.
watch(() => props.historyRequest && props.historyRequest.nonce, (nonce) => {
  if (nonce == null) return
  if (!historyMenu.value) openHistoryMenu()
})

// File → Save: open the save-query form on request from the native menu.
watch(() => props.saveRequest && props.saveRequest.nonce, (nonce) => {
  if (nonce == null) return
  showSaveForm.value = true
})

async function applyHistoryEntry(entry) {
  const tab = props.activeTab
  if (!tab) return
  if (entry.mode === 'aggregate') {
    tab.mode     = 'aggregate'
    tab.pipeline = entry.pipeline
  } else {
    tab.mode       = 'find'
    tab.filter     = entry.filter
    tab.sort       = entry.sort
    tab.projection = entry.projection
    tab.skip       = Number(entry.skip)
    tab.limit      = Number(entry.limit)
  }
  historyMenu.value = false
  await nextTick()
  emit('run')
}

async function clearHistory() {
  const tab = props.activeTab
  if (!tab) return
  try {
    await invoke('clear_query_history', {
      connectionId: tab.connectionId,
      database:     tab.dbName,
      collection:   tab.collectionName,
    })
    historyEntries.value = []
  } catch (_) {}
}

async function setDefaultQuery() {
  const tab = props.activeTab
  if (!tab) return
  try {
    await invoke('set_default_query', {
      connectionId: tab.connectionId,
      database:     tab.dbName,
      collection:   tab.collectionName,
      mode:         tab.mode       || 'find',
      filter:       tab.filter     || '',
      sort:         tab.sort       || '',
      projection:   tab.projection || '',
      skip:         tab.skip       ?? 0,
      limit:        tab.limit      ?? 50,
      pipeline:     tab.pipeline   || '',
    })
    showDefaultMenu.value = false
    emit('toast', 'Default query set for this collection.')
  } catch (e) {
    emit('toast', 'Failed: ' + errMessage(e))
  }
}

async function clearDefaultQuery() {
  const tab = props.activeTab
  if (!tab) return
  try {
    await invoke('clear_default_query', {
      connectionId: tab.connectionId,
      database:     tab.dbName,
      collection:   tab.collectionName,
    })
    showDefaultMenu.value = false
    emit('toast', 'Default query cleared.')
  } catch (e) {
    emit('toast', 'Failed: ' + errMessage(e))
  }
}

async function saveCurrentQuery() {
  const tab = props.activeTab
  const name = saveName.value.trim()
  if (!tab || !name) return
  try {
    await invoke('save_query', {
      name:       name,
      mode:       tab.mode       || 'find',
      filter:     tab.filter     || '',
      sort:       tab.sort       || '',
      projection: tab.projection || '',
      skip:       tab.skip       ?? 0,
      limit:      tab.limit      ?? 50,
      pipeline:   tab.pipeline   || '',
    })
    showSaveForm.value = false
    saveName.value = ''
    emit('toast', `Saved as "${name}"`)
  } catch (e) {
    emit('toast', 'Save failed: ' + errMessage(e))
  }
}

function formatHistoryTime(ranAt) {
  const ms = Number(ranAt)
  if (!ms) return ''
  return new Date(ms).toLocaleString(undefined, {
    month:  'short',
    day:    'numeric',
    hour:   '2-digit',
    minute: '2-digit',
  })
}

function historyLabel(entry) {
  if (entry.mode === 'aggregate') {
    const p = (entry.pipeline || '').trim()
    return p.length > 60 ? p.slice(0, 60) + '…' : (p || '[]')
  }
  const f = (entry.filter || '').trim()
  return f.length > 60 ? f.slice(0, 60) + '…' : (f || '{}')
}

watch(() => props.activeTab && props.activeTab.id, () => {
  historyMenu.value = false
  historyEntries.value = []
})
</script>

<template>
  <!-- Query bar -->
  <div class="qbar">
    <div class="mode-toggle">
      <button :class="{ on: !isAggregate }" @click="setMode('find')">Find</button>
      <button :class="{ on: isAggregate }" @click="setMode('aggregate')">Aggregate</button>
    </div>
    <button class="qbtn run" @click="emit('run')" :disabled="activeTab.isRunning || !runValid">
      <BaseIcon name="run" :size="16" class="ic" />
      {{ activeTab.isRunning ? 'Running…' : 'Run' }}
    </button>
    <template v-if="!isAggregate">
      <button class="qbtn" @click="emit('open-browser')"><BaseIcon name="load" :size="16" class="ic" /> Load query</button>
      <div class="save-wrap">
        <button class="qbtn" :class="{ on: showSaveForm }" @click="showSaveForm = !showSaveForm">
          <BaseIcon name="save" :size="16" class="ic" /> Save query
        </button>
        <div v-if="showSaveForm" class="save-backdrop" @mousedown.self="showSaveForm = false"></div>
        <div v-if="showSaveForm" class="save-form">
          <input
            v-model="saveName"
            placeholder="Query name…"
            class="save-input"
            @keydown.enter.prevent="saveCurrentQuery"
            @keydown.escape="showSaveForm = false"
            v-focus
          />
          <button class="save-btn primary" @click="saveCurrentQuery" :disabled="!saveName.trim()">Save</button>
          <button class="save-btn" @click="showSaveForm = false">Cancel</button>
        </div>
      </div>
      <div class="hist-wrap">
        <button class="qbtn" :class="{ on: historyMenu }" @click="openHistoryMenu">
          <BaseIcon name="history" :size="16" class="ic" /> Query history
        </button>
        <div v-if="historyMenu" class="hist-backdrop" @mousedown.self="historyMenu = false"></div>
        <div v-if="historyMenu" class="hist-menu">
          <div class="hist-header">
            <span class="hist-title">Query History</span>
            <button class="hist-clear" @click="clearHistory" :disabled="!historyEntries.length">Clear</button>
          </div>
          <div v-if="historyLoading" class="hist-empty">Loading…</div>
          <div v-else-if="!historyEntries.length" class="hist-empty">No history for this collection.</div>
          <div v-else class="hist-list">
            <div
              v-for="entry in historyEntries"
              :key="entry.id"
              class="hist-item"
              @click="applyHistoryEntry(entry)"
            >
              <div class="hist-item-top">
                <span class="hist-mode">{{ entry.mode }}</span>
                <span class="hist-time">{{ formatHistoryTime(entry.ran_at) }}</span>
              </div>
              <div class="hist-query">{{ historyLabel(entry) }}</div>
            </div>
          </div>
        </div>
      </div>
      <div class="default-wrap">
        <button class="qbtn" :class="{ on: showDefaultMenu }" @click="showDefaultMenu = !showDefaultMenu">
          <BaseIcon name="anchor" :size="16" class="ic" /> Set default query
          <BaseIcon name="caretDown" :size="11" class="drop" />
        </button>
        <div v-if="showDefaultMenu" class="default-backdrop" @mousedown.self="showDefaultMenu = false"></div>
        <div v-if="showDefaultMenu" class="default-menu">
          <button class="default-item" @click="setDefaultQuery">
            <BaseIcon name="anchor" :size="13" class="ic" /> Set as default for this collection
          </button>
          <button class="default-item" @click="clearDefaultQuery">
            <BaseIcon name="trash" :size="13" class="ic" /> Clear default
          </button>
        </div>
      </div>
      <button class="qbtn" @click="emit('copy-query')">
        <BaseIcon name="copy" :size="16" class="ic" /> Copy
      </button>
      <button class="qbtn" :disabled="!clipboardQuery" @click="emit('paste-query')">
        <BaseIcon name="paste" :size="16" class="ic" /> Paste
      </button>
    </template>
    <span class="qbar-spacer"></span>
    <button v-if="!isAggregate" class="vqb-toggle" :class="{ on: vqbOpen }" @click="emit('toggle-vqb')">
      <BaseIcon name="aggregate" :size="15" /> Visual Query Builder
    </button>
  </div>

  <!-- Query fields grid (find mode) -->
  <template v-if="!isAggregate">
    <div class="qfields">
      <span class="qlabel">Query</span>
      <div class="qinput">
        <input
          class="qval"
          :value="activeTab.filter"
          @input="activeTab.filter = $event.target.value"
          placeholder="{}"
          spellcheck="false"
          autocorrect="off"
          autocapitalize="off"
          @keydown.enter.prevent="emit('run')"
        />
        <span class="qicons">
          <BaseIcon name="brush" :size="15" @click="activeTab.filter = ''" style="cursor:pointer" />
        </span>
      </div>
      <span class="qlabel">Sort</span>
      <div class="qinput">
        <input class="qval" :value="activeTab.sort" @input="activeTab.sort = $event.target.value" placeholder="{}" spellcheck="false" autocorrect="off" autocapitalize="off" @keydown.enter.prevent="emit('run')" />
        <span class="qicon-col">
          <BaseIcon name="caret" :size="11" style="transform: rotate(-90deg)" title="Sort by _id ascending (oldest first)" @click="sortById(1)" />
          <BaseIcon name="caret" :size="11" style="transform: rotate(90deg)" title="Sort by _id descending (newest first)" @click="sortById(-1)" />
        </span>
      </div>
      <span></span>

      <span class="qlabel">Projection</span>
      <div class="qinput">
        <input class="qval" :value="activeTab.projection" @input="activeTab.projection = $event.target.value" placeholder="{}" spellcheck="false" autocorrect="off" autocapitalize="off" @keydown.enter.prevent="emit('run')" />
      </div>
      <div class="num-cluster">
        <span class="qlabel">Limit</span>
        <div class="numbox">
          <input
            :value="activeTab.limit || 50"
            placeholder="50"
            inputmode="numeric"
            @input="activeTab.limit = Math.max(1, parseInt($event.target.value) || 1)"
            @keydown.enter.prevent="emit('run')"
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
            @keydown.enter.prevent="emit('run')"
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
    <div v-if="queryErrorText" class="qparse-error">{{ queryErrorText }}</div>
  </template>
</template>

<style scoped>
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
.qbtn.run { min-width: 92px; border: 1px solid var(--green); }
.qbtn.run .ic { color: var(--green); }
.qbtn .ic  { color: var(--text-dim); }
.qbtn .drop { color: var(--text-faint); }
.qbtn:disabled { opacity: .5; cursor: default; }
.qbtn.on { background: var(--bg-hover); }
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

.mode-toggle { display: flex; border: 1px solid var(--border-soft); border-radius: 6px; overflow: hidden; margin-right: 6px; }
.mode-toggle button { padding: 4px 11px; background: none; border: none; color: var(--text-dim); font-size: 12px; cursor: pointer; }
.mode-toggle button.on { background: var(--accent); color: #fff; }

.qparse-error { color: var(--danger-text); font-size: 12px; padding: 4px 12px 6px; flex: none; }

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

/* Sort spinner: stacked up/down carets that set an _id sort and run. */
.qicon-col {
  display: flex;
  flex-direction: column;
  flex: none;
  line-height: 0;
  color: var(--text-faint);
}
.qicon-col :deep(svg) { cursor: pointer; }
.qicon-col :deep(svg:hover) { color: var(--text); }

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

/* ── set default query dropdown ────────────────────────── */
.default-wrap { position: relative; }
.default-backdrop { position: fixed; inset: 0; z-index: 19; }
.default-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 240px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 10px 28px rgba(0,0,0,.5);
  z-index: 20;
  padding: 4px;
  display: flex;
  flex-direction: column;
}
.default-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 5px;
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 12.5px;
  cursor: pointer;
  text-align: left;
  width: 100%;
}
.default-item:hover { background: var(--bg-hover); color: var(--text); }
.default-item .ic { color: var(--text-faint); flex: none; }

/* ── save query popover ────────────────────────────────── */
.save-wrap { position: relative; }
.save-backdrop { position: fixed; inset: 0; z-index: 19; }
.save-form {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  width: 290px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 10px 28px rgba(0,0,0,.5);
  z-index: 20;
  display: flex;
  gap: 6px;
  padding: 10px;
}
.save-input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  color: var(--text);
  font-size: 12.5px;
  padding: 6px 9px;
  outline: none;
  min-width: 0;
}
.save-input:focus { border-color: var(--accent); }
.save-btn {
  padding: 5px 10px;
  border-radius: 5px;
  font-size: 12px;
  background: var(--bg-toolbar);
  border: 1px solid var(--border-soft);
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
}
.save-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.save-btn.primary:hover:not(:disabled) { background: var(--accent-soft); }
.save-btn:disabled { opacity: .4; cursor: default; }

/* ── query history dropdown ────────────────────────────── */
.hist-wrap { position: relative; }
.hist-backdrop {
  position: fixed;
  inset: 0;
  z-index: 19;
}
.hist-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  width: 340px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 14px 34px rgba(0,0,0,.55);
  z-index: 20;
  display: flex;
  flex-direction: column;
  max-height: 360px;
}
.hist-header {
  display: flex;
  align-items: center;
  padding: 8px 12px 6px;
  border-bottom: 1px solid var(--border-soft);
  flex: none;
}
.hist-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-dim);
  flex: 1;
}
.hist-clear {
  font-size: 11px;
  color: var(--text-faint);
  background: none;
  border: none;
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
}
.hist-clear:hover:not(:disabled) { color: var(--text); background: var(--bg-hover); }
.hist-clear:disabled { opacity: .4; cursor: default; }
.hist-empty {
  padding: 20px 14px;
  font-size: 12px;
  color: var(--text-faint);
  text-align: center;
}
.hist-list {
  overflow-y: auto;
  flex: 1;
  padding: 4px;
}
.hist-item {
  padding: 7px 10px;
  border-radius: 5px;
  cursor: pointer;
}
.hist-item:hover { background: var(--bg-hover); }
.hist-item-top {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 2px;
}
.hist-mode {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: .5px;
  color: var(--accent);
  font-weight: 600;
}
.hist-time {
  font-size: 10.5px;
  color: var(--text-faint);
  margin-left: auto;
}
.hist-query {
  font-family: var(--mono);
  font-size: 11.5px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
