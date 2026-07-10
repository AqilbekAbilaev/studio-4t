<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Opened from App.vue for a database node. Reads the database's profiling status,
// lists the slow ops captured in `system.profile`, and lets the user change the
// profiling level / slow-op threshold. Profiling is per-database in MongoDB.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const status = ref(null)      // raw { profile: n, slowms: n } result
const entries = ref([])       // system.profile documents

// Control-bar state: the level/threshold the user is about to apply.
const level = ref(1)
const slowms = ref(100)
const applying = ref(false)

// Filter state for the list.
const slowerThan = ref(null)
const refreshing = ref(false)

// Which row is expanded to show its raw profile document.
const expanded = ref(null)

const LEVEL_LABELS = { 0: 'Off', 1: 'Slow ops', 2: 'All' }

const currentLevel = computed(() =>
  status.value && typeof status.value.was === 'number' ? status.value.was : null
)
const currentLevelLabel = computed(() =>
  currentLevel.value != null ? (LEVEL_LABELS[currentLevel.value] ?? currentLevel.value) : '—'
)

async function fetchStatus() {
  status.value = await invoke('get_profiling_status', {
    id: props.target.connId,
    database: props.target.dbName,
  })
  // Seed the control bar from the live status the first time we learn it.
  if (status.value && typeof status.value.was === 'number') {
    level.value = status.value.was
  }
  if (status.value && typeof status.value.slowms === 'number') {
    slowms.value = status.value.slowms
  }
}

async function fetchList() {
  entries.value = await invoke('list_profile', {
    id: props.target.connId,
    database: props.target.dbName,
    limit: 50,
    slowerThanMs: slowerThan.value != null && slowerThan.value !== '' ? Number(slowerThan.value) : null,
  })
}

onMounted(async () => {
  try {
    await fetchStatus()
    await fetchList()
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

async function applyLevel() {
  applying.value = true
  error.value = null
  errorCode.value = null
  try {
    await invoke('set_profiling_level', {
      id: props.target.connId,
      database: props.target.dbName,
      level: Number(level.value),
      slowms: Number(slowms.value),
    })
    await fetchStatus()
    await fetchList()
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    applying.value = false
  }
}

async function refreshList() {
  refreshing.value = true
  error.value = null
  errorCode.value = null
  try {
    await fetchList()
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    refreshing.value = false
  }
}

// The profile entries, normalized to the columns we show.
const rows = computed(() =>
  entries.value.map((op) => ({
    ts: formatTs(op.ts),
    op: op.op || '—',
    ns: op.ns || '—',
    millis: op.millis != null ? op.millis : '—',
    planSummary: op.planSummary || '—',
    docsExamined: op.docsExamined != null ? op.docsExamined : '—',
    nreturned: op.nreturned != null ? op.nreturned : '—',
    raw: op,
  }))
)

// Extended-JSON dates arrive as { $date: … }; render the plain string when we can.
function formatTs(ts) {
  if (ts == null) return '—'
  if (typeof ts === 'object' && ts.$date != null) {
    const value = ts.$date
    if (typeof value === 'string') return value
    if (value && typeof value === 'object' && value.$numberLong != null) {
      return new Date(Number(value.$numberLong)).toISOString()
    }
    return String(value)
  }
  return String(ts)
}

function toggleRow(i) {
  expanded.value = expanded.value === i ? null : i
}

function rawFor(op) {
  return JSON.stringify(op, null, 2)
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Query Profiler — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ss-body">
        <StateMessage v-if="loading" mode="loading" label="Reading profiling status…" />
        <template v-else>
          <div class="ctrl-bar">
            <span class="ctrl-label">Profiling</span>
            <span class="badge" :class="'lvl-' + (currentLevel ?? 'na')">{{ currentLevelLabel }}</span>
            <select v-model="level" class="ctrl-select">
              <option :value="0">Off</option>
              <option :value="1">Slow ops</option>
              <option :value="2">All</option>
            </select>
            <label class="ctrl-inline">
              slowms
              <input v-model="slowms" type="number" min="0" class="ctrl-num" />
            </label>
            <button class="btn" :disabled="applying" @click="applyLevel">
              {{ applying ? 'Applying…' : 'Apply' }}
            </button>
          </div>

          <div class="filter-bar">
            <label class="ctrl-inline">
              slower than
              <input v-model="slowerThan" type="number" min="0" class="ctrl-num" placeholder="—" />
              ms
            </label>
            <button class="btn" :disabled="refreshing" @click="refreshList">
              {{ refreshing ? 'Refreshing…' : 'Refresh' }}
            </button>
          </div>

          <StateMessage
            v-if="error"
            mode="error"
            :message="error"
            :code="errorCode"
          />

          <StateMessage
            v-if="!rows.length && !error"
            mode="empty"
            label="No profiled operations — enable profiling above to start capturing slow queries"
          />
          <table v-else-if="rows.length" class="ops-table">
            <thead>
              <tr>
                <th>Timestamp</th>
                <th>Op</th>
                <th>Namespace</th>
                <th>ms</th>
                <th>Plan</th>
                <th>Docs examined</th>
                <th>Returned</th>
              </tr>
            </thead>
            <tbody>
              <template v-for="(row, i) in rows" :key="i">
                <tr class="op-row" @click="toggleRow(i)">
                  <td>{{ row.ts }}</td>
                  <td>{{ row.op }}</td>
                  <td>{{ row.ns }}</td>
                  <td>{{ row.millis }}</td>
                  <td>{{ row.planSummary }}</td>
                  <td>{{ row.docsExamined }}</td>
                  <td>{{ row.nreturned }}</td>
                </tr>
                <tr v-if="expanded === i" :key="'raw-' + i">
                  <td colspan="7" class="raw-cell">
                    <pre class="ss-raw">{{ rawFor(row.raw) }}</pre>
                  </td>
                </tr>
              </template>
            </tbody>
          </table>
        </template>
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
  width: 860px;
  max-width: 92vw;
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

.ss-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 160px;
  max-height: 70vh;
  overflow-y: auto;
}

.ctrl-bar,
.filter-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.ctrl-label {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.ctrl-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text-dim);
}
.badge {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  color: var(--text-dim);
}
.badge.lvl-0 { color: var(--text-faint); }
.badge.lvl-1 { color: var(--accent); border-color: var(--accent); }
.badge.lvl-2 { color: var(--accent); border-color: var(--accent); }

.ctrl-select,
.ctrl-num {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 12.5px;
  padding: 4px 8px;
}
.ctrl-num { width: 84px; }

.btn {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 12.5px;
  padding: 5px 12px;
  cursor: pointer;
}
.btn:hover:not(:disabled) { background: var(--bg-hover); }
.btn:disabled { opacity: .55; cursor: default; }

.ops-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}
.ops-table th {
  text-align: left;
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
}
.ops-table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text);
  user-select: text;
  word-break: break-word;
}
.op-row { cursor: pointer; }
.op-row:hover td { background: var(--bg-hover); }
.raw-cell { padding: 0 8px 8px; }

.ss-raw {
  margin: 0;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  color: var(--text-dim);
  white-space: pre;
  overflow-x: auto;
  user-select: text;
}
</style>
