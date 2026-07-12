<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Opened from a collection node's "Collection Stats" action. Fetches collStats
// and surfaces the headline numbers plus a per-index size breakdown, the way
// Studio-3T's Collection Stats view does.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const stats = ref(null)
const showRaw = ref(false)

onMounted(async () => {
  try {
    stats.value = await invoke('collection_stats', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
    })
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

function fmtBytes(bytes) {
  if (bytes == null) return '—'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let i = 0
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024
    i++
  }
  return `${value.toFixed(value >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

function fmtNum(n) {
  if (n == null) return '—'
  return n.toLocaleString()
}

const cards = computed(() => {
  const s = stats.value
  if (!s) return []
  return [
    { label: 'Documents',        value: fmtNum(s.count),            icon: 'count' },
    { label: 'Data Size',        value: fmtBytes(s.size),           icon: 'dbSmall' },
    { label: 'Avg Document',     value: fmtBytes(s.avg_obj_size),   icon: 'count' },
    { label: 'Storage Size',     value: fmtBytes(s.storage_size),   icon: 'dbSmall' },
    { label: 'Indexes',          value: fmtNum(s.nindexes),         icon: 'expr' },
    { label: 'Total Index Size', value: fmtBytes(s.total_index_size), icon: 'expr' },
  ]
})

const rawJson = computed(() => (stats.value ? JSON.stringify(stats.value.raw, null, 2) : ''))
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">
          Collection Stats — {{ target.dbName }}.{{ target.collName }}
          <span v-if="stats && stats.capped" class="ss-tag">capped</span>
        </div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ss-body">
        <StateMessage v-if="loading" mode="loading" label="Fetching collection stats…" />
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

          <template v-if="stats.indexes && stats.indexes.length">
            <div class="ss-section">Indexes</div>
            <div class="ss-idx">
              <div v-for="ix in stats.indexes" :key="ix.name" class="ss-idx-row">
                <code class="ss-idx-name">{{ ix.name }}</code>
                <span class="ss-idx-size">{{ fmtBytes(ix.size) }}</span>
              </div>
            </div>
          </template>

          <button class="ss-raw-toggle" @click="showRaw = !showRaw">
            <BaseIcon :name="showRaw ? 'caretDown' : 'caret'" :size="12" />
            Raw collStats
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
  width: 620px;
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
.ss-tag {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--warn);
  border: 1px solid var(--warn);
  border-radius: 4px;
  padding: 0 4px;
  margin-left: 6px;
  vertical-align: middle;
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
  max-height: 72vh;
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
.ss-value { font-size: 14px; color: var(--text); margin-top: 2px; user-select: text; }

.ss-section {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
  border-bottom: 1px solid var(--border-soft);
  padding-bottom: 5px;
}
.ss-idx { display: flex; flex-direction: column; }
.ss-idx-row {
  display: flex;
  justify-content: space-between;
  padding: 5px 4px;
  border-bottom: 1px solid var(--grid-line);
  font-size: 12.5px;
}
.ss-idx-name { font-family: var(--mono); color: var(--text); }
.ss-idx-size { color: var(--text-dim); }

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
