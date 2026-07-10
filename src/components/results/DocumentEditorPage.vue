<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { buildExtensions, EditorView, EditorState } from '../../utils/docEditor'
import { parseDocumentJson } from '../../utils/docJson'
import { errMessage } from '../../utils/errors'

// The pop-out document editor (Studio-3T-style Cmd/Ctrl+J). It is the SINGLE reusable
// "doc-editor" window: opened by the Rust open_document_window command, seeded from the
// window URL on first load, and retargeted in place via the 'document-target' event.

const editorHost = ref(null)   // container the CodeMirror view mounts into
let view = null

// Current target ({ connId, db, coll, idFilter, label }) and editor state.
const target   = ref(null)
const title    = ref('Edit Document')
const text     = ref('')
const dirty    = ref(false)
const loading  = ref(false)
const saving   = ref(false)
const jsonErr  = ref(null)
const saved    = ref(false)   // brief "Saved" confirmation state
let savedTimer = null

// Read the initial target from the window URL query (?connId=&db=&coll=&idFilter=&label=).
function targetFromUrl() {
  const p = new URLSearchParams(location.search)
  const connId = p.get('connId')
  const db = p.get('db')
  const coll = p.get('coll')
  const idFilter = p.get('idFilter')
  if (!connId || !db || !coll || !idFilter) return null
  return {
    connId: connId,
    db: db,
    coll: coll,
    idFilter: idFilter,
    label: p.get('label') || '',
  }
}

function setEditorText(value) {
  text.value = value
  if (!view) return
  view.setState(EditorState.create({
    doc: value,
    extensions: buildExtensions({ onChange: onEditorChange, onSave: onSave }),
  }))
}

function onEditorChange(value) {
  text.value = value
  dirty.value = true
  saved.value = false
}

// Fetch the target document and load it into the editor. Reuses find_documents with the
// _id filter (limit 1) and pretty-prints the single match.
async function loadTarget(next) {
  if (!next) {
    jsonErr.value = 'No document was specified to edit.'
    return
  }
  target.value = next
  // The real OS window owns the titlebar, so reflect the document there rather than
  // drawing our own (a custom top bar would double up with the native one).
  title.value = next.label ? `Edit Document — ${next.label}` : 'Edit Document'
  getCurrentWindow().setTitle(title.value).catch(() => {})
  loading.value = true
  jsonErr.value = null
  try {
    const docs = await invoke('find_documents', {
      id: next.connId,
      database: next.db,
      collection: next.coll,
      filter: next.idFilter,
      projection: '{}',
      sort: '{}',
      skip: 0,
      limit: 1,
    })
    const doc = docs && docs.length ? docs[0] : null
    if (!doc) {
      jsonErr.value = 'The document no longer exists.'
      setEditorText('')
    } else {
      setEditorText(JSON.stringify(doc, null, 2))
    }
    dirty.value = false
    saved.value = false
  } catch (e) {
    jsonErr.value = errMessage(e)
  } finally {
    loading.value = false
  }
}

// Retarget guard: if there are unsaved edits, confirm before replacing them.
function confirmDiscardIfDirty() {
  if (!dirty.value) return true
  return window.confirm('This document has unsaved changes. Discard them?')
}

async function onSave() {
  if (!target.value || saving.value) return
  jsonErr.value = null
  try {
    parseDocumentJson(text.value)
  } catch (e) {
    jsonErr.value = e.message
    return
  }
  saving.value = true
  try {
    await invoke('replace_document', {
      id: target.value.connId,
      database: target.value.db,
      collection: target.value.coll,
      idFilter: target.value.idFilter,
      document: text.value,
    })
    await emit('document-saved', {
      connId: target.value.connId,
      db: target.value.db,
      coll: target.value.coll,
    })
    dirty.value = false
    saved.value = true
    clearTimeout(savedTimer)
    savedTimer = setTimeout(() => { saved.value = false }, 2000)
  } catch (e) {
    jsonErr.value = errMessage(e)
  } finally {
    saving.value = false
  }
}

async function onClose() {
  if (!confirmDiscardIfDirty()) return
  await getCurrentWindow().close()
}

let unlisten = null

onMounted(async () => {
  view = new EditorView({
    state: EditorState.create({
      doc: '',
      extensions: buildExtensions({ onChange: onEditorChange, onSave: onSave }),
    }),
    parent: editorHost.value,
  })

  // The single window is retargeted in place: reload when the backend emits a new target.
  unlisten = await listen('document-target', (e) => {
    if (!confirmDiscardIfDirty()) return
    loadTarget(e.payload)
  })

  await loadTarget(targetFromUrl())
  view.focus()
})

onBeforeUnmount(() => {
  clearTimeout(savedTimer)
  if (unlisten) unlisten()
  if (view) view.destroy()
})
</script>

<template>
  <div class="doc-editor">
    <div class="editor-wrap" ref="editorHost"></div>

    <div v-if="jsonErr" class="err">{{ jsonErr }}</div>

    <div class="footer">
      <span v-if="loading" class="hint">Loading…</span>
      <span v-else-if="saved" class="hint saved">Saved</span>
      <span v-else-if="dirty" class="hint">Unsaved changes</span>
      <span class="spacer"></span>
      <button class="btn" @click="onClose">Close</button>
      <button class="btn primary" :disabled="saving || loading" @click="onSave">
        {{ saving ? 'Saving…' : 'Save' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.doc-editor {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-window);
  color: var(--text);
}

.editor-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--bg-input);
}

.err {
  flex: none;
  font-size: 12px;
  color: var(--danger-text);
  padding: 8px 16px;
  border-top: 1px solid var(--border);
  background: var(--bg-panel);
}

.footer {
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
}

.hint { font-size: 12px; color: var(--text-faint); }
.hint.saved { color: var(--ok-text, var(--link)); }
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
.btn:disabled { opacity: .5; cursor: default; }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover:not(:disabled) { opacity: .88; }
</style>
