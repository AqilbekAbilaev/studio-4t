<script setup>
import { ref, computed, nextTick, defineAsyncComponent } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import TabBar from './TabBar.vue'
import QuickstartPane from './QuickstartPane.vue'
import QueryBar from './QueryBar.vue'
import PipelineEditor from './PipelineEditor.vue'
import ResultsPanel from './ResultsPanel.vue'
// Lazy-loaded so CodeMirror (a large dep) is only fetched when a shell tab opens.
const ShellConsole = defineAsyncComponent(() => import('./ShellConsole.vue'))
import QueryBrowserModal from './QueryBrowserModal.vue'
import { parseField, parsePipeline } from '../utils/queryParser'

const props = defineProps({
  tabs:           { type: Array,   required: true },
  activeTabId:    { type: String,  required: true },
  vqbOpen:        { type: Boolean, default: false },
  clipboardQuery: { type: Object,  default: null },
  docMenuRequest: { type: Object,  default: null },
  historyRequest: { type: Object,  default: null },
})
const emit = defineEmits(['activate-tab', 'close-tab', 'tab-context', 'run-query', 'run-aggregate', 'toggle-vqb', 'open-vqb', 'close-vqb', 'toast', 'copy-query', 'paste-query', 'cancel-query'])

const showQueryBrowser = ref(false)

const activeTab = computed(() => props.tabs.find(t => t.id === props.activeTabId))
const isAggregate = computed(() => activeTab.value && activeTab.value.mode === 'aggregate')

// Which result sub-tab is active. Kept here (rather than in ResultsPanel) because the
// run pipeline below lazily refreshes the Explain plan whenever a query re-runs while
// the Explain tab is open.
const rtab = ref('Result')

// ── query parsing & validation ─────────────────────────────
// Shell syntax is parsed to canonical Extended JSON by utils/queryParser.js (MongoDB's
// own parser), which the Rust backend decodes to BSON. Fields are parsed live so we can
// show an inline error and disable Run while the query is invalid, instead of silently
// sending corrupted JSON.
const parsedQuery = computed(() => {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return null
  return {
    filter:     parseField(tab.filter),
    projection: parseField(tab.projection),
    sort:       parseField(tab.sort),
  }
})
const parsedPipeline = computed(() => {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return null
  return parsePipeline(tab.pipeline)
})
const queryValid = computed(() => {
  const p = parsedQuery.value
  return !p || (p.filter.ok && p.projection.ok && p.sort.ok)
})
const pipelineValid = computed(() => {
  const p = parsedPipeline.value
  return !p || p.ok
})
const runValid = computed(() => (isAggregate.value ? pipelineValid.value : queryValid.value))
// First offending field's message, shown under the query area / pipeline editor.
const queryErrorText = computed(() => {
  const p = parsedQuery.value
  if (!p) return null
  if (!p.filter.ok) return 'Query: ' + p.filter.error
  if (!p.projection.ok) return 'Projection: ' + p.projection.error
  if (!p.sort.ok) return 'Sort: ' + p.sort.error
  return null
})
const pipelineErrorText = computed(() => {
  const p = parsedPipeline.value
  if (!p || p.ok) return null
  return 'Pipeline: ' + p.error
})

// The Run button (and the result toolbar's refresh) dispatch on the tab's mode.
function run() {
  if (isAggregate.value) {
    runAggregate()
  } else {
    runQuery()
  }
}

function runAggregate() {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return
  const parsed = parsedPipeline.value
  if (!parsed || !parsed.ok) return  // inline error is already shown
  emit('run-aggregate', tab.id, { pipeline: parsed.ejson })
}

function runQuery(addToHistory = true) {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return
  expandIdFilter(tab)
  const parsed = parsedQuery.value
  if (!parsed || !parsed.filter.ok || !parsed.projection.ok || !parsed.sort.ok) return
  emit('run-query', tab.id, {
    filter:        parsed.filter.ejson,
    projection:    parsed.projection.ejson,
    sort:          parsed.sort.ejson,
    skip:          tab.skip || 0,
    limit:         tab.limit || 50,
    addToHistory:  addToHistory,
  })
  // Keep the Explain plan in sync when it's the visible sub-tab.
  if (rtab.value === 'Explain') runExplain()
}

// Switch result sub-tab; the Explain plan is fetched lazily the first time it's
// shown (and re-fetched whenever the query re-runs while it's open).
function selectRtab(t) {
  rtab.value = t
  if (t === 'Explain') runExplain()
}

async function runExplain() {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return
  const parsed = parsedQuery.value
  if (!parsed || !parsed.filter.ok || !parsed.projection.ok || !parsed.sort.ok) {
    tab.explainError = 'Fix the query before running Explain.'
    tab.explainResult = null
    return
  }
  tab.explainRunning = true
  tab.explainError = null
  try {
    const result = await invoke('explain_query', {
      id:         tab.connectionId,
      database:   tab.dbName,
      collection: tab.collectionName,
      filter:     parsed.filter.ejson,
      projection: parsed.projection.ejson,
      sort:       parsed.sort.ejson,
      skip:       tab.skip || 0,
      limit:      tab.limit || 50,
    })
    tab.explainResult = result
  } catch (e) {
    tab.explainError = errMessage(e)
    tab.explainResult = null
  } finally {
    tab.explainRunning = false
  }
}

// When the whole Query value is a bare 24-hex ObjectId, build the _id filter so you
// can drop a copied id straight into the box. Done at run time (not on every
// keystroke) so the field stays a plain text input — rewriting its value on input is
// what defeats the browser's native undo/redo.
function expandIdFilter(tab) {
  const v = (tab.filter || '').trim()
  if (/^[0-9a-fA-F]{24}$/.test(v)) {
    tab.filter = `{ _id: ObjectId("${v}") }`
  }
}

function openQueryBrowser() {
  showQueryBrowser.value = true
}

async function applyFromBrowser(entry) {
  const tab = activeTab.value
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
  await nextTick()
  run()
}
</script>

<template>
  <div class="work">
    <!-- Tabs -->
    <TabBar
      :tabs="tabs"
      :active-tab-id="activeTabId"
      @activate-tab="emit('activate-tab', $event)"
      @close-tab="emit('close-tab', $event)"
      @tab-context="emit('tab-context', $event)"
    />

    <!-- Quickstart pane -->
    <QuickstartPane v-if="!activeTab || activeTab.kind === 'quickstart'" />

    <!-- IntelliShell -->
    <ShellConsole v-else-if="activeTab.kind === 'shell'" :active-tab="activeTab" />

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

      <!-- Query bar + find-mode inputs -->
      <QueryBar
        :active-tab="activeTab"
        :is-aggregate="isAggregate"
        :run-valid="runValid"
        :query-error-text="queryErrorText"
        :vqb-open="vqbOpen"
        :clipboard-query="clipboardQuery"
        :history-request="historyRequest"
        @run="run"
        @copy-query="emit('copy-query')"
        @paste-query="emit('paste-query')"
        @toggle-vqb="emit('toggle-vqb')"
        @toast="emit('toast', $event)"
        @open-browser="openQueryBrowser"
      />

      <!-- Aggregation pipeline editor -->
      <PipelineEditor
        v-if="isAggregate"
        :active-tab="activeTab"
        :pipeline-error-text="pipelineErrorText"
        @run="run"
      />

      <!-- Results -->
      <ResultsPanel
        :active-tab="activeTab"
        :is-aggregate="isAggregate"
        :run-valid="runValid"
        :rtab="rtab"
        :vqb-open="vqbOpen"
        :tabs="tabs"
        :active-tab-id="activeTabId"
        :doc-menu-request="docMenuRequest"
        @run="run"
        @requery="runQuery"
        @select-rtab="selectRtab"
        @open-vqb="emit('open-vqb')"
        @close-vqb="emit('close-vqb')"
        @toast="emit('toast', $event)"
        @cancel="activeTab && emit('cancel-query', activeTab.id)"
      />
    </template>
  </div>

  <QueryBrowserModal
    v-if="showQueryBrowser"
    @close="showQueryBrowser = false"
    @apply="applyFromBrowser"
  />
</template>

<style scoped>
.work { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

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
</style>
