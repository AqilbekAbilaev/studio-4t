<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import StateMessage from './StateMessage.vue'

// Opened from App.vue for a connection node. Fetches admin `serverStatus` once
// and surfaces the headline fields; the full document is available raw below.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const status = ref(null)
const showRaw = ref(false)

onMounted(async () => {
  try {
    status.value = await invoke('server_status', { id: props.target.connId })
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

function fmtUptime(seconds) {
  if (seconds == null) return '—'
  const s = Math.floor(seconds)
  const d = Math.floor(s / 86400)
  const h = Math.floor((s % 86400) / 3600)
  const m = Math.floor((s % 3600) / 60)
  const parts = []
  if (d) parts.push(`${d}d`)
  if (h) parts.push(`${h}h`)
  if (m || (!d && !h)) parts.push(`${m}m`)
  return parts.join(' ')
}

function fmtMB(mb) {
  if (mb == null) return '—'
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  return `${mb} MB`
}

// serverStatus headline fields, guarded for servers that omit some sections.
const stats = computed(() => {
  const s = status.value
  if (!s) return []
  const conn = s.connections || {}
  const mem = s.mem || {}
  const net = s.network || {}
  return [
    { label: 'Host',              value: s.host || '—',                     icon: 'connect' },
    { label: 'Version',           value: s.version || '—',                  icon: 'dbSmall' },
    { label: 'Process',           value: s.process || '—',                  icon: 'shell' },
    { label: 'Uptime',            value: fmtUptime(s.uptime),               icon: 'clock' },
    { label: 'Current Conns',     value: conn.current ?? '—',               icon: 'connect' },
    { label: 'Available Conns',   value: conn.available ?? '—',             icon: 'connect' },
    { label: 'Resident Memory',   value: fmtMB(mem.resident),               icon: 'count' },
    { label: 'Virtual Memory',    value: fmtMB(mem.virtual),                icon: 'count' },
    { label: 'Network In',        value: net.bytesIn != null ? fmtMB(Math.round(net.bytesIn / 1048576)) : '—', icon: 'count' },
    { label: 'Network Out',       value: net.bytesOut != null ? fmtMB(Math.round(net.bytesOut / 1048576)) : '—', icon: 'count' },
  ]
})

const rawJson = computed(() =>
  status.value ? JSON.stringify(status.value, null, 2) : ''
)
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Server Status — {{ target.connName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ss-body">
        <StateMessage v-if="loading" mode="loading" label="Fetching server status…" />
        <StateMessage
          v-else-if="error"
          mode="error"
          :message="error"
          :code="errorCode"
        />
        <template v-else>
          <div class="ss-grid">
            <div v-for="stat in stats" :key="stat.label" class="ss-card">
              <span class="ss-ic"><BaseIcon :name="stat.icon" :size="15" /></span>
              <div class="ss-meta">
                <div class="ss-label">{{ stat.label }}</div>
                <div class="ss-value">{{ stat.value }}</div>
              </div>
            </div>
          </div>

          <button class="ss-raw-toggle" @click="showRaw = !showRaw">
            <BaseIcon :name="showRaw ? 'caretDown' : 'caret'" :size="12" />
            Raw serverStatus
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
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px #000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(#34363a, #2c2e31);
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
