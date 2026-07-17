<script setup>
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import { EditorView } from '@codemirror/view'
import BaseIcon from '../base/BaseIcon.vue'
import CodeEditor from '../base/CodeEditor.vue'
import { docExtensions } from '../../utils/docEditor'
import { parseDocumentJson } from '../../utils/docJson'

const props = defineProps({
  mode:       { type: String, required: true },  // 'insert' | 'edit'
  initialDoc: { type: Object,  default: null },
  // Backend save error surfaced from the parent (insert/replace failure), shown inline.
  saveError:  { type: String,  default: null },
  // True while a save is in flight — disables the footer actions.
  saving:     { type: Boolean, default: false },
  // Bumped by the parent after a successful "Add & Continue" insert so this dialog can
  // clear its editor for the next document without closing.
  savedNonce: { type: Number,  default: 0 },
})
const emit = defineEmits(['close', 'save'])

const SKELETON = '{\n  \n}'

const text     = ref('')
const jsonErr  = ref(null)
const okMsg    = ref(null)   // transient "valid"/"added" confirmation
const wordWrap = ref(false)
const editorRef = ref(null)

const isInsert = computed(() => props.mode !== 'edit')

// Site-specific editor extensions: the shared document-editor niceties (bracket matching,
// close-brackets, active line, Mod-s → primary save) plus optional soft wrapping. A new
// array identity rebuilds the editor state, which is how the word-wrap toggle takes effect.
const editorExt = computed(() => {
  const ext = docExtensions({ onSave: () => submit(false) })
  if (wordWrap.value) ext.push(EditorView.lineWrapping)
  return ext
})

watch(() => props.initialDoc, (doc) => {
  text.value = doc ? JSON.stringify(doc, null, 2) : SKELETON
}, { immediate: true })

// After a successful "Add & Continue", reset to an empty document and confirm, staying
// open for the next insert.
watch(() => props.savedNonce, () => {
  text.value = SKELETON
  jsonErr.value = null
  okMsg.value = 'Document added — enter the next one'
  nextTick(() => editorRef.value?.focus())
})

// A backend error clears any stale "valid" confirmation.
watch(() => props.saveError, (e) => { if (e) okMsg.value = null })

onMounted(() => nextTick(() => editorRef.value?.focus()))

// Validate the buffer as a JSON document, returning the parsed object or null (and
// setting jsonErr) on failure. Shared by the Validate button and the save actions.
function validate() {
  jsonErr.value = null
  try {
    return parseDocumentJson(text.value)
  } catch (e) {
    jsonErr.value = e.message
    okMsg.value = null
    return null
  }
}

function onValidate() {
  if (validate() !== null) okMsg.value = 'Valid JSON document'
}

function onFormat() {
  const parsed = validate()
  if (parsed === null) return
  text.value = JSON.stringify(parsed, null, 2)
  okMsg.value = 'Formatted'
}

// keepOpen === true → "Add & Continue" (insert, then reset for the next document).
function submit(keepOpen) {
  if (props.saving) return
  if (validate() === null) return
  emit('save', { text: text.value, keepOpen: keepOpen })
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">

      <div class="dlg-title">
        <div class="t">{{ isInsert ? 'Insert JSON Document' : 'Edit JSON Document' }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="dm-body">
        <CodeEditor
          ref="editorRef"
          class="dm-editor"
          v-model="text"
          highlight="json"
          language="js"
          :extensions="editorExt"
        />
        <div v-if="jsonErr || saveError" class="dm-msg error">{{ jsonErr || saveError }}</div>
        <div v-else-if="okMsg" class="dm-msg ok">{{ okMsg }}</div>
      </div>

      <div class="dm-footer">
        <button class="btn" :disabled="saving" @click="onValidate">Validate JSON</button>
        <button class="btn" :disabled="saving" @click="onFormat">Format JSON</button>
        <label class="wrap-toggle">
          <input type="checkbox" v-model="wordWrap" />
          Enable word wrap
        </label>
        <span class="spacer"></span>
        <button v-if="isInsert" class="btn" :disabled="saving" @click="submit(true)">Add &amp; Continue</button>
        <button class="btn" :disabled="saving" @click="$emit('close')">Cancel</button>
        <button class="btn primary" :disabled="saving" @click="submit(false)">
          {{ isInsert ? 'Add Document' : 'Save' }}
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
  z-index: 60;
}

.dialog {
  width: 740px;
  max-width: 94vw;
  height: 520px;
  max-height: 92vh;
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

.dm-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 12px;
  gap: 8px;
}

.dm-editor {
  flex: 1;
  min-height: 0;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  overflow: hidden;
}

.dm-msg {
  font-size: 12px;
  padding: 2px 2px;
}
.dm-msg.error { color: var(--danger-text); }
.dm-msg.ok    { color: var(--success-text); }

.dm-footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 14px;
  gap: 6px;
}

.wrap-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 4px;
  flex: none;
  white-space: nowrap;
  font-size: 12.5px;
  color: var(--text-dim);
  cursor: pointer;
  user-select: none;
}
.wrap-toggle input { cursor: pointer; }

.spacer { flex: 1; }

.btn {
  height: 28px;
  padding: 0 12px;
  border-radius: 5px;
  border: none;
  flex: none;
  white-space: nowrap;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-toolbar);
  color: var(--text);
}
.btn:hover:not(:disabled) { background: var(--bg-hover); }
.btn:disabled { opacity: .5; cursor: default; }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover:not(:disabled) { opacity: .88; }
</style>
