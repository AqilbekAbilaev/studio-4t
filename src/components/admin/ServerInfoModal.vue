<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Reused for the extra Server Info menu entries (Build Info / Host Info / Replica
// Set Status). Shows the flat scalar fields as cards and the full document raw.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, kind, title }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const data = ref(null)
const showRaw = ref(false)

onMounted(async () => {
  try {
    data.value = await invoke('server_info', { id: props.target.connId, kind: props.target.kind })
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

// Surface the flat, human-meaningful top-level scalars as cards; nested objects
// and arrays stay in the raw view (they vary a lot across server versions).
const cards = computed(() => {
  const d = data.value
  if (!d || typeof d !== 'object') return []
  const out = []
  for (const [key, value] of Object.entries(d)) {
    if (key === 'ok') continue
    const t = typeof value
    if (t === 'string' || t === 'number' || t === 'boolean') {
      out.push({ label: key, value: String(value) })
    }
  }
  return out
})

const rawJson = computed(() => (data.value ? JSON.stringify(data.value, null, 2) : ''))
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">{{ target.title }} — {{ target.connName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ss-body">
        <StateMessage v-if="loading" mode="loading" :label="`Fetching ${target.title.toLowerCase()}…`" />
        <StateMessage
          v-else-if="error"
          mode="error"
          :message="error"
          :code="errorCode"
        />
        <template v-else>
          <div v-if="cards.length" class="ss-grid">
            <div v-for="card in cards" :key="card.label" class="ss-card">
              <div class="ss-meta">
                <div class="ss-label">{{ card.label }}</div>
                <div class="ss-value">{{ card.value }}</div>
              </div>
            </div>
          </div>

          <button class="ss-raw-toggle" @click="showRaw = !showRaw">
            <BaseIcon :name="showRaw ? 'caretDown' : 'caret'" :size="12" />
            Raw response
          </button>
          <pre v-if="showRaw || !cards.length" class="ss-raw">{{ rawJson }}</pre>
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
.ss-meta { min-width: 0; }
.ss-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; }
.ss-value { font-size: 13.5px; color: var(--text); margin-top: 2px; word-break: break-word; user-select: text; }

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
