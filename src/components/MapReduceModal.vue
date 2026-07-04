<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'

// Open Map-Reduce for a collection: enter map / reduce / (optional) finalize JS and
// an output collection (blank = inline), run mapReduce, and show the raw result.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
defineEmits(['close'])

const map = ref('function () {\n  emit(this.key, 1);\n}')
const reduce = ref('function (key, values) {\n  return Array.sum(values);\n}')
const finalize = ref('')
const outCollection = ref('')
const running = ref(false)
const error = ref(null)
const result = ref(null)

async function run() {
  running.value = true
  error.value = null
  result.value = null
  try {
    result.value = await invoke('map_reduce', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      map: map.value,
      reduce: reduce.value,
      finalize: finalize.value,
      outCollection: outCollection.value,
    })
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    running.value = false
  }
}

const resultJson = () => (result.value ? JSON.stringify(result.value, null, 2) : '')
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Map-Reduce — {{ target.collName }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="mr-body">
        <label class="mr-label">Map</label>
        <textarea v-model="map" class="mr-input mr-code" spellcheck="false"></textarea>
        <label class="mr-label">Reduce</label>
        <textarea v-model="reduce" class="mr-input mr-code" spellcheck="false"></textarea>
        <label class="mr-label">Finalize (optional)</label>
        <textarea v-model="finalize" class="mr-input mr-code short" spellcheck="false" placeholder="function (key, reducedValue) { … }"></textarea>
        <label class="mr-label">Output collection (blank = inline)</label>
        <input v-model="outCollection" class="mr-input" placeholder="e.g. mr_results" spellcheck="false" />

        <div v-if="error" class="mr-error">{{ error }}</div>
        <template v-if="result">
          <label class="mr-label">Result</label>
          <pre class="mr-result">{{ resultJson() }}</pre>
        </template>
      </div>

      <div class="mr-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Close</button>
        <button class="btn primary" :disabled="running || !map.trim() || !reduce.trim()" @click="run">
          {{ running ? 'Running…' : 'Run' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 70; }
.dialog {
  width: 640px; max-width: 92vw; background: var(--bg-window); border-radius: 10px;
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

.mr-body { padding: 14px 16px; display: flex; flex-direction: column; gap: 6px; max-height: 74vh; overflow-y: auto; }
.mr-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; margin-top: 6px; }
.mr-input {
  width: 100%; box-sizing: border-box; padding: 8px 10px; border-radius: 6px;
  border: 1px solid var(--border-soft); background: var(--bg-input); color: var(--text); font-size: 13px;
}
.mr-input:focus { outline: none; border-color: var(--accent); }
.mr-code { min-height: 84px; font-family: var(--mono); font-size: 12px; line-height: 1.5; resize: vertical; }
.mr-code.short { min-height: 48px; }
.mr-error { font-size: 12px; color: var(--danger-text); }
.mr-result {
  margin: 0; font-family: var(--mono); font-size: 12px; line-height: 1.5;
  background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px;
  padding: 10px 12px; color: var(--text-dim); white-space: pre; overflow-x: auto; user-select: text; max-height: 220px;
}
.mr-footer { display: flex; align-items: center; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--border); }
.mr-footer .spacer { flex: 1; }
.btn { height: 30px; padding: 0 14px; border-radius: 6px; border: 1px solid var(--border-soft); background: var(--bg-input); color: var(--text); font-size: 13px; cursor: pointer; }
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:disabled { opacity: .55; cursor: default; }
</style>
