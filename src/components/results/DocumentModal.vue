<script setup>
import { ref, watch } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'

const props = defineProps({
  mode:       { type: String, required: true },  // 'insert' | 'edit'
  initialDoc: { type: Object, default: null },
})
const emit = defineEmits(['close', 'save'])

const text    = ref('')
const jsonErr = ref(null)

watch(() => props.initialDoc, (doc) => {
  text.value = doc ? JSON.stringify(doc, null, 2) : '{\n  \n}'
}, { immediate: true })

function onSave() {
  jsonErr.value = null
  let parsed
  try {
    parsed = JSON.parse(text.value)
  } catch (e) {
    jsonErr.value = `Invalid JSON: ${e.message}`
    return
  }
  if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
    jsonErr.value = 'Document must be a JSON object'
    return
  }
  emit('save', text.value)
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">

      <div class="dlg-title">
        <div class="t">{{ mode === 'edit' ? 'Edit Document' : 'Insert Document' }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="dm-body">
        <textarea
          class="dm-editor"
          v-model="text"
          spellcheck="false"
          autocomplete="off"
          autocorrect="off"
        />
        <div v-if="jsonErr" class="dm-error">{{ jsonErr }}</div>
      </div>

      <div class="dm-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" @click="onSave">
          {{ mode === 'edit' ? 'Save' : 'Insert' }}
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
  width: 680px;
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
  resize: none;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.6;
  padding: 10px 12px;
  outline: none;
}
.dm-editor:focus { border-color: var(--accent); }

.dm-error {
  font-size: 12px;
  color: var(--danger-text);
  padding: 2px 2px;
}

.dm-footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
}

.spacer { flex: 1; }

.btn {
  height: 28px;
  padding: 0 14px;
  border-radius: 5px;
  border: none;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-toolbar);
  color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover { opacity: .88; }
</style>
