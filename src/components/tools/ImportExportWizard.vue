<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import BaseCheckbox from '../base/BaseCheckbox.vue'
import ReorderButtons from '../base/ReorderButtons.vue'

const IMPORT_FORMATS = [
  { value: 'json', label: 'JSON' },
  { value: 'csv',  label: 'CSV' },
]
const EXPORT_FORMATS = [
  { value: 'json', label: 'JSON' },
  { value: 'csv',  label: 'CSV' },
  { value: 'xlsx', label: 'Excel (.xlsx)' },
]

// Stepped Import / Export wizard for a single collection. One component, two
// modes:
//   import — pick a file, detect its columns, map each column to a target field
//            with a type coercion, preview, then run (`import_collection_mapped`).
//   export — sample the collection, choose/reorder/rename the fields (optionally
//            coercing), pick a format, preview, then run (`export_collection_fields`).
const props = defineProps({
  mode: { type: String, required: true },     // 'import' | 'export'
  target: { type: Object, required: true },   // { connId, connName, dbName, collName }
})
const emit = defineEmits(['close', 'toast', 'done'])

const isImport = computed(() => props.mode === 'import')

// The per-field type coercions the backend understands. 'auto' keeps the value
// as parsed (no coercion).
const KINDS = [
  { value: 'auto', label: 'Auto' },
  { value: 'string', label: 'String' },
  { value: 'int', label: 'Int32' },
  { value: 'long', label: 'Int64' },
  { value: 'double', label: 'Double' },
  { value: 'bool', label: 'Boolean' },
  { value: 'date', label: 'Date' },
  { value: 'objectId', label: 'ObjectId' },
]

const PREVIEW_LIMIT = 20

const step = ref(0)
const steps = computed(() =>
  isImport.value ? ['Source', 'Map fields', 'Preview & run'] : ['Select fields', 'Preview & run']
)

const loading = ref(false)
const error = ref(null)
const errorCode = ref(null)
const running = ref(false)

// import: chosen file + format; export: chosen output format.
const filePath = ref('')
const format = ref('json')
// Export only documents added since this collection's last incremental export (tracked
// by _id on the backend).
const incremental = ref(false)

// [{ source, target, kind, include }] — the mapping the user edits.
const fields = ref([])
// Sample rows (objects keyed by source column) for the preview.
const sampleRows = ref([])

const includedFields = computed(() =>
  fields.value.filter(f => f.include && String(f.target).trim() !== '')
)

onMounted(() => {
  if (!isImport.value) loadCollectionSample()
})

function setError(e) {
  error.value = errText(e)
  errorCode.value = errCode(e)
}

// ── export: sample the collection to discover fields ───────────
async function loadCollectionSample() {
  loading.value = true
  error.value = null
  try {
    const docs = await invoke('find_documents', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      filter: '{}',
      projection: '{}',
      sort: '{}',
      skip: 0,
      limit: PREVIEW_LIMIT,
    })
    sampleRows.value = docs || []
    const cols = []
    for (const doc of sampleRows.value) {
      for (const key of Object.keys(doc)) {
        if (!cols.includes(key)) cols.push(key)
      }
    }
    fields.value = cols.map(name => ({ source: name, target: name, kind: 'auto', include: true }))
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

// ── import: pick a file, then detect its columns ───────────────
async function pickFile() {
  let path
  try {
    path = await openDialog({
      multiple: false,
      filters: [{ name: 'JSON or CSV', extensions: ['json', 'csv'] }],
    })
  } catch (e) {
    setError(e)
    return
  }
  if (!path) return
  filePath.value = String(path)
  format.value = filePath.value.toLowerCase().endsWith('.csv') ? 'csv' : 'json'
}

async function detectColumns() {
  if (!filePath.value) return
  loading.value = true
  error.value = null
  try {
    const preview = await invoke('import_preview', {
      path: filePath.value,
      format: format.value,
      limit: PREVIEW_LIMIT,
    })
    sampleRows.value = preview.rows || []
    fields.value = (preview.columns || []).map(name => ({
      source: name,
      target: name,
      kind: 'auto',
      include: true,
    }))
    if (!fields.value.length) {
      setError('No columns were detected in this file')
      return
    }
    step.value = 1
  } catch (e) {
    setError(e)
  } finally {
    loading.value = false
  }
}

// ── field reordering (export) ──────────────────────────────────
function moveField(index, delta) {
  const next = index + delta
  if (next < 0 || next >= fields.value.length) return
  const arr = fields.value
  const tmp = arr[index]
  arr[index] = arr[next]
  arr[next] = tmp
}

// ── preview table ──────────────────────────────────────────────
// Columns shown in the preview = the target names of the included fields.
const previewColumns = computed(() => includedFields.value.map(f => f.target))

// Row objects rebuilt as { targetName: sourceValue } so the preview reflects
// renaming/selection. (Type coercion is applied server-side on run.)
const previewRows = computed(() =>
  sampleRows.value.map(row => {
    const out = {}
    for (const f of includedFields.value) out[f.target] = row[f.source]
    return out
  })
)

function cellText(value) {
  if (value === null || value === undefined) return ''
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

// ── navigation ─────────────────────────────────────────────────
const canGoNext = computed(() => {
  if (isImport.value && step.value === 0) return !!filePath.value
  return includedFields.value.length > 0
})

async function next() {
  error.value = null
  // Import step 0 → detect (which advances on success).
  if (isImport.value && step.value === 0) {
    await detectColumns()
    return
  }
  if (step.value < steps.value.length - 1) step.value += 1
}

function back() {
  error.value = null
  if (step.value > 0) step.value -= 1
}

// The mapping payload sent to the backend.
function mappingPayload() {
  return includedFields.value.map(f => ({
    source: f.source,
    target: String(f.target).trim(),
    kind: f.kind,
  }))
}

// ── run ────────────────────────────────────────────────────────
async function run() {
  if (isImport.value) return runImport()
  return runExport()
}

async function runImport() {
  running.value = true
  error.value = null
  try {
    const count = await invoke('import_collection_mapped', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      path: filePath.value,
      format: format.value,
      mapping: mappingPayload(),
    })
    emit('toast', `Imported ${count} document${count === 1 ? '' : 's'}`)
    emit('done', props.target.connId)
    emit('close')
  } catch (e) {
    setError(e)
  } finally {
    running.value = false
  }
}

async function runExport() {
  let path
  try {
    path = await saveDialog({
      defaultPath: `${props.target.collName}.${format.value}`,
      filters: [{ name: format.value.toUpperCase(), extensions: [format.value] }],
    })
  } catch (e) {
    setError(e)
    return
  }
  if (!path) return
  running.value = true
  error.value = null
  try {
    const count = await invoke('export_collection_fields', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      path: String(path),
      format: format.value,
      fields: mappingPayload(),
      incremental: incremental.value,
    })
    emit('toast', `Exported ${count} document${count === 1 ? '' : 's'} to ${format.value.toUpperCase()}`)
    emit('close')
  } catch (e) {
    setError(e)
  } finally {
    running.value = false
  }
}

const isLastStep = computed(() => step.value === steps.value.length - 1)
const titleText = computed(
  () => `${isImport.value ? 'Import' : 'Export'} — ${props.target.dbName}.${props.target.collName}`
)
</script>

<template>
  <BaseModal :title="`${titleText}`" width="720px" max-width="94vw" @close="$emit('close')">

      <!-- step indicator -->
      <div class="iew-steps">
        <span
          v-for="(label, i) in steps"
          :key="label"
          class="iew-step"
          :class="{ active: i === step, done: i < step }"
        >
          <span class="iew-dot">{{ i + 1 }}</span>{{ label }}
        </span>
      </div>

      <div class="iew-body">
        <StateMessage v-if="loading" mode="loading" label="Working…" />
        <StateMessage
          v-else-if="error && !fields.length"
          mode="error"
          :message="error"
          :code="errorCode"
        />

        <!-- IMPORT step 0: source -->
        <template v-else-if="isImport && step === 0">
          <p class="iew-note">Choose a JSON or CSV file to import.</p>
          <div class="iew-source">
            <BaseButton bordered @click="pickFile">Choose file…</BaseButton>
            <code class="iew-path" :title="filePath">{{ filePath || 'No file selected' }}</code>
          </div>
          <label class="iew-f">
            Format
            <BaseSelect v-model="format" class="iew-select" :options="IMPORT_FORMATS" size="sm" />
          </label>
        </template>

        <!-- Field mapping (import step 1 / export step 0) -->
        <template v-else-if="(isImport && step === 1) || (!isImport && step === 0)">
          <p class="iew-note">
            {{ isImport
              ? 'Map each detected column to a target field and pick its type.'
              : 'Choose which fields to export, rename or reorder them, and optionally coerce a type.' }}
          </p>
          <div class="iew-head">
            <span></span>
            <span>{{ isImport ? 'Source column' : 'Field' }}</span>
            <span>{{ isImport ? 'Target field' : 'Export as' }}</span>
            <span>Type</span>
            <span v-if="!isImport">Order</span>
          </div>
          <div class="iew-rows">
            <div v-for="(f, i) in fields" :key="f.source" class="iew-row">
              <BaseCheckbox v-model="f.include" class="iew-chk" />
              <code class="iew-field" :title="f.source">{{ f.source }}</code>
              <BaseInput v-model="f.target" class="iew-input" :disabled="!f.include" />
              <BaseSelect v-model="f.kind" class="iew-select" :options="KINDS" :disabled="!f.include" size="sm" />
              <span v-if="!isImport" class="iew-order">
                <ReorderButtons
                  :up-disabled="i === 0"
                  :down-disabled="i === fields.length - 1"
                  @up="moveField(i, -1)"
                  @down="moveField(i, 1)"
                />
              </span>
            </div>
          </div>
        </template>

        <!-- Preview & run (last step) -->
        <template v-else-if="isLastStep">
          <div class="iew-preview-top">
            <p class="iew-note">
              Preview of the first {{ previewRows.length }} row{{ previewRows.length === 1 ? '' : 's' }}.
              <template v-if="isImport"> Types are applied on import.</template>
            </p>
            <div v-if="!isImport" class="iew-export-opts">
              <label class="iew-f">
                Format
                <BaseSelect v-model="format" class="iew-select" :options="EXPORT_FORMATS" size="sm" />
              </label>
              <label class="iew-f iew-inc" title="Export only documents added since this collection's last incremental export (tracked by _id)">
                <BaseCheckbox v-model="incremental" />
                Incremental (new only)
              </label>
            </div>
          </div>
          <div class="iew-table-wrap">
            <table class="iew-table" v-if="previewColumns.length">
              <thead>
                <tr><th v-for="c in previewColumns" :key="c">{{ c }}</th></tr>
              </thead>
              <tbody>
                <tr v-for="(row, ri) in previewRows" :key="ri">
                  <td v-for="c in previewColumns" :key="c" :title="cellText(row[c])">{{ cellText(row[c]) }}</td>
                </tr>
              </tbody>
            </table>
            <StateMessage v-else mode="empty" label="No fields selected" />
          </div>
          <StateMessage v-if="error" mode="error" :message="error" :code="errorCode" />
        </template>
      </div>

      <div class="iew-footer">
        <BaseButton v-if="step > 0" bordered :disabled="running" @click="back">Back</BaseButton>
        <span class="iew-spacer"></span>
        <BaseButton
          v-if="!isLastStep"
          variant="primary"
          :disabled="!canGoNext || loading"
          @click="next"
        >Next</BaseButton>
        <BaseButton
          v-else
          variant="primary"
          :disabled="running || !includedFields.length"
          @click="run"
        >
          {{ running ? (isImport ? 'Importing…' : 'Exporting…') : (isImport ? 'Run import' : 'Run export') }}
        </BaseButton>
      </div>
    </BaseModal>
</template>

<style scoped>

.iew-steps {
  display: flex;
  gap: 18px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-soft);
  font-size: 12px;
  color: var(--text-faint);
}
.iew-step { display: flex; align-items: center; gap: 6px; }
.iew-step.active { color: var(--text); }
.iew-step.done { color: var(--text-dim); }
.iew-dot {
  display: inline-grid;
  place-items: center;
  width: 18px; height: 18px;
  border-radius: 50%;
  background: var(--bg-input);
  border: 1px solid var(--border);
  font-size: 11px;
}
.iew-step.active .iew-dot { background: var(--accent); color: #fff; border-color: var(--accent); }

.iew-body {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 240px;
  max-height: 66vh;
  overflow: hidden;
}
.iew-note { margin: 0; font-size: 12px; color: var(--text-dim); }

.iew-source { display: flex; align-items: center; gap: 10px; }
.iew-path {
  font-family: var(--mono);
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.iew-head, .iew-row {
  display: grid;
  grid-template-columns: 28px 1fr 1fr 120px auto;
  gap: 10px;
  align-items: center;
}
.iew-head {
  padding: 0 4px 6px;
  border-bottom: 1px solid var(--border-soft);
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.iew-rows { overflow-y: auto; display: flex; flex-direction: column; }
.iew-row {
  padding: 5px 4px;
  border-bottom: 1px solid var(--grid-line);
}
.iew-chk { justify-self: center; }
.iew-field {
  font-family: var(--mono);
  font-size: 12.5px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.base-input.iew-input {
  border-radius: 5px;
  padding: 3px 6px;
  font-size: 12px;
}
.iew-select { min-width: 110px; }
.iew-order { display: flex; gap: 4px; }

.iew-f { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 6px; }
.iew-export-opts { display: flex; align-items: center; gap: 16px; flex: none; }
.iew-inc { cursor: pointer; }
.iew-inc input { cursor: pointer; }

.iew-preview-top { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.iew-table-wrap { overflow: auto; border: 1px solid var(--border-soft); border-radius: 6px; }
.iew-table { border-collapse: collapse; font-size: 12px; min-width: 100%; }
.iew-table th, .iew-table td {
  border-bottom: 1px solid var(--grid-line);
  border-right: 1px solid var(--grid-line);
  padding: 4px 8px;
  text-align: left;
  white-space: nowrap;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.iew-table th {
  position: sticky;
  top: 0;
  background: var(--bg-input);
  color: var(--text-dim);
  font-weight: 500;
}
.iew-table td { color: var(--text); font-family: var(--mono); }

.iew-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-top: 1px solid var(--border-soft);
}
.iew-spacer { flex: 1; }
</style>
