<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseModal from '../base/BaseModal.vue'
import BaseModalBody from '../base/BaseModalBody.vue'
import BaseModalFoot from '../base/BaseModalFoot.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import BaseTextarea from '../base/BaseTextarea.vue'
import FieldError from '../base/FieldError.vue'

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

      <BaseModalBody>
        <label class="mr-label">Map</label>
        <BaseTextarea v-model="map" class="mr-code" spellcheck="false"></BaseTextarea>
        <label class="mr-label">Reduce</label>
        <BaseTextarea v-model="reduce" class="mr-code" spellcheck="false"></BaseTextarea>
        <label class="mr-label">Finalize (optional)</label>
        <BaseTextarea v-model="finalize" class="mr-code short" spellcheck="false" placeholder="function (key, reducedValue) { … }"></BaseTextarea>
        <label class="mr-label">Output collection (blank = inline)</label>
        <BaseInput v-model="outCollection" placeholder="e.g. mr_results" spellcheck="false" />

        <FieldError :text="error" />
        <template v-if="result">
          <label class="mr-label">Result</label>
          <pre class="mr-result">{{ resultJson() }}</pre>
        </template>
      </BaseModalBody>

      <BaseModalFoot>
        <BaseButton bordered @click="$emit('close')">Close</BaseButton>
        <BaseButton variant="primary" :disabled="running || !map.trim() || !reduce.trim()" @click="run">
          {{ running ? 'Running…' : 'Run' }}
        </BaseButton>
      </BaseModalFoot>
    </BaseModal>
</template>

<style scoped>

.mr-label { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; margin-top: 6px; }
.base-textarea.mr-code { min-height: 84px; }
.base-textarea.mr-code.short { min-height: 48px; }
.mr-result {
  margin: 0; font-family: var(--mono); font-size: 12px; line-height: 1.5;
  background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px;
  padding: 10px 12px; color: var(--text-dim); white-space: pre; overflow-x: auto; user-select: text; max-height: 220px;
}

</style>
