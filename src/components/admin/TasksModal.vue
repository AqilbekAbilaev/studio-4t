<script setup>
import { ref, computed, reactive, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { save as saveDialog, open as openDialog } from '@tauri-apps/plugin-dialog'
import { errMessage } from '../../utils/errors'
import { scheduleSummary } from '../../utils/taskSchedule'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// The Tasks panel: saved, parameterised invocations of an existing operation
// (Export / Import / Data Masking / SQL Migration / IntelliShell Script) that the
// user can run on demand or on a schedule. Two views share this modal: the list of
// saved tasks (run / edit / delete, live-refreshed on the backend `task-ran` event)
// and the create/edit form.
const emit = defineEmits(['close', 'toast'])

// The five task types that map to an existing operation command, plus the two
// "coming soon" rows (no backing operation yet) shown disabled in the picker.
const TYPES = [
  { value: 'export',    label: 'Export',              icon: 'export',    enabled: true },
  { value: 'import',    label: 'Import',              icon: 'import',    enabled: true },
  { value: 'masking',   label: 'Data Masking',        icon: 'mask',      enabled: true },
  { value: 'migration', label: 'SQL Migration',       icon: 'migration', enabled: true },
  { value: 'shell',     label: 'IntelliShell Script', icon: 'shell',     enabled: true },
  { value: 'reschema',  label: 'Reschema',            icon: 'reschema',  enabled: false },
  { value: 'compare',   label: 'Data Compare & Sync', icon: 'compare',   enabled: false },
]
const TYPE_META = Object.fromEntries(TYPES.map(t => [t.value, t]))

const MASK_STRATEGIES = ['redact', 'hash', 'partial', 'nullify', 'remove']
const WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']

const view = ref('list')            // 'list' | 'form'
const tasks = ref([])
const connections = ref([])
const databases = ref([])           // DatabaseInfo[] for the selected connection
const loading = ref(true)
const error = ref(null)
const saving = ref(false)
// Ids of tasks whose Run now is in flight, so their button shows a busy state.
const running = ref(new Set())

let unlisten = null

// The create/edit form. `id` empty means create.
const form = reactive(blankForm())

function blankForm() {
  return {
    id: '',
    name: '',
    // Server-owned fields round-tripped on edit so saving a definition change never
    // clobbers the task's creation time or last-run state (upsert replaces the whole
    // record). Blank/null on create — the backend stamps created_at.
    createdAt: '',
    lastRun: null,
    lastStatus: null,
    type: 'export',
    connId: '',
    database: '',
    collection: '',
    path: '',
    format: 'json',
    filter: '{}',
    limit: '',
    tableName: '',
    code: '',
    rules: [],
    schedKind: 'manual',            // 'manual' | 'interval' | 'daily' | 'weekly'
    everyMinutes: 60,
    atHHMM: '09:00',
    weekday: 1,
  }
}

const dialogTitle = computed(() => {
  if (view.value === 'form') return form.id ? 'Edit Task' : 'New Task'
  return 'Tasks'
})

// Whether the currently selected type needs a collection (shell scripts don't).
const needsCollection = computed(() => form.type !== 'shell')

onMounted(async () => {
  await Promise.all([refresh(), loadConnections()])
  // The scheduler emits `task-ran` when it fires a task in the background; keep the
  // list current and toast the outcome.
  unlisten = await listen('task-ran', (event) => {
    refresh()
    const payload = event.payload
    if (payload && payload.run) {
      const name = taskName(payload.task_id)
      emit('toast', `${name || 'Task'} ran: ${payload.run.status}`)
    }
  })
})

onBeforeUnmount(() => {
  if (unlisten) unlisten()
})

async function refresh() {
  try {
    tasks.value = await invoke('list_tasks')
    error.value = null
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    loading.value = false
  }
}

async function loadConnections() {
  try {
    connections.value = await invoke('list_connections')
  } catch (_) {
    connections.value = []
  }
}

// Try to fetch databases + collections for the selected connection so the form can
// offer suggestions. Best-effort: on failure the user just types names by hand.
async function loadDatabases() {
  databases.value = []
  if (!form.connId) return
  try {
    databases.value = await invoke('list_databases', { id: form.connId })
  } catch (_) {
    databases.value = []
  }
}

const collectionOptions = computed(() => {
  const db = databases.value.find(d => d.name === form.database)
  return db ? db.collections : []
})

function taskName(id) {
  const found = tasks.value.find(t => t.id === id)
  return found ? found.name : null
}

function typeMeta(task) {
  const kind = task.spec && task.spec.kind
  return TYPE_META[kind] || { label: kind || 'Task', icon: 'tasks' }
}

// Epoch-ms string -> short local date/time, or an em dash when never run.
function lastRunLabel(task) {
  if (!task.last_run) return '—'
  const ms = Number(task.last_run)
  if (!Number.isFinite(ms)) return '—'
  return new Date(ms).toLocaleString()
}

async function runNow(task) {
  if (running.value.has(task.id)) return
  running.value = new Set(running.value).add(task.id)
  try {
    const run = await invoke('run_task', { id: task.id })
    emit('toast', `${task.name}: ${run.status === 'ok' ? 'ran successfully' : run.message}`)
    await refresh()
  } catch (e) {
    emit('toast', `${task.name} failed: ${errMessage(e)}`)
  } finally {
    const next = new Set(running.value)
    next.delete(task.id)
    running.value = next
  }
}

async function remove(task) {
  try {
    await invoke('delete_task', { id: task.id })
    await refresh()
    emit('toast', `Deleted "${task.name}"`)
  } catch (e) {
    emit('toast', `Could not delete: ${errMessage(e)}`)
  }
}

// ── Form ──────────────────────────────────────────────────────────────────

function startCreate() {
  Object.assign(form, blankForm())
  databases.value = []
  view.value = 'form'
}

async function startEdit(task) {
  const next = blankForm()
  next.id = task.id
  next.name = task.name
  next.createdAt = task.created_at
  next.lastRun = task.last_run
  next.lastStatus = task.last_status
  next.connId = task.connection_id
  const spec = task.spec || {}
  next.type = spec.kind || 'export'
  next.database = spec.database || ''
  next.collection = spec.collection || ''
  next.path = spec.path || ''
  next.format = spec.format || 'json'
  next.filter = spec.filter || '{}'
  next.limit = spec.limit == null ? '' : String(spec.limit)
  next.tableName = spec.table_name || ''
  next.code = spec.code || ''
  next.rules = (spec.rules || []).map(r => ({
    field: r.field || '',
    strategy: r.strategy || 'redact',
    keepStart: r.keep_start == null ? '' : String(r.keep_start),
    keepEnd: r.keep_end == null ? '' : String(r.keep_end),
    maskChar: r.mask_char || '',
    replacement: r.replacement || '',
  }))
  const sched = task.schedule
  if (sched && sched.kind) {
    next.schedKind = sched.kind
    if (sched.every_minutes != null) next.everyMinutes = sched.every_minutes
    if (sched.at_hhmm) next.atHHMM = sched.at_hhmm
    if (sched.weekday != null) next.weekday = sched.weekday
  }
  Object.assign(form, next)
  view.value = 'form'
  await loadDatabases()
}

function cancelForm() {
  view.value = 'list'
}

function pickType(type) {
  const meta = TYPE_META[type]
  if (!meta || !meta.enabled) return
  form.type = type
}

function addRule() {
  form.rules.push({ field: '', strategy: 'redact', keepStart: '4', keepEnd: '0', maskChar: '', replacement: '' })
}
function removeRule(index) {
  form.rules.splice(index, 1)
}

async function browseOutput() {
  try {
    const chosen = await saveDialog({ defaultPath: form.path || undefined })
    if (chosen) form.path = chosen
  } catch (_) {}
}
async function browseInput() {
  try {
    const chosen = await openDialog({ multiple: false, defaultPath: form.path || undefined })
    if (typeof chosen === 'string') form.path = chosen
  } catch (_) {}
}

function numOrNull(value) {
  if (value === '' || value == null) return null
  const n = Number(value)
  return Number.isFinite(n) ? n : null
}

function buildRules() {
  return form.rules
    .filter(r => r.field.trim())
    .map(r => {
      const rule = { field: r.field.trim(), strategy: r.strategy }
      if (r.strategy === 'partial') {
        rule.keep_start = numOrNull(r.keepStart) ?? 0
        rule.keep_end = numOrNull(r.keepEnd) ?? 0
      }
      if (r.maskChar) rule.mask_char = r.maskChar
      if (r.replacement) rule.replacement = r.replacement
      return rule
    })
}

function buildSpec() {
  const database = form.database.trim()
  const collection = form.collection.trim()
  switch (form.type) {
    case 'export':
      return { kind: 'export', database, collection, path: form.path, format: form.format }
    case 'import':
      return { kind: 'import', database, collection, path: form.path, format: form.format }
    case 'masking':
      return {
        kind: 'masking', database, collection,
        filter: form.filter.trim() || '{}',
        rules: buildRules(),
        path: form.path, format: form.format,
        limit: numOrNull(form.limit),
      }
    case 'migration':
      return {
        kind: 'migration', database, collection,
        table_name: form.tableName.trim() ? form.tableName.trim() : null,
        limit: numOrNull(form.limit),
        path: form.path,
      }
    case 'shell':
      return { kind: 'shell', database, code: form.code }
    default:
      return null
  }
}

function buildSchedule() {
  switch (form.schedKind) {
    case 'interval':
      return { kind: 'interval', every_minutes: numOrNull(form.everyMinutes) || 1 }
    case 'daily':
      return { kind: 'daily', at_hhmm: form.atHHMM || '09:00' }
    case 'weekly':
      return { kind: 'weekly', weekday: Number(form.weekday), at_hhmm: form.atHHMM || '09:00' }
    default:
      return null
  }
}

// Return an error string if the form is incomplete, else null.
function validate() {
  if (!form.name.trim()) return 'Give the task a name'
  if (!form.connId) return 'Choose a connection'
  if (!form.database.trim()) return 'Choose a database'
  if (needsCollection.value && !form.collection.trim()) return 'Choose a collection'
  if (form.type === 'shell' && !form.code.trim()) return 'Enter a script to run'
  if ((form.type === 'export' || form.type === 'import' || form.type === 'masking' || form.type === 'migration') && !form.path.trim()) {
    return 'Choose a file path'
  }
  if (form.schedKind === 'interval' && !(numOrNull(form.everyMinutes) > 0)) {
    return 'Interval must be at least 1 minute'
  }
  return null
}

async function save() {
  const problem = validate()
  if (problem) {
    emit('toast', problem)
    return
  }
  const spec = buildSpec()
  if (!spec) {
    emit('toast', 'That task type is not available yet')
    return
  }
  const task = {
    id: form.id || '',
    name: form.name.trim(),
    connection_id: form.connId,
    spec,
    schedule: buildSchedule(),
    created_at: form.createdAt || '',
    last_run: form.lastRun ?? null,
    last_status: form.lastStatus ?? null,
  }
  saving.value = true
  try {
    await invoke('save_task', { task })
    await refresh()
    view.value = 'list'
    emit('toast', form.id ? 'Task updated' : 'Task created')
  } catch (e) {
    emit('toast', `Could not save: ${errMessage(e)}`)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">{{ dialogTitle }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <!-- LIST VIEW -->
      <div v-if="view === 'list'" class="tk-body">
        <div class="tk-toolbar">
          <button class="tk-new" @click="startCreate">
            <BaseIcon name="plus" :size="13" /> New Task
          </button>
        </div>

        <StateMessage v-if="error" mode="error" :message="error" />

        <div v-if="loading" class="tk-empty">Loading…</div>

        <div v-else-if="!tasks.length" class="tk-empty">
          <BaseIcon name="tasks" :size="26" />
          <p>No tasks yet.</p>
          <span class="tk-empty-sub">Create a task to save an Export, Import, Masking, SQL Migration or IntelliShell operation you can run on demand or on a schedule.</span>
        </div>

        <ul v-else class="tk-list">
          <li v-for="task in tasks" :key="task.id" class="tk-row">
            <div class="tk-icon"><BaseIcon :name="typeMeta(task).icon" :size="16" /></div>
            <div class="tk-main">
              <div class="tk-name">{{ task.name }}</div>
              <div class="tk-meta">
                <span class="tk-type">{{ typeMeta(task).label }}</span>
                <span class="tk-dot">·</span>
                <span class="tk-sched"><BaseIcon name="clock" :size="11" /> {{ scheduleSummary(task.schedule) }}</span>
              </div>
            </div>
            <div class="tk-status">
              <span v-if="task.last_status" :class="['tk-badge', task.last_status]">{{ task.last_status }}</span>
              <span class="tk-lastrun">{{ lastRunLabel(task) }}</span>
            </div>
            <div class="tk-actions">
              <button
                class="tk-btn run"
                :disabled="running.has(task.id)"
                :title="running.has(task.id) ? 'Running…' : 'Run now'"
                @click="runNow(task)"
              >
                <BaseIcon name="run" :size="13" />
                {{ running.has(task.id) ? 'Running…' : 'Run now' }}
              </button>
              <button class="tk-icon-btn" title="Edit" @click="startEdit(task)">
                <BaseIcon name="edit" :size="14" />
              </button>
              <button class="tk-icon-btn danger" title="Delete" @click="remove(task)">
                <BaseIcon name="trash" :size="14" />
              </button>
            </div>
          </li>
        </ul>
      </div>

      <!-- FORM VIEW -->
      <div v-else class="tk-body">
        <!-- Type picker -->
        <label class="tk-lbl">Task type</label>
        <div class="tk-types">
          <button
            v-for="t in TYPES"
            :key="t.value"
            :class="['tk-type-card', { active: form.type === t.value, disabled: !t.enabled }]"
            :disabled="!t.enabled"
            :title="t.enabled ? '' : 'Coming soon'"
            @click="pickType(t.value)"
          >
            <BaseIcon :name="t.icon" :size="16" />
            <span>{{ t.label }}</span>
            <span v-if="!t.enabled" class="tk-soon">soon</span>
          </button>
        </div>

        <label class="tk-lbl">Name</label>
        <input v-model="form.name" class="tk-input" placeholder="e.g. Nightly orders export" />

        <label class="tk-lbl">Connection</label>
        <select v-model="form.connId" class="tk-input" @change="loadDatabases">
          <option value="" disabled>Select a connection…</option>
          <option v-for="c in connections" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>

        <div class="tk-two">
          <div class="tk-col">
            <label class="tk-lbl">Database</label>
            <input v-model="form.database" list="tk-dblist" class="tk-input" placeholder="database" />
            <datalist id="tk-dblist">
              <option v-for="d in databases" :key="d.name" :value="d.name" />
            </datalist>
          </div>
          <div class="tk-col" v-if="needsCollection">
            <label class="tk-lbl">Collection</label>
            <input v-model="form.collection" list="tk-colllist" class="tk-input" placeholder="collection" />
            <datalist id="tk-colllist">
              <option v-for="c in collectionOptions" :key="c" :value="c" />
            </datalist>
          </div>
        </div>

        <!-- Export / Import: format + path -->
        <template v-if="form.type === 'export' || form.type === 'import'">
          <label class="tk-lbl">Format</label>
          <select v-model="form.format" class="tk-input">
            <option value="json">JSON</option>
            <option value="csv">CSV</option>
          </select>
          <label class="tk-lbl">{{ form.type === 'import' ? 'Source file' : 'Destination file' }}</label>
          <div class="tk-path">
            <input v-model="form.path" class="tk-input" :placeholder="form.type === 'import' ? '/path/to/input' : '/path/to/output'" />
            <button class="tk-browse" @click="form.type === 'import' ? browseInput() : browseOutput()">Browse…</button>
          </div>
        </template>

        <!-- Masking: filter, limit, rules, format, path -->
        <template v-else-if="form.type === 'masking'">
          <label class="tk-lbl">Filter (EJSON)</label>
          <textarea v-model="form.filter" class="tk-input mono" rows="2" spellcheck="false" placeholder="{}"></textarea>
          <div class="tk-rules-head">
            <label class="tk-lbl">Masking rules</label>
            <button class="tk-addrule" @click="addRule"><BaseIcon name="plus" :size="11" /> Add rule</button>
          </div>
          <div v-for="(rule, i) in form.rules" :key="i" class="tk-rule">
            <input v-model="rule.field" class="tk-input" placeholder="field.path" />
            <select v-model="rule.strategy" class="tk-input narrow">
              <option v-for="s in MASK_STRATEGIES" :key="s" :value="s">{{ s }}</option>
            </select>
            <template v-if="rule.strategy === 'partial'">
              <input v-model="rule.keepStart" class="tk-input tiny" placeholder="start" title="Keep first N chars" />
              <input v-model="rule.keepEnd" class="tk-input tiny" placeholder="end" title="Keep last N chars" />
            </template>
            <button class="tk-icon-btn danger" title="Remove rule" @click="removeRule(i)"><BaseIcon name="trash" :size="13" /></button>
          </div>
          <div class="tk-two">
            <div class="tk-col">
              <label class="tk-lbl">Format</label>
              <select v-model="form.format" class="tk-input">
                <option value="json">JSON</option>
                <option value="csv">CSV</option>
              </select>
            </div>
            <div class="tk-col">
              <label class="tk-lbl">Limit (optional)</label>
              <input v-model="form.limit" class="tk-input" placeholder="e.g. 1000" />
            </div>
          </div>
          <label class="tk-lbl">Destination file</label>
          <div class="tk-path">
            <input v-model="form.path" class="tk-input" placeholder="/path/to/output" />
            <button class="tk-browse" @click="browseOutput">Browse…</button>
          </div>
        </template>

        <!-- Migration: table name, limit, path -->
        <template v-else-if="form.type === 'migration'">
          <div class="tk-two">
            <div class="tk-col">
              <label class="tk-lbl">Table name (optional)</label>
              <input v-model="form.tableName" class="tk-input" :placeholder="form.collection || 'table'" />
            </div>
            <div class="tk-col">
              <label class="tk-lbl">Sample limit (optional)</label>
              <input v-model="form.limit" class="tk-input" placeholder="e.g. 1000" />
            </div>
          </div>
          <label class="tk-lbl">Destination .sql file</label>
          <div class="tk-path">
            <input v-model="form.path" class="tk-input" placeholder="/path/to/migration.sql" />
            <button class="tk-browse" @click="browseOutput">Browse…</button>
          </div>
        </template>

        <!-- Shell: code -->
        <template v-else-if="form.type === 'shell'">
          <label class="tk-lbl">Script</label>
          <textarea v-model="form.code" class="tk-input mono" rows="5" spellcheck="false" placeholder="db.collection.find({}).count()"></textarea>
        </template>

        <!-- Schedule -->
        <label class="tk-lbl">Schedule</label>
        <select v-model="form.schedKind" class="tk-input">
          <option value="manual">Manual (run on demand only)</option>
          <option value="interval">Every N minutes</option>
          <option value="daily">Daily at a time</option>
          <option value="weekly">Weekly on a day</option>
        </select>
        <div v-if="form.schedKind === 'interval'" class="tk-sched-row">
          <span>Every</span>
          <input v-model="form.everyMinutes" class="tk-input tiny" />
          <span>minutes</span>
        </div>
        <div v-else-if="form.schedKind === 'daily'" class="tk-sched-row">
          <span>At</span>
          <input v-model="form.atHHMM" type="time" class="tk-input narrow" />
        </div>
        <div v-else-if="form.schedKind === 'weekly'" class="tk-sched-row">
          <span>On</span>
          <select v-model="form.weekday" class="tk-input narrow">
            <option v-for="(d, i) in WEEKDAYS" :key="i" :value="i">{{ d }}</option>
          </select>
          <span>at</span>
          <input v-model="form.atHHMM" type="time" class="tk-input narrow" />
        </div>

        <div class="tk-form-actions">
          <button class="tk-cancel" @click="cancelForm">Cancel</button>
          <button class="tk-save" :disabled="saving" @click="save">
            <BaseIcon name="save" :size="13" /> {{ saving ? 'Saving…' : (form.id ? 'Save changes' : 'Create task') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 70;
}
.dialog {
  width: 720px;
  max-width: 94vw;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
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

.tk-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 76vh;
  overflow-y: auto;
}

.tk-toolbar { display: flex; justify-content: flex-end; }
.tk-new {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 12.5px;
  cursor: pointer;
}
.tk-new:hover { background: var(--accent-soft); }

.tk-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  color: var(--text-faint);
  padding: 40px 20px;
  text-align: center;
}
.tk-empty p { margin: 0; font-size: 13px; color: var(--text-dim); }
.tk-empty-sub { font-size: 11.5px; max-width: 420px; }

.tk-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.tk-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
}
.tk-icon { flex: none; color: var(--text-dim); display: flex; align-items: center; }
.tk-main { flex: 1; min-width: 0; }
.tk-name {
  font-size: 13px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tk-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
  font-size: 11.5px;
  color: var(--text-faint);
}
.tk-sched { display: inline-flex; align-items: center; gap: 4px; }
.tk-dot { opacity: .5; }

.tk-status {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 3px;
  flex: none;
}
.tk-badge {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: .04em;
  padding: 1px 6px;
  border-radius: 4px;
}
.tk-badge.ok { background: var(--ok-soft, rgba(60,180,90,.18)); color: var(--ok, #4caf6a); }
.tk-badge.error { background: var(--err-soft, rgba(210,70,70,.18)); color: var(--err, #e06060); }
.tk-lastrun { font-size: 11px; color: var(--text-faint); }

.tk-actions { display: flex; align-items: center; gap: 6px; flex: none; }
.tk-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 5px 11px;
  font-size: 12px;
  cursor: pointer;
}
.tk-btn:hover:not(:disabled) { background: var(--accent-soft); }
.tk-btn:disabled { opacity: .6; cursor: default; }
.tk-icon-btn {
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-faint);
  border-radius: 6px;
  padding: 5px;
  cursor: pointer;
  display: flex;
  align-items: center;
}
.tk-icon-btn:hover { background: var(--bg-hover); color: var(--text); }
.tk-icon-btn.danger:hover { color: var(--err, #e06060); }

/* Form */
.tk-lbl {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
  margin-top: 4px;
}
.tk-input {
  width: 100%;
  box-sizing: border-box;
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 9px;
  font-size: 12.5px;
}
.tk-input:focus { outline: none; border-color: var(--accent); }
.tk-input.mono { font-family: var(--mono); line-height: 1.5; resize: vertical; }
.tk-input.narrow { width: auto; min-width: 120px; }
.tk-input.tiny { width: 74px; }

.tk-two { display: flex; gap: 10px; }
.tk-col { flex: 1; display: flex; flex-direction: column; gap: 4px; min-width: 0; }

.tk-types {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 6px;
}
.tk-type-card {
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 7px;
  padding: 8px 10px;
  font-size: 12px;
  cursor: pointer;
  position: relative;
}
.tk-type-card:hover:not(.disabled) { background: var(--bg-hover); }
.tk-type-card.active {
  border-color: var(--accent);
  color: var(--text);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.tk-type-card.disabled { opacity: .5; cursor: not-allowed; }
.tk-soon {
  margin-left: auto;
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--text-faint);
  border: 1px solid var(--border-soft);
  border-radius: 4px;
  padding: 0 4px;
}

.tk-path { display: flex; gap: 8px; }
.tk-browse {
  flex: none;
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 6px;
  padding: 0 12px;
  font-size: 12px;
  cursor: pointer;
}
.tk-browse:hover { background: var(--bg-hover); color: var(--text); }

.tk-rules-head { display: flex; align-items: center; justify-content: space-between; margin-top: 4px; }
.tk-addrule {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 5px;
  padding: 3px 8px;
  font-size: 11.5px;
  cursor: pointer;
}
.tk-addrule:hover { background: var(--bg-hover); color: var(--text); }
.tk-rule { display: flex; gap: 6px; align-items: center; }

.tk-sched-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: var(--text-dim);
}

.tk-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-soft);
}
.tk-cancel {
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.tk-cancel:hover { background: var(--bg-hover); color: var(--text); }
.tk-save {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12.5px;
  cursor: pointer;
}
.tk-save:hover:not(:disabled) { background: var(--accent-soft); }
.tk-save:disabled { opacity: .6; cursor: default; }
</style>
