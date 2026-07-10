import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { inspectField, setFieldValue, addFieldValue, removeField, renameField, getContainer } from '../utils/docEdit'
import { valueToClipboard, valueToEjson, documentToClipboard, fieldPath } from '../utils/clipboardCopy'

// Document CRUD + field-edit + native Document/Collection menu dispatch for the results
// grid. UI-agnostic: `activeTab`/`docMenuRequest` are getters onto ResultsPanel's props,
// `viewMode` is the shared view ref the menu's View items flip, and `showToast`/`requery`
// are injected callbacks (ResultsPanel forwards them to its `toast`/`requery` emits).
export function useDocumentActions({ activeTab, docMenuRequest, viewMode, showToast, requery }) {
  // ── drill into nested object cells ─────────────────────
  // field-name path navigated into, e.g. ['bank_account', 'account']. Owned here
  // (not in ResultTable) so it survives switching to the JSON / Explain view and back,
  // and so the run-reset below has a stable place to live.
  const drillPath = ref([])

  // The parent's run pipeline no longer clears the drill path directly; reset it on the
  // rising edge of isRunning so every fresh run (refresh, pagination, query bar) starts
  // at the top level. The grid shows its loading skeleton while isRunning, so the reset
  // is never visible mid-flight.
  watch(() => { const tab = activeTab(); return tab && tab.isRunning }, (running, prev) => {
    if (running && !prev) drillPath.value = []
  })

  // Reset the drill path when switching tabs so the new collection opens at top level.
  watch(() => activeTab()?.id, () => { drillPath.value = [] })

  // ── document CRUD ──────────────────────────────────────
  const showDocModal     = ref(false)
  const docModalMode     = ref('insert')
  const showDeleteConfirm = ref(false)
  const crudError        = ref(null)
  const crudSaving       = ref(false)

  function openInsert() {
    docModalMode.value = 'insert'
    crudError.value = null
    showDocModal.value = true
  }

  function openEdit() {
    docModalMode.value = 'edit'
    crudError.value = null
    showDocModal.value = true
  }

  function buildIdFilter(doc) {
    return JSON.stringify({ _id: doc._id })
  }

  async function onDocSave(jsonStr) {
    crudSaving.value = true
    crudError.value = null
    const tab = activeTab()
    try {
      if (docModalMode.value === 'insert') {
        await invoke('insert_document', {
          id: tab.connectionId,
          database: tab.dbName,
          collection: tab.collectionName,
          document: jsonStr,
        })
      } else {
        const original = tab.results[tab.selectedRow]
        await invoke('replace_document', {
          id: tab.connectionId,
          database: tab.dbName,
          collection: tab.collectionName,
          idFilter: buildIdFilter(original),
          document: jsonStr,
        })
      }
      showDocModal.value = false
      requery(true)
    } catch (e) {
      crudError.value = errMessage(e)
    } finally {
      crudSaving.value = false
    }
  }

  // ── copy document (toolbar button) ─────────────────────
  function copySelectedDocument() {
    const tab = activeTab()
    if (!tab || tab.selectedRow < 0) return
    navigator.clipboard.writeText(JSON.stringify(tab.results[tab.selectedRow], null, 2))
  }

  async function onDeleteConfirm() {
    const tab = activeTab()
    const original = tab.results[tab.selectedRow]
    crudError.value = null
    try {
      await invoke('delete_document', {
        id: tab.connectionId,
        database: tab.dbName,
        collection: tab.collectionName,
        idFilter: buildIdFilter(original),
      })
      showDeleteConfirm.value = false
      tab.selectedRow = -1
      requery(true)
    } catch (e) {
      crudError.value = errMessage(e)
    }
  }

  // ── Document / Collection menu editors ─────────────────
  // Field-level edits (Edit Value/Type, Add Field, Remove Field, Rename Field), the
  // read-only JSON view, and the collection-wide Update/Delete/Clear dialogs. Field
  // ops operate on the selected row's document at the current drill path, sent through
  // the same replace_document command the inline cell editor uses.
  const fieldEdit        = ref(null)   // { mode:'edit'|'add'|'rename', fieldName, initialType, initialRaw }
  const fieldEditError   = ref(null)
  const removeFieldName  = ref(null)   // field pending remove-confirm
  const removeFieldError = ref(null)
  const viewJsonDoc      = ref(null)   // document shown read-only
  const showUpdateDialog = ref(false)
  const showDeleteDialog = ref(false)
  const showClearConfirm = ref(false)
  const clearConfirmText = ref('')
  const clearBusy        = ref(false)
  const clearError       = ref(null)

  // The currently selected document, or null when no row is selected / out of range.
  function selectedDoc() {
    const tab = activeTab()
    if (!tab || (tab.selectedRow ?? -1) < 0) return null
    return tab.results?.[tab.selectedRow] ?? null
  }

  // Dispatch a native Document/Collection menu action onto this panel. The menu gates
  // guarantee the prerequisites (active collection / selected row / selected field);
  // we re-check defensively and guide the user if the selection changed meanwhile.
  function runDocMenuAction(action) {
    const tab = activeTab()
    if (!tab || tab.kind !== 'collection') return

    // Collection-wide actions — no row selection required.
    switch (action) {
      case 'coll:insert_document': openInsert(); return
      case 'coll:update_dialog':   showUpdateDialog.value = true; return
      case 'coll:delete_dialog':   showDeleteDialog.value = true; return
      case 'edit:paste_documents': pasteDocuments(); return
      // View → results view mode (mirrors the in-panel view picker).
      case 'view:table': viewMode.value = 'table'; return
      case 'view:tree':  viewMode.value = 'tree';  return
      case 'view:json':  viewMode.value = 'json';  return
      // View → Refresh Document: re-run the current query to refresh the results.
      case 'view:refresh_document': requery(true); return
      // View → Step Out: pop one drill level (the results grid is field-path based).
      case 'view:step_out':
        if (drillPath.value.length) {
          drillPath.value = drillPath.value.slice(0, -1)
        } else {
          showToast('Already at the top level')
        }
        return
      // View → Step Into Column / Cell: drill into the selected field if it holds an
      // object/array. Both step by field name in this field-path-based grid.
      case 'view:step_column':
      case 'view:step_cell': {
        const stepDoc = selectedDoc()
        const stepField = tab.selectedField
        if (!stepDoc || !stepField) { showToast('Select a cell to step into'); return }
        const stepVal = selectedFieldValue(stepDoc)
        if (stepVal === null || typeof stepVal !== 'object') {
          showToast('Selected cell is not an object or array')
          return
        }
        drillPath.value = [...drillPath.value, stepField]
        tab.selectedRow = -1
        tab.selectedField = null
        return
      }
      case 'coll:clear':
        clearConfirmText.value = ''
        clearError.value = null
        showClearConfirm.value = true
        return
    }

    const doc = selectedDoc()
    if (!doc) { showToast('Select a document in the results first'); return }

    // Whole-document actions.
    switch (action) {
      case 'doc:view_json': viewJsonDoc.value = doc; return
      case 'doc:edit_json': openEdit(); return
      // Edit Document in Window: pop the selected document out into the dedicated
      // editor window (Studio-3T-style Cmd/Ctrl+J). The single window retargets when
      // fired again on a different document.
      case 'doc:edit_window': {
        const target = {
          connId: tab.connectionId,
          db: tab.dbName,
          coll: tab.collectionName,
          idFilter: buildIdFilter(doc),
          label: (doc._id !== null && typeof doc._id === 'object')
            ? JSON.stringify(doc._id)
            : String(doc._id),
        }
        invoke('open_document_window', { target: target })
        return
      }
      case 'doc:delete':    crudError.value = null; showDeleteConfirm.value = true; return
      case 'doc:add_field':
        fieldEditError.value = null
        fieldEdit.value = { mode: 'add', fieldName: '', initialType: 'String', initialRaw: '' }
        return
      // Edit → Copy Document: the whole selected document as pretty Extended JSON.
      case 'edit:copy_document':
        writeClipboard(documentToClipboard(doc), 'Document copied')
        return
      // Edit → Copy: context-appropriate — the selected cell's value if a cell is
      // selected, otherwise the whole document (mirrors the grid's Ctrl+C).
      case 'edit:copy':
        if (tab.selectedField) {
          writeClipboard(valueToClipboard(selectedFieldValue(doc)), 'Copied')
        } else {
          writeClipboard(documentToClipboard(doc), 'Copied')
        }
        return
    }

    // Field-level actions — need a selected field.
    const field = tab.selectedField
    if (!field) { showToast('Select a field (click a cell) first'); return }

    // Field-level copies are read-only, so they're allowed on any field including _id
    // (unlike the edits below, which _id blocks).
    switch (action) {
      case 'edit:copy_value':
        writeClipboard(valueToEjson(selectedFieldValue(doc)), 'Value copied')
        return
      case 'edit:copy_field':
        writeClipboard(field, 'Field name copied')
        return
      case 'edit:copy_field_path':
        writeClipboard(fieldPath(drillPath.value, field), 'Field path copied')
        return
    }

    // The _id field can't be changed: replace_document preserves the original _id, so
    // editing/removing/renaming it would be a silent no-op. Block it here, matching the
    // inline cell editor, which already refuses to edit _id (ResultTable guessType 'id').
    if (field === '_id') { showToast('The _id field cannot be edited'); return }
    switch (action) {
      case 'doc:edit_value': {
        const info = inspectField(doc, drillPath.value, field)
        fieldEditError.value = null
        fieldEdit.value = { mode: 'edit', fieldName: field, initialType: info.type, initialRaw: info.raw }
        return
      }
      case 'doc:rename_field':
        fieldEditError.value = null
        fieldEdit.value = { mode: 'rename', fieldName: field, initialType: 'String', initialRaw: '' }
        return
      case 'doc:remove_field':
        removeFieldError.value = null
        removeFieldName.value = field
        return
    }
  }

  watch(() => docMenuRequest() && docMenuRequest().nonce, (nonce) => {
    if (nonce == null) return
    runDocMenuAction(docMenuRequest().action)
  })

  // The selected cell's value: the field on the container at the current drill path.
  // (When not drilled the container is the document root.) Returns undefined if the
  // path/field no longer resolves.
  function selectedFieldValue(doc) {
    const container = getContainer(doc, drillPath.value)
    if (container === null || typeof container !== 'object') return undefined
    return container[activeTab().selectedField]
  }

  // Put `text` on the system clipboard and confirm with a toast, or report failure.
  function writeClipboard(text, okMessage) {
    navigator.clipboard.writeText(text ?? '')
      .then(() => showToast(okMessage))
      .catch(() => showToast('Copy to clipboard failed'))
  }

  // Edit → Paste Document(s): read the clipboard, insert its document(s) into the
  // active collection, and refresh. Parse/insert errors surface as a toast (the
  // backend validates the Extended JSON), so a bad paste never crashes.
  async function pasteDocuments() {
    const tab = activeTab()
    if (!tab || tab.kind !== 'collection' || !tab.collectionName) {
      showToast('Open a collection first')
      return
    }
    let text
    try {
      text = await navigator.clipboard.readText()
    } catch (e) {
      showToast('Cannot read from clipboard')
      return
    }
    if (!text || !text.trim()) {
      showToast('Clipboard is empty')
      return
    }
    try {
      const count = await invoke('insert_documents', {
        id: tab.connectionId,
        database: tab.dbName,
        collection: tab.collectionName,
        documents: text,
      })
      showToast(`Pasted ${count} document${count !== 1 ? 's' : ''}`)
      requery(true)
    } catch (e) {
      showToast(errMessage(e))
    }
  }

  // Persist a field-op mutation of the selected document via replace_document, then
  // refresh the page so the grid reflects it.
  async function saveDocReplacement(newDoc, original) {
    const tab = activeTab()
    await invoke('replace_document', {
      id: tab.connectionId,
      database: tab.dbName,
      collection: tab.collectionName,
      idFilter: buildIdFilter(original),
      document: JSON.stringify(newDoc),
    })
    requery(true)
  }

  async function onFieldEditSave(payload) {
    const tab = activeTab()
    const doc = selectedDoc()
    if (!doc || !fieldEdit.value) { fieldEdit.value = null; return }
    fieldEditError.value = null
    try {
      const mode = fieldEdit.value.mode
      let newDoc
      if (mode === 'edit') {
        newDoc = setFieldValue(doc, drillPath.value, payload.name, payload.value)
      } else if (mode === 'add') {
        newDoc = addFieldValue(doc, drillPath.value, payload.name, payload.value)
      } else {
        newDoc = renameField(doc, drillPath.value, fieldEdit.value.fieldName, payload.name)
      }
      await saveDocReplacement(newDoc, doc)
      fieldEdit.value = null
      tab.selectedField = null
    } catch (e) {
      fieldEditError.value = errMessage(e)
    }
  }

  async function onRemoveFieldConfirm() {
    const tab = activeTab()
    const doc = selectedDoc()
    const field = removeFieldName.value
    if (!doc || !field) { removeFieldName.value = null; return }
    removeFieldError.value = null
    try {
      const newDoc = removeField(doc, drillPath.value, field)
      await saveDocReplacement(newDoc, doc)
      removeFieldName.value = null
      tab.selectedField = null
    } catch (e) {
      removeFieldError.value = errMessage(e)
    }
  }

  async function onClearConfirm() {
    const tab = activeTab()
    if (clearConfirmText.value !== tab.collectionName) return
    clearError.value = null
    clearBusy.value = true
    try {
      const removed = await invoke('clear_collection', {
        id: tab.connectionId,
        database: tab.dbName,
        collection: tab.collectionName,
      })
      showClearConfirm.value = false
      tab.selectedRow = -1
      tab.selectedField = null
      showToast(`Cleared ${removed} document${removed !== 1 ? 's' : ''} from ${tab.collectionName}`)
      requery(true)
    } catch (e) {
      clearError.value = errMessage(e)
    } finally {
      clearBusy.value = false
    }
  }

  function onUpdateDialogDone(message) {
    showUpdateDialog.value = false
    showToast(message)
    requery(true)
  }

  function onDeleteDialogDone(message) {
    const tab = activeTab()
    showDeleteDialog.value = false
    tab.selectedRow = -1
    tab.selectedField = null
    showToast(message)
    requery(true)
  }

  return {
    drillPath: drillPath,
    showDocModal: showDocModal,
    docModalMode: docModalMode,
    showDeleteConfirm: showDeleteConfirm,
    crudError: crudError,
    openInsert: openInsert,
    openEdit: openEdit,
    onDocSave: onDocSave,
    copySelectedDocument: copySelectedDocument,
    onDeleteConfirm: onDeleteConfirm,
    fieldEdit: fieldEdit,
    fieldEditError: fieldEditError,
    removeFieldName: removeFieldName,
    removeFieldError: removeFieldError,
    viewJsonDoc: viewJsonDoc,
    showUpdateDialog: showUpdateDialog,
    showDeleteDialog: showDeleteDialog,
    showClearConfirm: showClearConfirm,
    clearConfirmText: clearConfirmText,
    clearBusy: clearBusy,
    clearError: clearError,
    onFieldEditSave: onFieldEditSave,
    onRemoveFieldConfirm: onRemoveFieldConfirm,
    onClearConfirm: onClearConfirm,
    onUpdateDialogDone: onUpdateDialogDone,
    onDeleteDialogDone: onDeleteDialogDone,
  }
}
