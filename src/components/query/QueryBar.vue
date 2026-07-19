<script setup>
import { ref, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import { DATE_TAGS } from '../../utils/dateTags'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import SegmentedControl from '../base/SegmentedControl.vue'
import NumberStepper from '../base/NumberStepper.vue'
import MenuItem from '../base/MenuItem.vue'

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
const showDateTags    = ref(false)

// Insert a date tag into the Query field at the caret (append with a space if the
// field already has content). Tags expand to a concrete date when the query runs.
function insertDateTag(token) {
  const current = props.activeTab.filter || ''
  const needsSpace = current.length > 0 && !/\s$/.test(current)
  props.activeTab.filter = current + (needsSpace ? ' ' : '') + '#' + token
  showDateTags.value = false
}

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
    emit('toast', 'Failed: ' + errText(e))
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
    emit('toast', 'Failed: ' + errText(e))
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
    emit('toast', 'Save failed: ' + errText(e))
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
    <SegmentedControl
      class="mode-toggle"
      :model-value="isAggregate ? 'aggregate' : 'find'"
      :options="[{ value: 'find', label: 'Find' }, { value: 'aggregate', label: 'Aggregate' }]"
      @update:model-value="setMode"
    />
    <BaseButton variant="ghost" size="sm" class="run" @click="emit('run')" :disabled="activeTab.isRunning || !runValid">
      <BaseIcon name="run" :size="18" class="ic" />
      {{ activeTab.isRunning ? 'Running…' : 'Run' }}
    </BaseButton>
    <template v-if="!isAggregate">
      <BaseButton variant="ghost" size="sm" @click="emit('open-browser')"><BaseIcon name="load" :size="18" class="ic" /> Load query</BaseButton>
      <div class="save-wrap">
        <BaseButton variant="ghost" size="sm" :active="showSaveForm" @click="showSaveForm = !showSaveForm">
          <BaseIcon name="save" :size="18" class="ic" /> Save query
        </BaseButton>
        <div v-if="showSaveForm" class="save-backdrop" @mousedown.self="showSaveForm = false"></div>
        <div v-if="showSaveForm" class="save-form">
          <BaseInput
            v-model="saveName"
            placeholder="Query name…"
            class="save-input"
            @keydown.enter.prevent="saveCurrentQuery"
            @keydown.escape="showSaveForm = false"
            v-focus
          />
          <BaseButton variant="primary" size="sm" @click="saveCurrentQuery" :disabled="!saveName.trim()">Save</BaseButton>
          <BaseButton variant="ghost" size="sm" bordered @click="showSaveForm = false">Cancel</BaseButton>
        </div>
      </div>
      <div class="hist-wrap">
        <BaseButton variant="ghost" size="sm" :active="historyMenu" @click="openHistoryMenu">
          <BaseIcon name="history" :size="18" class="ic" /> Query history
        </BaseButton>
        <div v-if="historyMenu" class="hist-backdrop" @mousedown.self="historyMenu = false"></div>
        <div v-if="historyMenu" class="hist-menu">
          <div class="hist-header">
            <span class="hist-title">Query History</span>
            <BaseButton variant="ghost" size="sm" @click="clearHistory" :disabled="!historyEntries.length">Clear</BaseButton>
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
        <BaseButton variant="ghost" size="sm" :active="showDefaultMenu" @click="showDefaultMenu = !showDefaultMenu">
          <BaseIcon name="anchor" :size="18" class="ic" /> Set default query
          <BaseIcon name="caretDown" :size="11" class="drop" />
        </BaseButton>
        <div v-if="showDefaultMenu" class="default-backdrop" @mousedown.self="showDefaultMenu = false"></div>
        <div v-if="showDefaultMenu" class="default-menu">
          <MenuItem @click="setDefaultQuery">
            <BaseIcon name="anchor" :size="13" class="ic" /> Set as default for this collection
          </MenuItem>
          <MenuItem @click="clearDefaultQuery">
            <BaseIcon name="trash" :size="13" class="ic" /> Clear default
          </MenuItem>
        </div>
      </div>
      <BaseButton variant="ghost" size="sm" @click="emit('copy-query')">
        <BaseIcon name="copy" :size="18" class="ic" /> Copy
      </BaseButton>
      <BaseButton variant="ghost" size="sm" :disabled="!clipboardQuery" @click="emit('paste-query')">
        <BaseIcon name="paste" :size="18" class="ic" /> Paste
      </BaseButton>
    </template>
    <span class="qbar-spacer"></span>
    <BaseButton v-if="!isAggregate" size="sm" bordered class="vqb-toggle" :class="{ on: vqbOpen }" @click="emit('toggle-vqb')">
      <BaseIcon name="aggregate" :size="15" /> Visual Query Builder
    </BaseButton>
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
          <span class="datetag-wrap">
            <span class="datetag-btn" :class="{ on: showDateTags }" title="Insert a dynamic date tag" @click="showDateTags = !showDateTags">#</span>
            <div v-if="showDateTags" class="datetag-backdrop" @mousedown.self="showDateTags = false"></div>
            <div v-if="showDateTags" class="datetag-menu">
              <div class="datetag-head">Date tags — expand to a date when the query runs</div>
              <MenuItem v-for="tag in DATE_TAGS" :key="tag.token" @click="insertDateTag(tag.token)">
                <code class="datetag-code">{{ tag.label }}</code>
                <span class="datetag-hint">{{ tag.hint }}</span>
              </MenuItem>
            </div>
          </span>
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
        <NumberStepper :model-value="activeTab.limit || 50" :min="1" placeholder="50"
          @update:model-value="activeTab.limit = $event" @enter="emit('run')" />
        <span class="qlabel">Skip</span>
        <NumberStepper :model-value="activeTab.skip || 0" :min="0" placeholder="0"
          @update:model-value="activeTab.skip = $event" @enter="emit('run')" />
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
/* Run button: green outline + fixed width, left-aligned so the icon doesn't shift
   when the label flips Run → Running…. Scoped to .base-btn to beat BaseButton's
   own border/justify defaults reliably (order-independent). */
.base-btn.run { min-width: 92px; justify-content: flex-start; border: 1px solid var(--green); }
.run .ic { color: var(--green); }
.ic  { color: var(--text-dim); }
.drop { color: var(--text-faint); }
.qbar-spacer { flex: 1; }
.vqb-toggle.on { color: var(--accent); border-color: var(--accent-soft); }
.vqb-toggle:disabled { opacity: .4; }

.mode-toggle { margin-right: 6px; }

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

/* Date-tags helper (Query field) */
.datetag-wrap { position: relative; display: inline-flex; }
.datetag-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  font-family: var(--mono);
  font-size: 13px;
  font-weight: 700;
  color: var(--text-faint);
  cursor: pointer;
  border-radius: 4px;
}
.datetag-btn:hover, .datetag-btn.on { color: var(--accent); background: var(--bg-hover); }
.datetag-backdrop { position: fixed; inset: 0; z-index: 19; }
.datetag-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 280px;
  background: var(--bg-field);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 10px 28px rgba(0,0,0,.5);
  z-index: 20;
  padding: 4px;
  display: flex;
  flex-direction: column;
  max-height: 320px;
  overflow: auto;
}
.datetag-head {
  padding: 7px 10px;
  font-size: 11px;
  color: var(--text-faint);
  border-bottom: 1px solid var(--border-soft);
  margin-bottom: 4px;
}
.datetag-code { font-family: var(--mono); font-size: 12px; color: var(--accent); flex: none; }
.datetag-hint { font-size: 11.5px; color: var(--text-dim); }

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
.base-input.save-input { flex: 1; min-width: 0; }

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
