<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import { parseField } from '../../utils/queryParser'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

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
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Validator — {{ target.collName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

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
              <select v-model="level" class="vd-select">
                <option value="off">off</option>
                <option value="moderate">moderate</option>
                <option value="strict">strict</option>
              </select>
            </div>
            <div class="vd-field">
              <label class="vd-label">Validation Action</label>
              <select v-model="action" class="vd-select">
                <option value="error">error</option>
                <option value="warn">warn</option>
              </select>
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
.vd-select {
  height: 30px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-size: 13px;
}
.vd-select:focus { outline: none; border-color: var(--accent); }
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
