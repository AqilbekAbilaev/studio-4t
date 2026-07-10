<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../utils/errors'
import BaseIcon from './base/BaseIcon.vue'
import StateMessage from './base/StateMessage.vue'

// Opened from App.vue for a database node. Fetches `dbStats` once and surfaces the
// headline fields; the full document is available raw below.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const stats = ref(null)
const showRaw = ref(false)

onMounted(async () => {
  try {
    stats.value = await invoke('database_stats', {
      id: props.target.connId,
      database: props.target.dbName,
    })
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

function fmtBytes(bytes) {
  if (bytes == null) return '—'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let n = bytes
  let i = 0
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++ }
  return `${i === 0 ? n : n.toFixed(1)} ${units[i]}`
}

function fmtNum(n) {
  return n == null ? '—' : Number(n).toLocaleString()
}

// dbStats headline fields, guarded for servers that omit some.
const cards = computed(() => {
  const s = stats.value
  if (!s) return []
  return [
    { label: 'Database',     value: s.db || props.target.dbName || '—', icon: 'dbSmall' },
    { label: 'Collections',  value: fmtNum(s.collections),              icon: 'count' },
    { label: 'Objects',      value: fmtNum(s.objects),                  icon: 'count' },
    { label: 'Data Size',    value: fmtBytes(s.dataSize),               icon: 'count' },
    { label: 'Storage Size', value: fmtBytes(s.storageSize),            icon: 'count' },
    { label: 'Indexes',      value: fmtNum(s.indexes),                  icon: 'count' },
    { label: 'Index Size',   value: fmtBytes(s.indexSize),              icon: 'count' },
    { label: 'Avg Obj Size', value: fmtBytes(s.avgObjSize),             icon: 'count' },
  ]
})

const rawJson = computed(() =>
  stats.value ? JSON.stringify(stats.value, null, 2) : ''
)
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Database Statistics — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ss-body">
        <StateMessage v-if="loading" mode="loading" label="Fetching database statistics…" />
        <StateMessage
          v-else-if="error"
          mode="error"
          :message="error"
          :code="errorCode"
        />
        <template v-else>
          <div class="ss-grid">
            <div v-for="card in cards" :key="card.label" class="ss-card">
              <span class="ss-ic"><BaseIcon :name="card.icon" :size="15" /></span>
              <div class="ss-meta">
                <div class="ss-label">{{ card.label }}</div>
                <div class="ss-value">{{ card.value }}</div>
              </div>
            </div>
          </div>

          <button class="ss-raw-toggle" @click="showRaw = !showRaw">
            <BaseIcon :name="showRaw ? 'caretDown' : 'caret'" :size="12" />
            Raw dbStats
          </button>
          <pre v-if="showRaw" class="ss-raw">{{ rawJson }}</pre>
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
  width: 640px;
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

.ss-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.ss-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
}
.ss-ic { color: var(--text-faint); flex: none; }
.ss-meta { min-width: 0; }
.ss-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; }
.ss-value {
  font-size: 13.5px;
  color: var(--text);
  margin-top: 2px;
  word-break: break-word;
  user-select: text;
}

.ss-raw-toggle {
  align-self: flex-start;
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 12.5px;
  cursor: pointer;
  padding: 2px 0;
}
.ss-raw-toggle:hover { color: var(--text); }
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
