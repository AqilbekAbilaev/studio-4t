<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseModal from '../base/BaseModal.vue'
import BaseButton from '../base/BaseButton.vue'

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
    error.value = errText(e)
  } finally {
    running.value = false
  }
}

const resultJson = () => (result.value ? JSON.stringify(result.value, null, 2) : '')
</script>

<template>
  <BaseModal :title="`Map-Reduce — ${target.collName}`" width="640px" max-width="92vw" @close="$emit('close')">

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
        <BaseButton bordered @click="$emit('close')">Close</BaseButton>
        <BaseButton variant="primary" :disabled="running || !map.trim() || !reduce.trim()" @click="run">
          {{ running ? 'Running…' : 'Run' }}
        </BaseButton>
      </div>
    </BaseModal>
</template>

<style scoped>

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
</style>
