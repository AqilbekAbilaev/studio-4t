<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'

// App preferences. Persisted via update_settings; on save the parent adopts the
// new default so newly opened collection tabs use it.
const props = defineProps({
  defaultQueryLimit: { type: Number, default: 50 },
  theme: { type: String, default: 'dark' },
})
const emit = defineEmits(['close', 'saved', 'open-shortcuts'])

const PAGE_SIZES = [10, 25, 50, 100, 200]
const limit = ref(props.defaultQueryLimit)
const theme = ref(props.theme)
const saving = ref(false)
const error = ref(null)

async function save() {
  saving.value = true
  error.value = null
  try {
    const settings = await invoke('update_settings', {
      defaultQueryLimit: Number(limit.value),
      theme: theme.value,
    })
    emit('saved', {
      defaultQueryLimit: Number(settings.default_query_limit),
      theme: settings.theme,
    })
    emit('close')
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Preferences</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="pf-body">
        <div class="pf-row">
          <div class="pf-meta">
            <div class="pf-label">Theme</div>
            <div class="pf-hint">Overall color scheme for the app.</div>
          </div>
          <select v-model="theme" class="pf-select">
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </div>

        <div class="pf-row">
          <div class="pf-meta">
            <div class="pf-label">Default query limit</div>
            <div class="pf-hint">Page size used when a collection is first opened.</div>
          </div>
          <select v-model.number="limit" class="pf-select">
            <option v-for="sz in PAGE_SIZES" :key="sz" :value="sz">{{ sz }}</option>
          </select>
        </div>

        <div class="pf-row">
          <div class="pf-meta">
            <div class="pf-label">Keyboard shortcuts</div>
            <div class="pf-hint">View the full list of shortcuts the app handles.</div>
          </div>
          <button class="pf-link" @click="$emit('open-shortcuts')">Open reference</button>
        </div>

        <div v-if="error" class="pf-error">{{ error }}</div>
      </div>

      <div class="pf-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" :disabled="saving" @click="save">Save</button>
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
  width: 480px;
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

.pf-body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.pf-row {
  display: flex;
  align-items: center;
  gap: 16px;
}
.pf-meta { flex: 1; min-width: 0; }
.pf-label { font-size: 13px; color: var(--text); }
.pf-hint { font-size: 12px; color: var(--text-faint); margin-top: 2px; }

.pf-select {
  flex: none;
  height: 30px;
  min-width: 80px;
  padding: 0 8px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 13px;
}
.pf-link {
  flex: none;
  height: 30px;
  padding: 0 12px;
  background: var(--bg-toolbar);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 12.5px;
  cursor: pointer;
}
.pf-link:hover { background: var(--bg-hover); }

.pf-error { font-size: 12.5px; color: var(--danger-text); }

.pf-footer {
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
.btn:disabled { opacity: .5; cursor: default; }
</style>
