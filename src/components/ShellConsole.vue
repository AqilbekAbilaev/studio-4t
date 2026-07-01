<script setup>
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import ResultTable from './ResultTable.vue'
import TreeView from './TreeView.vue'
import { mongoStringify, syntaxHighlight } from '../utils/mongoFormat'
import { buildExtensions, EditorView, EditorState } from '../utils/shellEditor'

// IntelliShell, Studio-3T style: a code editor on top, the command's output in
// the reused result grid (Table / JSON / Tree) below, plus a Console tab for
// print() output, scalar results, and errors. Bound to a shell tab (connection
// + database) with its own backend JS session (tab.sessionId).
const props = defineProps({
  activeTab: { type: Object, required: true },
})

const historyMenu = ref(false)
const drillPath = computed({
  get: () => props.activeTab.drillPath || [],
  set: (val) => { props.activeTab.drillPath = val },
})

const VIEWS = [['table', 'Table View'], ['json', 'JSON View'], ['tree', 'Tree View']]
const viewMenu = ref(false)
const viewLabel = computed(() => {
  const found = VIEWS.find(([k]) => k === props.activeTab.resultView)
  return found ? found[1] : 'Table View'
})

const resultCount = computed(() => props.activeTab.results?.length ?? 0)

// ── CodeMirror editor ──────────────────────────────────
const editorEl = ref(null)
let view = null
let syncingFromTab = false          // guard so programmatic doc swaps don't echo back
const dbCollections = ref([])       // collection names for `db.` autocomplete

function createEditor() {
  if (!editorEl.value) return
  const extensions = buildExtensions({
    onRun:     () => run(),
    onRunLine: (line) => run(line),
    onChange:  (text) => { if (!syncingFromTab) props.activeTab.code = text },
    getCollections: () => dbCollections.value,
  })
  view = new EditorView({
    state: EditorState.create({ doc: props.activeTab.code || '', extensions: extensions }),
    parent: editorEl.value,
  })
}

// Swap the editor's contents when the active shell tab changes, without echoing
// the swap back into the (now different) tab's code.
function setEditorDoc(text) {
  if (!view) return
  syncingFromTab = true
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text || '' } })
  syncingFromTab = false
}

// Best-effort: preload this database's collection names for `db.` completion.
async function loadCollections() {
  try {
    const dbs = await invoke('list_databases', { id: props.activeTab.connectionId })
    const match = Array.isArray(dbs) ? dbs.find(d => d.name === props.activeTab.dbName) : null
    if (match && Array.isArray(match.collections)) dbCollections.value = match.collections
  } catch (_) {}
}

onMounted(async () => {
  await nextTick()
  createEditor()
  loadCollections()
  // Load this connection's persisted command history so the History dropdown
  // (and recall) spans previous sessions.
  try {
    const past = await invoke('get_shell_history', { connectionId: props.activeTab.connectionId })
    if (Array.isArray(past)) props.activeTab.history = past
  } catch (_) {}
})

onUnmounted(() => {
  if (view) { view.destroy(); view = null }
})

// One ShellConsole instance is reused across shell tabs — reset the editor and
// reload completions whenever the bound tab changes.
watch(() => props.activeTab.id, () => {
  setEditorDoc(props.activeTab.code || '')
  dbCollections.value = []
  loadCollections()
})

async function run(codeOverride) {
  const tab = props.activeTab
  const code = (codeOverride ?? tab.code ?? '').trim()
  if (!code || tab.isRunning) return

  tab.isRunning = true
  tab.runError = null
  const t0 = Date.now()
  invoke('push_shell_command', { connectionId: tab.connectionId, command: code }).catch(() => {})
  if (!tab.history.includes(code)) tab.history.push(code)

  try {
    const res = await invoke('run_shell_command', {
      id:        tab.connectionId,
      database:  tab.dbName,
      sessionId: tab.sessionId,
      code:      code,
    })
    tab.elapsedMs = Date.now() - t0
    tab.logs = res.logs || []
    tab.selectedRow = -1
    tab.drillPath = []
    tab.hasRun = true   // ResultTable hides its grid until a run has happened

    if (res.error) {
      tab.runError = res.error
      tab.results = []
      tab.hasScalar = false
      tab.resultTab = 'Console'
    } else {
      tab.runError = null
      const value = res.value
      if (Array.isArray(value)) {
        tab.results = value
        tab.hasScalar = false
        // Objects → table; arrays of scalars → JSON reads better.
        const objects = value.length === 0 || value.every(v => v && typeof v === 'object' && !Array.isArray(v))
        tab.resultView = objects ? 'table' : 'json'
        tab.resultTab = 'Result'
      } else if (value !== null && value !== undefined && typeof value === 'object') {
        tab.results = [value]
        tab.hasScalar = false
        tab.resultView = 'table'
        tab.resultTab = 'Result'
      } else {
        // scalar or undefined → no grid; show it in the Console
        tab.results = []
        tab.scalar = value
        tab.hasScalar = value !== undefined
        tab.resultTab = 'Console'
      }
    }
  } catch (e) {
    tab.elapsedMs = Date.now() - t0
    tab.runError = errMessage(e)
    tab.results = []
    tab.hasScalar = false
    tab.resultTab = 'Console'
  } finally {
    tab.isRunning = false
  }
}

// Run just the line under the cursor (toolbar button; mirrors ⌘⇧⏎).
function runCurrentLine() {
  if (!view) return
  const line = view.state.doc.lineAt(view.state.selection.main.head)
  run(line.text)
}

function openHistory() {
  if (historyMenu.value) { historyMenu.value = false; return }
  historyMenu.value = true
}
function applyHistory(cmd) {
  props.activeTab.code = cmd
  setEditorDoc(cmd)
  historyMenu.value = false
}

function formatScalar(value) {
  if (value === undefined) return ''
  try {
    return syntaxHighlight(mongoStringify(value))
  } catch (_) {
    return String(value)
  }
}
</script>

<template>
  <div class="shell">
    <!-- Toolbar -->
    <div class="shell-toolbar">
      <button class="qbtn run" @click="run()" :disabled="activeTab.isRunning">
        <BaseIcon name="run" :size="16" class="ic" /> Run <span class="kbd">⌘⏎</span>
      </button>
      <button class="qbtn" @click="runCurrentLine" :disabled="activeTab.isRunning" title="Run the line under the cursor">
        <BaseIcon name="run" :size="16" class="ic" /> Run line <span class="kbd">⌘⇧⏎</span>
      </button>
      <div class="tb-sep"></div>
      <div class="hist-wrap">
        <button class="qbtn" :class="{ on: historyMenu }" @click="openHistory">
          <BaseIcon name="history" :size="16" class="ic" /> History
        </button>
        <div v-if="historyMenu" class="hist-backdrop" @mousedown.self="historyMenu = false"></div>
        <div v-if="historyMenu" class="hist-menu">
          <div v-if="!activeTab.history.length" class="hist-empty">No history yet.</div>
          <div
            v-for="(cmd, i) in [...activeTab.history].reverse()"
            :key="i"
            class="hist-item"
            @click="applyHistory(cmd)"
          >{{ cmd }}</div>
        </div>
      </div>
      <span class="tb-spacer"></span>
      <span class="shell-db"><BaseIcon name="dbSmall" :size="14" /> {{ activeTab.dbName }}</span>
    </div>

    <!-- Editor: CodeMirror (JS highlighting + Mongo autocomplete) -->
    <div class="shell-editor">
      <div class="shell-cm" ref="editorEl"></div>
    </div>

    <!-- Results -->
    <div class="shell-results">
      <div class="rtabs">
        <button class="rtab" :class="{ active: activeTab.resultTab === 'Result' }"
          @click="activeTab.resultTab = 'Result'">Result</button>
        <button class="rtab" :class="{ active: activeTab.resultTab === 'Console' }"
          @click="activeTab.resultTab = 'Console'">Console</button>

        <span class="rtabs-spacer"></span>

        <!-- view switch (Result only) -->
        <div v-if="activeTab.resultTab === 'Result'" class="view-select-wrap">
          <span class="view-select" @click="viewMenu = !viewMenu">
            {{ viewLabel }} <BaseIcon name="caretDown" :size="12" />
          </span>
          <div v-if="viewMenu" class="view-menu">
            <div
              v-for="[k, label] in VIEWS"
              :key="k"
              class="view-menu-item"
              :class="{ on: activeTab.resultView === k }"
              @click="activeTab.resultView = k; viewMenu = false"
            >
              <BaseIcon v-if="activeTab.resultView === k" name="check" :size="13" />
              <span>{{ label }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Result tab -->
      <template v-if="activeTab.resultTab === 'Result'">
        <ResultTable
          v-if="activeTab.resultView === 'table'"
          :active-tab="activeTab"
          :readonly="true"
          v-model:drillPath="drillPath"
        />
        <div v-else-if="activeTab.resultView === 'json'" class="json-view">
          <div v-if="!resultCount" class="empty">No documents</div>
          <div v-else class="json-doc" v-for="(doc, i) in activeTab.results" :key="i"
            v-html="syntaxHighlight(mongoStringify(doc))"></div>
        </div>
        <div v-else class="tree-view">
          <div v-if="!resultCount" class="empty">No documents</div>
          <template v-else>
            <div class="tree-head">
              <span class="th-key">Key</span><span class="th-val">Value</span><span class="th-type">Type</span>
            </div>
            <div class="tree-body">
              <TreeView v-for="(doc, i) in activeTab.results" :key="i" :label="`(${i + 1})`" :value="doc" :depth="0" />
            </div>
          </template>
        </div>
      </template>

      <!-- Console tab -->
      <div v-else class="console">
        <div v-if="activeTab.elapsedMs != null" class="out-meta">
          → {{ activeTab.runError ? 'error' : (resultCount ? `${resultCount} document${resultCount === 1 ? '' : 's'}` : 'ok') }}
          · {{ (activeTab.elapsedMs / 1000).toFixed(3) }}s
        </div>
        <pre v-for="(line, i) in activeTab.logs" :key="'l' + i" class="c-log">{{ line }}</pre>
        <pre v-if="activeTab.runError" class="c-err">{{ activeTab.runError }}</pre>
        <pre v-else-if="activeTab.hasScalar" class="c-val" v-html="formatScalar(activeTab.scalar)"></pre>
        <div v-if="!activeTab.logs.length && !activeTab.runError && !activeTab.hasScalar && activeTab.elapsedMs == null"
          class="empty">Type a command and press ⌘⏎ / Ctrl+⏎ to run.</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.shell { flex: 1; display: flex; flex-direction: column; min-height: 0; background: var(--bg-window); }

/* toolbar (matches ui-design) */
.shell-toolbar {
  display: flex; align-items: center; gap: 2px;
  padding: 6px 10px; border-bottom: 1px solid var(--border);
  background: var(--bg-toolbar); flex: none;
}
.qbtn {
  display: flex; align-items: center; gap: 6px;
  padding: 5px 9px; border-radius: 6px;
  background: none; border: none; color: var(--text); font-size: 12.5px; cursor: pointer;
}
.qbtn:hover { background: var(--bg-hover); }
.qbtn.on { background: var(--bg-hover); }
.qbtn:disabled { opacity: .5; cursor: default; }
.qbtn .ic { color: var(--text-dim); }
.qbtn.run .ic { color: var(--green); }
.kbd {
  font-size: 10.5px; color: var(--text-faint);
  border: 1px solid var(--border-soft); border-radius: 4px; padding: 1px 5px; margin-left: 4px;
}
.tb-sep { width: 1px; align-self: stretch; background: var(--border-soft); margin: 4px 6px; }
.tb-spacer { flex: 1; }
.shell-db { display: flex; align-items: center; gap: 6px; color: var(--text-dim); font-size: 12.5px; padding: 5px 10px; }

/* history dropdown (mirrors QueryBar) */
.hist-wrap { position: relative; }
.hist-backdrop { position: fixed; inset: 0; z-index: 40; }
.hist-menu {
  position: absolute; top: calc(100% + 4px); left: 0; z-index: 41;
  width: 420px; max-height: 320px; overflow-y: auto;
  background: var(--bg-panel); border: 1px solid var(--border-soft);
  border-radius: 8px; padding: 4px; box-shadow: 0 8px 24px rgba(0,0,0,.4);
}
.hist-empty { padding: 12px; color: var(--text-faint); font-size: 12px; }
.hist-item {
  padding: 7px 9px; border-radius: 5px; cursor: pointer;
  font-family: var(--mono); font-size: 12px; color: var(--text);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.hist-item:hover { background: var(--bg-hover); }

/* editor: CodeMirror host (matches ui-design dimensions) */
.shell-editor {
  flex: none; display: flex; height: 180px; min-height: 90px; max-height: 320px;
  resize: vertical; overflow: hidden;
  border-bottom: 1px solid var(--border); background: var(--bg-window);
}
.shell-cm { flex: 1; min-width: 0; overflow: hidden; }
.shell-cm :deep(.cm-editor) { height: 100%; }
.shell-cm :deep(.cm-editor.cm-focused) { outline: none; }

/* results */
.shell-results { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.rtabs {
  display: flex; align-items: center;
  border-bottom: 1px solid var(--border); padding: 0 8px; flex: none;
}
.rtab {
  padding: 8px 16px; background: none; border: none; cursor: pointer;
  color: var(--text-dim); font-size: 12.5px; border-bottom: 2px solid transparent;
}
.rtab:hover { color: var(--text); }
.rtab.active { color: var(--text); border-bottom-color: var(--accent); }
.rtabs-spacer { flex: 1; }

.view-select-wrap { position: relative; }
.view-select {
  display: flex; align-items: center; gap: 5px; cursor: pointer;
  font-size: 12px; color: var(--text-dim); padding: 4px 6px;
}
.view-select:hover { color: var(--text); }
.view-menu {
  position: absolute; top: 100%; right: 0; z-index: 20;
  background: var(--bg-panel); border: 1px solid var(--border-soft);
  border-radius: 6px; padding: 4px; min-width: 130px; box-shadow: 0 8px 24px rgba(0,0,0,.4);
}
.view-menu-item {
  display: flex; align-items: center; gap: 6px; padding: 6px 8px;
  border-radius: 4px; font-size: 12px; color: var(--text); cursor: pointer;
}
.view-menu-item:hover { background: var(--bg-hover); }
.view-menu-item.on { color: var(--accent); }

.empty { padding: 32px; color: var(--text-faint); font-size: 12px; }

/* json view (matches ResultsPanel) */
.json-view { flex: 1; overflow: auto; padding: 12px 16px; }
.json-doc {
  font-family: var(--mono); font-size: 12.5px; line-height: 1.2; color: var(--text);
  white-space: pre; border-left: 2px solid var(--border-soft); padding: 8px 0 8px 14px; margin-bottom: 10px;
  -webkit-user-select: text; user-select: text;
}
.json-doc :deep(span) { -webkit-user-select: text; user-select: text; }
.json-doc :deep(.jk)  { color: var(--cell-key); }
.json-doc :deep(.jop) { color: var(--cell-op); }
.json-doc :deep(.js)  { color: var(--cell-str); }
.json-doc :deep(.jn)  { color: var(--cell-num); }
.json-doc :deep(.jb)  { color: var(--cell-num); }
.json-doc :deep(.jl)  { color: var(--text-faint); }
.json-doc :deep(.joid) { color: var(--link); }

/* tree view (matches ResultsPanel) */
.tree-view { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: auto; background: var(--bg-window); }
.tree-head {
  display: grid; grid-template-columns: minmax(220px, 1.4fr) minmax(160px, 2fr) 110px;
  position: sticky; top: 0; z-index: 2; height: 26px; align-items: center;
  background: var(--bg-toolbar); color: var(--text-dim); font-weight: 600; font-size: 11px;
  border-bottom: 1px solid var(--border);
}
.tree-head span { padding: 0 8px; border-right: 1px solid var(--border); height: 100%; display: flex; align-items: center; }
.tree-head .th-type { border-right: none; }
.tree-body { min-width: max-content; }

/* console */
.console {
  flex: 1; overflow: auto; padding: 10px 16px;
  font-family: var(--mono); font-size: 12.5px; line-height: 1.5;
}
.out-meta { color: var(--text-faint); font-size: 12px; margin-bottom: 12px; }
.c-log { color: var(--text-dim); white-space: pre-wrap; word-break: break-word; margin: 0 0 4px; }
.c-val { color: var(--text); white-space: pre-wrap; word-break: break-word; margin: 0; }
.c-err { color: var(--danger-text); white-space: pre-wrap; word-break: break-word; margin: 0; }
.console, .console :deep(span) { -webkit-user-select: text; user-select: text; }
.c-val :deep(.jk)  { color: var(--cell-key); }
.c-val :deep(.jop) { color: var(--cell-op); }
.c-val :deep(.js)  { color: var(--cell-str); }
.c-val :deep(.jn)  { color: var(--cell-num); }
.c-val :deep(.jb)  { color: var(--cell-num); }
.c-val :deep(.jl)  { color: var(--text-faint); }
.c-val :deep(.joid) { color: var(--link); }
</style>
