<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../utils/errors'
import { mongoStringify, syntaxHighlight } from '../utils/mongoFormat'
import BaseIcon from './BaseIcon.vue'
import StateMessage from './StateMessage.vue'

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
    error.value = errMessage(e)
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
    error.value = errMessage(e)
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
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    applying.value = false
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Reschema — {{ target.dbName }}.{{ target.collName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

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
              <select v-model="row.kind" class="rs-select" @change="preview = null">
                <option v-for="k in OP_KINDS" :key="k.value" :value="k.value">{{ k.label }}</option>
              </select>

              <template v-if="row.kind === 'rename' || row.kind === 'move'">
                <input v-model="row.from" list="rs-fields" class="rs-input" placeholder="from path" />
                <span class="rs-arrow">→</span>
                <input v-model="row.to" class="rs-input" placeholder="to path" />
              </template>
              <template v-else-if="row.kind === 'changeType'">
                <input v-model="row.field" list="rs-fields" class="rs-input" placeholder="field path" />
                <span class="rs-arrow">→</span>
                <select v-model="row.toType" class="rs-select">
                  <option v-for="t in TYPES" :key="t" :value="t">{{ t }}</option>
                </select>
              </template>
              <template v-else>
                <input v-model="row.field" list="rs-fields" class="rs-input wide" placeholder="field path" />
              </template>

              <span class="rs-row-actions">
                <button class="rs-icon" title="Move up" :disabled="i === 0" @click="moveOp(i, -1)">↑</button>
                <button class="rs-icon" title="Move down" :disabled="i === ops.length - 1" @click="moveOp(i, 1)">↓</button>
                <button class="rs-icon" title="Remove op" @click="removeOp(i)">
                  <BaseIcon name="close" :size="12" />
                </button>
              </span>
            </div>

            <button class="rs-add" @click="addOp">
              <BaseIcon name="plus" :size="12" /> Add operation
            </button>
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
            <input
              v-if="mode === 'new_collection'"
              v-model="newName"
              class="rs-input"
              placeholder="new collection name"
            />
            <span class="rs-spacer" />
            <button class="rs-btn" :disabled="!canRun || previewing" @click="runPreview">
              {{ previewing ? 'Previewing…' : 'Preview' }}
            </button>
            <button class="rs-btn primary" :disabled="!canRun || applying" @click="runApply">
              {{ applying ? 'Applying…' : 'Apply' }}
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
  width: 760px;
  max-width: 94vw;
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
.rs-select {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 4px 6px;
  font-size: 12px;
}
.rs-input {
  flex: 1;
  min-width: 0;
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 4px 7px;
  font-size: 12.5px;
  font-family: var(--mono);
}
.rs-input.wide { flex: 2; }
.rs-arrow { color: var(--text-faint); flex: none; }
.rs-row-actions { display: flex; align-items: center; gap: 2px; margin-left: auto; }
.rs-icon {
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 3px 5px;
  border-radius: 4px;
  font-size: 12px;
  display: flex;
  align-items: center;
}
.rs-icon:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
.rs-icon:disabled { opacity: .35; cursor: default; }
.rs-add {
  align-self: flex-start;
  background: none;
  border: 1px dashed var(--border);
  color: var(--text-dim);
  border-radius: 6px;
  padding: 5px 10px;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 2px;
}
.rs-add:hover { background: var(--bg-hover); color: var(--text); }

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
.rs-btn {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.rs-btn:hover:not(:disabled) { background: var(--bg-hover); }
.rs-btn.primary { background: var(--accent); color: #fff; border-color: transparent; }
.rs-btn.primary:hover:not(:disabled) { background: var(--accent-soft); }
.rs-btn:disabled { opacity: .55; cursor: default; }
</style>
