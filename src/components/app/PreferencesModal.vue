<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseSelect from '../base/BaseSelect.vue'
import BaseModal from '../base/BaseModal.vue'
import BaseButton from '../base/BaseButton.vue'

// App preferences. Persisted via update_settings; on save the parent adopts the
// new default so newly opened collection tabs use it.
const props = defineProps({
  defaultQueryLimit: { type: Number, default: 50 },
  theme: { type: String, default: 'dark' },
})
const emit = defineEmits(['close', 'saved', 'open-shortcuts'])

const PAGE_SIZES = [10, 25, 50, 100, 200]
const THEME_OPTIONS = [{ value: 'dark', label: 'Dark' }, { value: 'light', label: 'Light' }]
const pageSizeOptions = PAGE_SIZES.map((sz) => ({ value: sz, label: String(sz) }))
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
    error.value = errText(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal title="Preferences" width="480px" max-width="92vw" @close="$emit('close')">

      <div class="pf-body">
        <div class="pf-row">
          <div class="pf-meta">
            <div class="pf-label">Theme</div>
            <div class="pf-hint">Overall color scheme for the app.</div>
          </div>
          <BaseSelect v-model="theme" class="pf-select" :options="THEME_OPTIONS" />
        </div>

        <div class="pf-row">
          <div class="pf-meta">
            <div class="pf-label">Default query limit</div>
            <div class="pf-hint">Page size used when a collection is first opened.</div>
          </div>
          <BaseSelect v-model="limit" class="pf-select" :options="pageSizeOptions" />
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
        <BaseButton @click="$emit('close')">Cancel</BaseButton>
        <BaseButton variant="primary" :disabled="saving" @click="save">Save</BaseButton>
      </div>
    </BaseModal>
</template>

<style scoped>

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

.pf-select { flex: none; min-width: 120px; }
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
</style>
