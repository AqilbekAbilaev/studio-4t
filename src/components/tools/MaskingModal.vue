<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'
import HintText from '../base/HintText.vue'
import BaseModalBody from '../base/BaseModalBody.vue'
import BaseModalFoot from '../base/BaseModalFoot.vue'

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
const FORMAT_OPTIONS = [
  { value: 'json', label: 'JSON' },
  { value: 'csv',  label: 'CSV' },
  { value: 'xlsx', label: 'Excel (.xlsx)' },
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
    error.value = errText(e)
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
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <BaseModal :title="`Data Masking — ${target.dbName}.${target.collName}`" width="640px" max-width="92vw" @close="$emit('close')">

      <BaseModalBody>
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
          <HintText dim>
            Choose how each field is obfuscated in the exported copy. The source
            collection is never modified.
          </HintText>

          <div class="mk-head">
            <span>Field</span>
            <span>Strategy</span>
            <span>Options</span>
          </div>
          <div class="mk-rows">
            <div v-for="f in fields" :key="f.name" class="mk-row">
              <code class="mk-field" :title="f.name">{{ f.name }}</code>
              <BaseSelect v-model="f.strategy" class="mk-select" :options="STRATEGIES" size="sm" />
              <span class="mk-opts">
                <template v-if="f.strategy === 'partial'">
                  keep
                  <BaseInput v-model="f.keepStart" type="number" min="0" class="mk-num" /> start
                  <BaseInput v-model="f.keepEnd" type="number" min="0" class="mk-num" /> end
                </template>
              </span>
            </div>
          </div>

          <StateMessage v-if="error && fields.length" mode="error" :message="error" :code="errorCode" />
        </template>
      </BaseModalBody>

      <BaseModalFoot v-if="fields.length">
        <template #left>
          <label class="mk-f">
            Format
            <BaseSelect v-model="format" class="mk-select" :options="FORMAT_OPTIONS" size="sm" />
          </label>
          <label class="mk-f">
            Limit
            <BaseInput v-model="limit" type="number" min="1" placeholder="all" class="mk-num wide" />
          </label>
        </template>
        <span class="mk-summary">{{ maskedCount }} field{{ maskedCount === 1 ? '' : 's' }} masked</span>
        <BaseButton variant="primary" :disabled="exporting" @click="runExport">
          {{ exporting ? 'Exporting…' : 'Export masked copy' }}
        </BaseButton>
      </BaseModalFoot>
    </BaseModal>
</template>

<style scoped>


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
.mk-select { min-width: 120px; }
.mk-opts { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 5px; }
.base-input.mk-num {
  width: 44px;
  border-radius: 5px;
  padding: 3px 5px;
  font-size: 12px;
}
.base-input.mk-num.wide { width: 64px; }

.mk-f { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 6px; }
.mk-summary { font-size: 12px; color: var(--text-faint); margin-left: auto; }
</style>
