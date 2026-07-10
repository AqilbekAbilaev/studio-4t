<script setup>
// Small dialog behind the Document menu's field editors:
//   mode 'edit'   → change a field's value and BSON type
//   mode 'add'    → add a new field (name + type + value)
//   mode 'rename' → rename a field (name only)
// It builds the Extended-JSON value with the shared docEdit helpers and validates
// locally; the parent performs the actual document mutation + save and can feed a
// backend error back in via `saveError` (the dialog stays open on error).
import { ref, computed, watch } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'
import { BSON_TYPES, buildTypedValue } from '../../utils/docEdit'

const props = defineProps({
  mode:        { type: String, required: true },   // 'edit' | 'add' | 'rename'
  fieldName:   { type: String, default: '' },      // current name (edit / rename)
  initialType: { type: String, default: 'String' },
  initialRaw:  { type: String, default: '' },
  saveError:   { type: String, default: null },    // error from the parent's save
})
const emit = defineEmits(['close', 'save'])

const name  = ref(props.fieldName)
const type  = ref(props.initialType)
const raw   = ref(props.initialRaw)
const localError = ref(null)

// Re-seed if the dialog is reused for a different field without unmounting.
watch(() => [props.fieldName, props.initialType, props.initialRaw], () => {
  name.value = props.fieldName
  type.value = props.initialType
  raw.value  = props.initialRaw
})

const title = computed(() => ({
  edit:   'Edit Value / Type',
  add:    'Add Field / Value',
  rename: 'Rename Field',
}[props.mode]))

const showName  = computed(() => props.mode === 'add' || props.mode === 'rename')
const showValue = computed(() => props.mode === 'add' || props.mode === 'edit')
// Null and JSON aside, the value input is a single line; JSON gets a textarea.
const useTextarea = computed(() => showValue.value && type.value === 'JSON')
const showValueInput = computed(() => showValue.value && type.value !== 'Null')

function onSave() {
  localError.value = null
  if (props.mode === 'rename') {
    const trimmed = name.value.trim()
    if (!trimmed) { localError.value = 'Enter a new field name'; return }
    emit('save', { name: trimmed })
    return
  }
  if (props.mode === 'add') {
    if (!name.value.trim()) { localError.value = 'Enter a field name'; return }
  }
  let value
  try {
    value = buildTypedValue(type.value, raw.value)
  } catch (e) {
    localError.value = e.message
    return
  }
  emit('save', { name: props.mode === 'edit' ? props.fieldName : name.value.trim(), value: value })
}

const shownError = computed(() => localError.value || props.saveError)
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">{{ title }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="fe-body">
        <div v-if="mode === 'edit'" class="fe-field-label">Field: <code>{{ fieldName }}</code></div>

        <label v-if="showName" class="fe-row">
          <span class="fe-lbl">{{ mode === 'rename' ? 'New name' : 'Field name' }}</span>
          <input class="fe-input" v-model="name" spellcheck="false" autocomplete="off"
                 @keydown.enter="onSave" />
        </label>

        <label v-if="showValue" class="fe-row">
          <span class="fe-lbl">Type</span>
          <select class="fe-input" v-model="type">
            <option v-for="t in BSON_TYPES" :key="t" :value="t">{{ t }}</option>
          </select>
        </label>

        <label v-if="showValueInput" class="fe-row">
          <span class="fe-lbl">Value</span>
          <textarea v-if="useTextarea" class="fe-input fe-area" v-model="raw" spellcheck="false"
                    autocomplete="off"></textarea>
          <input v-else class="fe-input" v-model="raw" spellcheck="false" autocomplete="off"
                 @keydown.enter="onSave" />
        </label>

        <div v-if="shownError" class="fe-error">{{ shownError }}</div>
      </div>

      <div class="fe-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" @click="onSave">Save</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 60; }
.dialog {
  width: 460px; max-width: 94vw;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex; flex-direction: column; overflow: hidden;
}
.dlg-title {
  height: 36px; flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 10px; position: relative;
}
.dlg-title .t {
  position: absolute; left: 0; right: 0; text-align: center;
  font-size: 13px; color: var(--text-dim); font-weight: 500; pointer-events: none;
}
.close-btn {
  margin-left: auto; background: none; border: none; color: var(--text-faint);
  cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
.fe-body { padding: 16px 18px 8px; display: flex; flex-direction: column; gap: 12px; }
.fe-field-label { font-size: 12.5px; color: var(--text-dim); }
.fe-field-label code { font-family: var(--mono); color: var(--text); }
.fe-row { display: flex; flex-direction: column; gap: 5px; }
.fe-lbl { font-size: 11px; text-transform: uppercase; letter-spacing: .4px; color: var(--text-faint); }
.fe-input {
  background: var(--bg-input); border: 1px solid var(--border); border-radius: 5px;
  color: var(--text); font-family: var(--mono); font-size: 12.5px; padding: 7px 9px; outline: none;
}
.fe-input:focus { border-color: var(--accent); }
.fe-area { resize: vertical; min-height: 96px; line-height: 1.5; }
.fe-error { font-size: 12px; color: var(--danger-text); }
.fe-footer {
  height: 48px; flex: none; border-top: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 16px; gap: 8px; margin-top: 8px;
}
.spacer { flex: 1; }
.btn {
  height: 28px; padding: 0 14px; border-radius: 5px; border: none;
  font-size: 13px; cursor: pointer; background: var(--bg-toolbar); color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover { opacity: .88; }
</style>
