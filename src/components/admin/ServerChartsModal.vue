<script setup>
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Live server metrics: polls serverStatus on an interval and draws simple SVG
// sparklines. Reuses the existing server_status command (no new backend).
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName }
})
defineEmits(['close'])

const MAX = 40           // samples kept per series
const INTERVAL = 2000    // poll every 2s

const loading = ref(true)
const error = ref(null)
const series = ref({
  connections: [],
  netIn: [],
  netOut: [],
  resident: [],
})
let lastNet = null       // { in, out } for computing per-interval deltas
let timer = null

function push(key, value) {
  const arr = series.value[key]
  arr.push(value)
  if (arr.length > MAX) arr.shift()
}

async function sample() {
  try {
    const s = await invoke('server_status', { id: props.target.connId })
    error.value = null
    const conn = s.connections || {}
    const net = s.network || {}
    const mem = s.mem || {}
    push('connections', conn.current ?? 0)
    push('resident', mem.resident ?? 0)
    if (lastNet) {
      push('netIn', Math.max(0, (net.bytesIn ?? 0) - lastNet.in))
      push('netOut', Math.max(0, (net.bytesOut ?? 0) - lastNet.out))
    }
    lastNet = { in: net.bytesIn ?? 0, out: net.bytesOut ?? 0 }
  } catch (e) {
    error.value = errText(e)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await sample()
  timer = setInterval(sample, INTERVAL)
})
onBeforeUnmount(() => { if (timer) clearInterval(timer) })

// Build an SVG polyline path for a series scaled into a 200x40 box.
function path(arr) {
  if (!arr.length) return ''
  const w = 200, h = 40
  const max = Math.max(1, ...arr)
  const step = arr.length > 1 ? w / (arr.length - 1) : w
  return arr.map((v, i) => `${i === 0 ? 'M' : 'L'} ${(i * step).toFixed(1)} ${(h - (v / max) * (h - 4) - 2).toFixed(1)}`).join(' ')
}

function last(arr) { return arr.length ? arr[arr.length - 1] : 0 }
function fmtBytes(b) {
  if (b < 1024) return `${b} B/s`
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB/s`
  return `${(b / 1048576).toFixed(1)} MB/s`
}

const charts = computed(() => [
  { label: 'Current connections', arr: series.value.connections, value: last(series.value.connections).toLocaleString() },
  { label: 'Network in',  arr: series.value.netIn,  value: fmtBytes(last(series.value.netIn)) },
  { label: 'Network out', arr: series.value.netOut, value: fmtBytes(last(series.value.netOut)) },
  { label: 'Resident memory', arr: series.value.resident, value: `${last(series.value.resident)} MB` },
])
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Server Status Charts — {{ target.connName }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>
      <div class="sc-body">
        <StateMessage v-if="loading" mode="loading" label="Sampling server status…" />
        <StateMessage v-else-if="error" mode="error" :message="error" />
        <template v-else>
          <div class="sc-hint">Live — sampled every {{ INTERVAL / 1000 }}s</div>
          <div class="sc-grid">
            <div v-for="c in charts" :key="c.label" class="sc-card">
              <div class="sc-head"><span class="sc-label">{{ c.label }}</span><span class="sc-value">{{ c.value }}</span></div>
              <svg class="sc-spark" viewBox="0 0 200 40" preserveAspectRatio="none">
                <path :d="path(c.arr)" fill="none" stroke="var(--accent)" stroke-width="1.5" />
              </svg>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 70; }
.dialog {
  width: 620px; max-width: 92vw; background: var(--bg-window); border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex; flex-direction: column; overflow: hidden;
}
.dlg-title {
  height: 36px; flex: none; background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border); display: flex; align-items: center; padding: 0 10px; position: relative;
}
.dlg-title .t { position: absolute; left: 0; right: 0; text-align: center; font-size: 13px; color: var(--text-dim); font-weight: 500; pointer-events: none; }
.close-btn { margin-left: auto; background: none; border: none; color: var(--text-faint); cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; z-index: 1; }
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
.sc-body { padding: 16px; min-height: 200px; display: flex; flex-direction: column; gap: 12px; }
.sc-hint { font-size: 11.5px; color: var(--text-faint); }
.sc-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.sc-card { background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; }
.sc-head { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 6px; }
.sc-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; }
.sc-value { font-size: 13px; color: var(--text); font-variant-numeric: tabular-nums; }
.sc-spark { width: 100%; height: 40px; display: block; }
</style>
