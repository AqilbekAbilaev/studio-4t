<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import { parseField } from '../../utils/queryParser'
import BaseIcon from '../base/BaseIcon.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'

// Add / Edit Validator for a collection. Fetches the current validator on open so an
// existing rule is never silently overwritten, then writes changes via collMod.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
const emit = defineEmits(['close', 'saved'])

const loading = ref(true)
const saving = ref(false)
const error = ref(null)
const validatorText = ref('')
const level = ref('strict')   // off | moderate | strict
const action = ref('error')   // error | warn
const LEVEL_OPTIONS = ['off', 'moderate', 'strict'].map((v) => ({ value: v, label: v }))
const ACTION_OPTIONS = ['error', 'warn'].map((v) => ({ value: v, label: v }))

onMounted(async () => {
  try {
    const info = await invoke('get_validator', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
    })
    validatorText.value = info.validator || ''
    if (info.validation_level) level.value = info.validation_level
    if (info.validation_action) action.value = info.validation_action
  } catch (e) {
    error.value = errText(e)
  } finally {
    loading.value = false
  }
})

async function save() {
  // Validate the validator document up front (empty clears the rule).
  const raw = validatorText.value.trim()
  let ejson = ''
  if (raw !== '' && raw !== '{}') {
    const pf = parseField(raw)
    if (!pf.ok) { error.value = pf.error; return }
    ejson = pf.ejson
  }
  saving.value = true
  error.value = null
  try {
    await invoke('set_validator', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      validator: ejson,
      validationLevel: level.value,
      validationAction: action.value,
    })
    emit('saved', props.target.collName)
    emit('close')
  } catch (e) {
    error.value = errText(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal :title="`Validator — ${target.collName}`" width="620px" max-width="92vw" @close="$emit('close')">

      <div class="vd-body">
        <StateMessage v-if="loading" mode="loading" label="Loading validator…" />
        <template v-else>
          <label class="vd-label">Validator (JSON schema document — leave empty to clear)</label>
          <textarea
            v-model="validatorText"
            class="vd-editor"
            spellcheck="false"
            placeholder='{ "$jsonSchema": { "bsonType": "object", "required": ["name"] } }'
          ></textarea>

          <div class="vd-row">
            <div class="vd-field">
              <label class="vd-label">Validation Level</label>
              <BaseSelect v-model="level" class="vd-select" :options="LEVEL_OPTIONS" />
            </div>
            <div class="vd-field">
              <label class="vd-label">Validation Action</label>
              <BaseSelect v-model="action" class="vd-select" :options="ACTION_OPTIONS" />
            </div>
          </div>

          <div v-if="error" class="vd-error">{{ error }}</div>
        </template>
      </div>

      <div class="vd-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" :disabled="loading || saving" @click="save">
          {{ saving ? 'Saving…' : 'Save' }}
        </button>
      </div>
    </BaseModal>
</template>

<style scoped>

.vd-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 160px;
  max-height: 70vh;
  overflow-y: auto;
}
.vd-label {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.vd-editor {
  width: 100%;
  min-height: 160px;
  box-sizing: border-box;
  padding: 10px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  resize: vertical;
}
.vd-editor:focus { outline: none; border-color: var(--accent); }
.vd-row { display: flex; gap: 12px; }
.vd-field { flex: 1; display: flex; flex-direction: column; gap: 6px; }
.vd-select { width: 100%; }
.vd-error { font-size: 12px; color: var(--danger-text); }

.vd-footer {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  gap: 8px;
}
.vd-footer .spacer { flex: 1; }
.btn {
  height: 30px;
  padding: 0 14px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
}
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:disabled { opacity: .55; cursor: default; }
</style>
