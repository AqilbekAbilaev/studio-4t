<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import { errMessage, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Top-bar "Data Masking" tool for the active collection. Lists the collection's
// fields (from a sample document) and lets the user pick a masking strategy per
// field, then exports an obfuscated copy — the source collection is never touched.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
const emit = defineEmits(['close', 'toast'])

const STRATEGIES = [
  { value: 'keep',    label: 'Keep' },
  { value: 'redact',  label: 'Redact' },
  { value: 'hash',    label: 'Hash' },
  { value: 'partial', label: 'Partial' },
  { value: 'nullify', label: 'Null' },
  { value: 'remove',  label: 'Remove' },
]

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const fields = ref([])          // [{ name, strategy, keepStart, keepEnd }]
const format = ref('json')
const limit = ref('')
const exporting = ref(false)

onMounted(async () => {
  try {
    const sample = await invoke('find_documents', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      filter: '{}',
      projection: '{}',
      sort: '{}',
      skip: 0,
      limit: 1,
    })
    if (sample && sample.length) {
      fields.value = Object.keys(sample[0]).map(name => ({
        name,
        strategy: 'keep',
        keepStart: 0,
        keepEnd: 4,
      }))
    }
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

const maskedCount = computed(() => fields.value.filter(f => f.strategy !== 'keep').length)

async function runExport() {
  error.value = null
  const rules = fields.value
    .filter(f => f.strategy !== 'keep')
    .map(f => {
      const rule = { field: f.name, strategy: f.strategy }
      if (f.strategy === 'partial') {
        rule.keepStart = Number(f.keepStart) || 0
        rule.keepEnd = Number(f.keepEnd) || 0
      }
      return rule
    })

  let path
  try {
    path = await saveDialog({
      defaultPath: `${props.target.collName}-masked.${format.value}`,
      filters: [{ name: format.value.toUpperCase(), extensions: [format.value] }],
    })
  } catch (_) {
    return
  }
  if (!path) return

  exporting.value = true
  try {
    const trimmed = String(limit.value).trim()
    const lim = trimmed ? Number(trimmed) : null
    const count = await invoke('export_masked_collection', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      filter: '{}',
      rules,
      path,
      format: format.value,
      limit: lim,
    })
    emit('toast', `Exported ${count} masked document${count === 1 ? '' : 's'}`)
    emit('close')
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Data Masking — {{ target.dbName }}.{{ target.collName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="mk-body">
        <StateMessage v-if="loading" mode="loading" label="Reading fields…" />
        <StateMessage
          v-else-if="error && !fields.length"
          mode="error"
          :message="error"
          :code="errorCode"
        />
        <StateMessage
          v-else-if="!fields.length"
          mode="empty"
          label="No documents to sample fields from"
        />
        <template v-else>
          <p class="mk-note">
            Choose how each field is obfuscated in the exported copy. The source
            collection is never modified.
          </p>

          <div class="mk-head">
            <span>Field</span>
            <span>Strategy</span>
            <span>Options</span>
          </div>
          <div class="mk-rows">
            <div v-for="f in fields" :key="f.name" class="mk-row">
              <code class="mk-field" :title="f.name">{{ f.name }}</code>
              <select v-model="f.strategy" class="mk-select">
                <option v-for="s in STRATEGIES" :key="s.value" :value="s.value">{{ s.label }}</option>
              </select>
              <span class="mk-opts">
                <template v-if="f.strategy === 'partial'">
                  keep
                  <input v-model="f.keepStart" type="number" min="0" class="mk-num" /> start
                  <input v-model="f.keepEnd" type="number" min="0" class="mk-num" /> end
                </template>
              </span>
            </div>
          </div>

          <StateMessage v-if="error && fields.length" mode="error" :message="error" :code="errorCode" />

          <div class="mk-footer">
            <label class="mk-f">
              Format
              <select v-model="format" class="mk-select">
                <option value="json">JSON</option>
                <option value="csv">CSV</option>
                <option value="xlsx">Excel (.xlsx)</option>
              </select>
            </label>
            <label class="mk-f">
              Limit
              <input v-model="limit" type="number" min="1" placeholder="all" class="mk-num wide" />
            </label>
            <span class="mk-summary">{{ maskedCount }} field{{ maskedCount === 1 ? '' : 's' }} masked</span>
            <button class="mk-export" :disabled="exporting" @click="runExport">
              {{ exporting ? 'Exporting…' : 'Export masked copy' }}
            </button>
          </div>
        </template>
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
  width: 640px;
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

.mk-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 200px;
  max-height: 74vh;
  overflow: hidden;
}
.mk-note { margin: 0; font-size: 12px; color: var(--text-dim); }
.mk-head {
  display: grid;
  grid-template-columns: 1fr 130px 1.2fr;
  gap: 10px;
  padding: 0 4px 6px;
  border-bottom: 1px solid var(--border-soft);
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.mk-rows { overflow-y: auto; display: flex; flex-direction: column; }
.mk-row {
  display: grid;
  grid-template-columns: 1fr 130px 1.2fr;
  gap: 10px;
  align-items: center;
  padding: 5px 4px;
  border-bottom: 1px solid var(--grid-line);
}
.mk-field {
  font-family: var(--mono);
  font-size: 12.5px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.mk-select {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 3px 6px;
  font-size: 12px;
}
.mk-opts { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 5px; }
.mk-num {
  width: 44px;
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 3px 5px;
  font-size: 12px;
}
.mk-num.wide { width: 64px; }
.mk-footer {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-top: 10px;
  border-top: 1px solid var(--border-soft);
}
.mk-f { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 6px; }
.mk-summary { font-size: 12px; color: var(--text-faint); margin-left: auto; }
.mk-export {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.mk-export:hover { background: var(--accent-soft); }
.mk-export:disabled { opacity: .6; cursor: default; }
</style>
