<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import { mongoStringify, syntaxHighlight } from '../../utils/mongoFormat'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import ReorderButtons from '../base/ReorderButtons.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'

// Top-bar "Reschema" tool for the active collection. Builds an ordered list of
// transform ops (rename / remove / change type / move nested) and runs them as a
// server-side aggregation. Preview shows the first N documents before and after;
// apply writes either in place (over the source) or to a new collection.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
const emit = defineEmits(['close', 'toast', 'applied'])

const OP_KINDS = [
  { value: 'rename',     label: 'Rename field' },
  { value: 'move',       label: 'Move nested' },
  { value: 'changeType', label: 'Change type' },
  { value: 'remove',     label: 'Remove field' },
]

// $convert targets exposed in the UI (map 1:1 to the backend `toType`).
const TYPES = ['string', 'int', 'long', 'double', 'decimal', 'bool', 'date', 'objectId']
const TYPE_OPTIONS = TYPES.map((t) => ({ value: t, label: t }))

const PREVIEW_LIMIT = 20

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const fieldPaths = ref([])      // dotted paths sampled from one document (datalist hints)
const ops = ref([])            // [{ kind, from, to, field, toType }]
const mode = ref('in_place')   // 'in_place' | 'new_collection'
const newName = ref('')
const previewing = ref(false)
const applying = ref(false)
const preview = ref(null)      // { before: [...], after: [...] } | null

// Flatten one sample document into dotted paths (objects recursed, arrays/scalars
// treated as leaves) so the field inputs can offer autocompletion.
function collectPaths(value, prefix, out) {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    // Skip EJSON wrappers ({ $oid: … }) — they are scalar leaves, not sub-docs.
    const keys = Object.keys(value)
    const isWrapper = keys.length === 1 && keys[0].startsWith('$')
    if (isWrapper) {
      out.push(prefix)
      return
    }
    for (const key of keys) {
      const path = prefix ? `${prefix}.${key}` : key
      collectPaths(value[key], path, out)
    }
  } else if (prefix) {
    out.push(prefix)
  }
}

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
      const paths = []
      collectPaths(sample[0], '', paths)
      fieldPaths.value = paths
    }
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

function addOp() {
  ops.value.push({ kind: 'rename', from: '', to: '', field: '', toType: 'string' })
  preview.value = null
}

function removeOp(index) {
  ops.value.splice(index, 1)
  preview.value = null
}

function moveOp(index, delta) {
  const next = index + delta
  if (next < 0 || next >= ops.value.length) return
  const row = ops.value[index]
  ops.value.splice(index, 1)
  ops.value.splice(next, 0, row)
  preview.value = null
}

// Translate the editor rows into the backend op payload, dropping incomplete rows.
function buildOps() {
  const built = []
  for (const row of ops.value) {
    if (row.kind === 'rename' || row.kind === 'move') {
      const from = row.from.trim()
      const to = row.to.trim()
      if (from && to) built.push({ op: row.kind, from, to })
    } else if (row.kind === 'remove') {
      const field = row.field.trim()
      if (field) built.push({ op: 'remove', field })
    } else if (row.kind === 'changeType') {
      const field = row.field.trim()
      if (field) built.push({ op: 'changeType', field, toType: row.toType })
    }
  }
  return built
}

const builtOps = computed(() => buildOps())
const canRun = computed(() => builtOps.value.length > 0)

function renderDoc(doc) {
  return syntaxHighlight(mongoStringify(doc))
}

async function runPreview() {
  error.value = null
  errorCode.value = null
  previewing.value = true
  try {
    preview.value = await invoke('reschema_preview', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      ops: builtOps.value,
      limit: PREVIEW_LIMIT,
    })
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    previewing.value = false
  }
}

async function runApply() {
  error.value = null
  errorCode.value = null
  if (mode.value === 'new_collection' && !newName.value.trim()) {
    error.value = 'Enter a name for the new collection'
    return
  }
  applying.value = true
  try {
    const count = await invoke('reschema_apply', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      ops: builtOps.value,
      target: {
        mode: mode.value,
        newName: mode.value === 'new_collection' ? newName.value.trim() : null,
      },
    })
    const where = mode.value === 'new_collection'
      ? `to ${newName.value.trim()}`
      : 'in place'
    emit('toast', `Reschema applied ${where} — ${count} document${count === 1 ? '' : 's'}`)
    emit('applied', {
      newCollection: mode.value === 'new_collection',
      connId: props.target.connId,
    })
    emit('close')
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    applying.value = false
  }
}
</script>

<template>
  <BaseModal :title="`Reschema — ${target.dbName}.${target.collName}`" width="760px" max-width="94vw" @close="$emit('close')">

      <div class="rs-body">
        <StateMessage v-if="loading" mode="loading" label="Reading fields…" />
        <template v-else>
          <p class="rs-note">
            Define an ordered list of transforms. They run as a server-side
            aggregation — nothing is written until you apply.
          </p>

          <datalist id="rs-fields">
            <option v-for="p in fieldPaths" :key="p" :value="p" />
          </datalist>

          <div class="rs-ops">
            <div v-for="(row, i) in ops" :key="i" class="rs-op">
              <BaseSelect :model-value="row.kind" class="rs-select" :options="OP_KINDS" size="sm"
                @update:model-value="v => { row.kind = v; preview = null }" />

              <template v-if="row.kind === 'rename' || row.kind === 'move'">
                <BaseInput v-model="row.from" list="rs-fields" class="rs-input" placeholder="from path" />
                <span class="rs-arrow">→</span>
                <BaseInput v-model="row.to" class="rs-input" placeholder="to path" />
              </template>
              <template v-else-if="row.kind === 'changeType'">
                <BaseInput v-model="row.field" list="rs-fields" class="rs-input" placeholder="field path" />
                <span class="rs-arrow">→</span>
                <BaseSelect v-model="row.toType" class="rs-select" :options="TYPE_OPTIONS" size="sm" />
              </template>
              <template v-else>
                <BaseInput v-model="row.field" list="rs-fields" class="rs-input wide" placeholder="field path" />
              </template>

              <span class="rs-row-actions">
                <ReorderButtons
                  :up-disabled="i === 0"
                  :down-disabled="i === ops.length - 1"
                  @up="moveOp(i, -1)"
                  @down="moveOp(i, 1)"
                />
                <BaseButton icon="close" :icon-size="12" title="Remove op" @click="removeOp(i)" />
              </span>
            </div>

            <BaseButton bordered @click="addOp">
              <BaseIcon name="plus" :size="12" /> Add operation
            </BaseButton>
          </div>

          <StateMessage v-if="error" mode="error" :message="error" :code="errorCode" />

          <div v-if="preview" class="rs-preview">
            <div class="rs-pane">
              <div class="rs-pane-head">Before</div>
              <div class="rs-docs">
                <StateMessage v-if="!preview.before.length" mode="empty" label="No documents" />
                <pre v-for="(doc, i) in preview.before" :key="i" class="rs-doc" v-html="renderDoc(doc)" />
              </div>
            </div>
            <div class="rs-pane">
              <div class="rs-pane-head">After</div>
              <div class="rs-docs">
                <StateMessage v-if="!preview.after.length" mode="empty" label="No documents" />
                <pre v-for="(doc, i) in preview.after" :key="i" class="rs-doc" v-html="renderDoc(doc)" />
              </div>
            </div>
          </div>

          <div class="rs-footer">
            <label class="rs-f">
              <input type="radio" value="in_place" v-model="mode" /> In place
            </label>
            <label class="rs-f">
              <input type="radio" value="new_collection" v-model="mode" /> New collection
            </label>
            <BaseInput
              v-if="mode === 'new_collection'"
              v-model="newName"
              class="rs-input"
              placeholder="new collection name"
            />
            <span class="rs-spacer" />
            <BaseButton bordered :disabled="!canRun || previewing" @click="runPreview">
              {{ previewing ? 'Previewing…' : 'Preview' }}
            </BaseButton>
            <BaseButton variant="primary" :disabled="!canRun || applying" @click="runApply">
              {{ applying ? 'Applying…' : 'Apply' }}
            </BaseButton>
          </div>
        </template>
      </div>
    </BaseModal>
</template>

<style scoped>

.rs-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 220px;
  max-height: 80vh;
  overflow-y: auto;
}
.rs-note { margin: 0; font-size: 12px; color: var(--text-dim); }

.rs-ops { display: flex; flex-direction: column; gap: 6px; }
.rs-op {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}
.rs-select { min-width: 120px; }
.base-input.rs-input {
  flex: 1;
  min-width: 0;
  border-radius: 5px;
  padding: 4px 7px;
  font-size: 12.5px;
  font-family: var(--mono);
}
.base-input.rs-input.wide { flex: 2; }
.rs-arrow { color: var(--text-faint); flex: none; }
.rs-row-actions { display: flex; align-items: center; gap: 2px; margin-left: auto; }

.rs-preview {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  border-top: 1px solid var(--border-soft);
  padding-top: 10px;
}
.rs-pane { display: flex; flex-direction: column; min-width: 0; }
.rs-pane-head {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
  margin-bottom: 5px;
}
.rs-docs {
  max-height: 260px;
  overflow: auto;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px;
}
.rs-doc {
  margin: 0 0 8px;
  font-family: var(--mono);
  font-size: 11.5px;
  line-height: 1.45;
  white-space: pre;
  color: var(--text);
}
.rs-doc:last-child { margin-bottom: 0; }

.rs-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-top: 10px;
  border-top: 1px solid var(--border-soft);
}
.rs-f { font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 5px; cursor: pointer; }
.rs-spacer { margin-left: auto; }
</style>
