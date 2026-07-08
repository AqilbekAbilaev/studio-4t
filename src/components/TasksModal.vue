<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errMessage } from '../utils/errors'
import { scheduleSummary } from '../utils/taskSchedule'
import BaseIcon from './BaseIcon.vue'
import StateMessage from './StateMessage.vue'

// The Tasks panel: saved, parameterised invocations of an existing operation
// (Export / Import / Data Masking / SQL Migration / IntelliShell Script) that the
// user can run on demand or on a schedule. This step is the list surface — run,
// delete, and live-refresh on the backend's `task-ran` event. Creating and editing
// tasks is added in the next step.
const emit = defineEmits(['close', 'toast'])

// Maps a task spec's `kind` to a display label + BaseIcon name.
const TYPE_META = {
  export:    { label: 'Export',              icon: 'export' },
  import:    { label: 'Import',              icon: 'import' },
  masking:   { label: 'Data Masking',        icon: 'mask' },
  migration: { label: 'SQL Migration',       icon: 'migration' },
  shell:     { label: 'IntelliShell Script', icon: 'shell' },
}

const tasks = ref([])
const loading = ref(true)
const error = ref(null)
// Ids of tasks whose Run now is in flight, so their button shows a busy state and
// can't be double-clicked.
const running = ref(new Set())

let unlisten = null

onMounted(async () => {
  await refresh()
  // The scheduler emits `task-ran` whenever it fires a task in the background; keep
  // the list (last run / status) current and toast the outcome.
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

function taskName(id) {
  const found = tasks.value.find(t => t.id === id)
  return found ? found.name : null
}

function typeMeta(task) {
  return TYPE_META[task.spec && task.spec.kind] || { label: task.spec?.kind || 'Task', icon: 'tasks' }
}

function summaryOf(task) {
  return scheduleSummary(task.schedule)
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
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Tasks</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="tk-body">
        <StateMessage v-if="error" mode="error" :message="error" />

        <div v-if="loading" class="tk-empty">Loading…</div>

        <div v-else-if="!tasks.length" class="tk-empty">
          <BaseIcon name="tasks" :size="26" />
          <p>No tasks yet.</p>
          <span class="tk-empty-sub">Saved Export, Import, Masking, SQL Migration and IntelliShell operations appear here.</span>
        </div>

        <ul v-else class="tk-list">
          <li v-for="task in tasks" :key="task.id" class="tk-row">
            <div class="tk-icon"><BaseIcon :name="typeMeta(task).icon" :size="16" /></div>
            <div class="tk-main">
              <div class="tk-name">{{ task.name }}</div>
              <div class="tk-meta">
                <span class="tk-type">{{ typeMeta(task).label }}</span>
                <span class="tk-dot">·</span>
                <span class="tk-sched"><BaseIcon name="clock" :size="11" /> {{ summaryOf(task) }}</span>
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
              <button class="tk-icon-btn" title="Delete" @click="remove(task)">
                <BaseIcon name="trash" :size="14" />
              </button>
            </div>
          </li>
        </ul>
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
  gap: 10px;
  max-height: 74vh;
  overflow-y: auto;
}

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
.tk-empty-sub { font-size: 11.5px; max-width: 380px; }

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
.tk-icon {
  flex: none;
  color: var(--text-dim);
  display: flex;
  align-items: center;
}
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

.tk-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}
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
.tk-icon-btn:hover { background: var(--bg-hover); color: var(--err, #e06060); }
</style>
