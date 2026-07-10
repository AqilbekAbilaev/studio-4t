import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { isProtectedIndex, indexKeyLabel, indexSpecJson } from '../utils/indexSpec'
// NOTE: indexSpecJson is needed because copyIndex() calls it. App.vue ALSO keeps
// its own indexSpecJson import for the template — both importing the pure helper is fine.

// Index-management state + actions for the Indexes dialog and Index menu.
// `showToast` is injected so the composable stays UI-agnostic.
export function useIndexes({ showToast }) {
  const indexesTarget   = ref(null)  // { connId, dbName, collName } | null
  const indexesList     = ref([])
  const indexesLoading  = ref(false)
  const indexesError    = ref(null)
  const newIndexKeys    = ref('')
  const newIndexName    = ref('')
  const newIndexUnique  = ref(false)
  const indexCreating   = ref(false)
  const pendingDropIndex = ref(null)  // index name armed for a confirming second click

  // Index-menu selection & dialogs. `selectedIndex` is the index row highlighted in
  // the Indexes dialog; it drives the Index menu's enablement (see menuContext) and
  // is the target of every Index-menu action.
  const selectedIndex        = ref(null)   // the selected index doc | null
  const indexFormMode        = ref('create')  // 'create' | 'edit'
  const indexEditOriginalName = ref('')    // name of the index being edited (edit mode)
  const indexDetailsTarget   = ref(null)   // the index shown in the View Details modal | null
  const indexDetailsStats    = ref(null)   // its $indexStats entry | null
  const indexDetailsLoading  = ref(false)
  const dropIndexTarget      = ref(null)   // { name } armed for the type-to-confirm drop | null
  const dropIndexConfirmText = ref('')
  const dropIndexError       = ref(null)
  const dropIndexBusy        = ref(false)

  async function loadIndexes() {
    const target = indexesTarget.value
    if (!target) return
    indexesLoading.value = true
    indexesError.value = null
    try {
      indexesList.value = await invoke('list_indexes', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
      })
      // Re-point the selection at the reloaded index object (a fresh list replaces
      // the old references); clear it if that index no longer exists.
      if (selectedIndex.value) {
        selectedIndex.value = indexesList.value.find(i => i.name === selectedIndex.value.name) || null
      }
    } catch (e) {
      indexesError.value = errMessage(e)
      indexesList.value = []
    } finally {
      indexesLoading.value = false
    }
  }

  // Reset the create/edit form back to a blank create.
  function resetIndexForm() {
    newIndexKeys.value = ''
    newIndexName.value = ''
    newIndexUnique.value = false
    indexFormMode.value = 'create'
    indexEditOriginalName.value = ''
  }

  // Closes the Indexes dialog and clears its selection/form so the Index menu
  // disables again (and any half-typed edit is discarded).
  function closeIndexesModal() {
    indexesTarget.value = null
    selectedIndex.value = null
    pendingDropIndex.value = null
    resetIndexForm()
  }

  async function confirmCreateIndex() {
    const target = indexesTarget.value
    const keys = newIndexKeys.value.trim()
    if (!target || !keys) return
    const editing = indexFormMode.value === 'edit' && !!indexEditOriginalName.value
    indexCreating.value = true
    indexesError.value = null
    try {
      // MongoDB has no in-place index edit, so an edit drops the original first and
      // recreates it from the (possibly changed) form values.
      if (editing) {
        await invoke('drop_index', {
          id: target.connId,
          database: target.dbName,
          collection: target.collName,
          name: indexEditOriginalName.value,
        })
      }
      await invoke('create_index', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
        keys: keys,
        unique: newIndexUnique.value,
        name: newIndexName.value.trim(),
      })
      resetIndexForm()
      await loadIndexes()
      showToast(editing ? 'Index updated' : 'Index created')
    } catch (e) {
      indexesError.value = errMessage(e)
    } finally {
      indexCreating.value = false
    }
  }

  // Two-click guard: the first click arms a row, the second actually drops it.
  async function dropIndex(name) {
    if (pendingDropIndex.value !== name) {
      pendingDropIndex.value = name
      return
    }
    const target = indexesTarget.value
    if (!target) return
    indexesError.value = null
    try {
      await invoke('drop_index', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
        name: name,
      })
      pendingDropIndex.value = null
      await loadIndexes()
      showToast(`Index "${name}" dropped`)
    } catch (e) {
      indexesError.value = errMessage(e)
      pendingDropIndex.value = null
    }
  }

  // --- Index menu actions (operate on the selected row in the Indexes dialog) ---

  // The selected index, or null with a nudge if somehow invoked without one. The
  // Index-menu gate guarantees a selection, so this is just defensive.
  function requireSelectedIndex() {
    if (!indexesTarget.value || !selectedIndex.value) {
      showToast('Select an index first')
      return null
    }
    return selectedIndex.value
  }

  // Edit Index…: pre-fill the create form with the selected index as a starting
  // point and switch it to edit mode (save = drop-and-recreate).
  function startEditIndex() {
    const idx = requireSelectedIndex()
    if (!idx) return
    if (isProtectedIndex(idx.name)) {
      showToast('The _id index cannot be edited')
      return
    }
    newIndexKeys.value = indexKeyLabel(idx) ? JSON.stringify(idx.key) : ''
    newIndexName.value = idx.name || ''
    newIndexUnique.value = !!idx.unique
    indexFormMode.value = 'edit'
    indexEditOriginalName.value = idx.name
  }

  // View Details: show the full spec (read-only) plus usage stats when available.
  async function openIndexDetails() {
    const idx = requireSelectedIndex()
    if (!idx) return
    const target = indexesTarget.value
    indexDetailsTarget.value = idx
    indexDetailsStats.value = null
    indexDetailsLoading.value = true
    try {
      const all = await invoke('index_stats', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
      })
      indexDetailsStats.value = all.find(s => s.name === idx.name) || null
    } catch (e) {
      // $indexStats can be unsupported (older server, non-replicated deployment);
      // the spec is still shown, just without usage numbers.
      indexDetailsStats.value = null
    } finally {
      indexDetailsLoading.value = false
    }
  }

  // $indexStats.accesses.since is a BSON date, which crosses the wire as relaxed
  // Extended JSON (a string, or a { $date } wrapper). Render whichever we get as a
  // plain string rather than "[object Object]".
  function formatIndexSince(value) {
    if (value == null) return '—'
    if (typeof value === 'object') {
      const inner = value.$date
      if (inner == null) return JSON.stringify(value)
      return typeof inner === 'object' ? (inner.$numberLong ?? JSON.stringify(inner)) : inner
    }
    return value
  }

  // Copy Index: put the full index definition on the clipboard as pretty JSON.
  function copyIndex() {
    const idx = requireSelectedIndex()
    if (!idx) return
    navigator.clipboard.writeText(indexSpecJson(idx))
    showToast('Index copied')
  }

  // Drop Index: open the type-to-confirm dialog; never for the _id_ index.
  function openDropIndexConfirm() {
    const idx = requireSelectedIndex()
    if (!idx) return
    if (isProtectedIndex(idx.name)) {
      showToast('The _id index cannot be dropped')
      return
    }
    dropIndexTarget.value = { name: idx.name }
    dropIndexConfirmText.value = ''
    dropIndexError.value = null
  }

  async function confirmDropIndex() {
    const target = indexesTarget.value
    const drop = dropIndexTarget.value
    if (!target || !drop) return
    if (dropIndexConfirmText.value !== drop.name) return
    dropIndexBusy.value = true
    dropIndexError.value = null
    try {
      await invoke('drop_index', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
        name: drop.name,
      })
      dropIndexTarget.value = null
      await loadIndexes()
      showToast(`Index "${drop.name}" dropped`)
    } catch (e) {
      dropIndexError.value = errMessage(e)
    } finally {
      dropIndexBusy.value = false
    }
  }

  // Hide / Unhide Index: toggle the planner-visibility flag without a rebuild.
  async function setIndexHidden(hidden) {
    const idx = requireSelectedIndex()
    if (!idx) return
    if (isProtectedIndex(idx.name)) {
      showToast('The _id index cannot be hidden')
      return
    }
    const target = indexesTarget.value
    const name = idx.name
    indexesError.value = null
    try {
      await invoke('set_index_hidden', {
        id: target.connId,
        database: target.dbName,
        collection: target.collName,
        name: name,
        hidden: hidden,
      })
      await loadIndexes()
      showToast(hidden ? `Index "${name}" hidden` : `Index "${name}" unhidden`)
    } catch (e) {
      indexesError.value = errMessage(e)
    }
  }

  // Opener: absorbs App.vue's old "Indexes…" branch body.
  async function openIndexes(target) {
    indexesTarget.value = {
      connId: target.connId,
      dbName: target.dbName,
      collName: target.collName,
    }
    indexesError.value = null
    selectedIndex.value = null
    pendingDropIndex.value = null
    resetIndexForm()
    await loadIndexes()
  }

  return {
    indexesTarget: indexesTarget,
    indexesList: indexesList,
    indexesLoading: indexesLoading,
    indexesError: indexesError,
    newIndexKeys: newIndexKeys,
    newIndexName: newIndexName,
    newIndexUnique: newIndexUnique,
    indexCreating: indexCreating,
    pendingDropIndex: pendingDropIndex,
    selectedIndex: selectedIndex,
    indexFormMode: indexFormMode,
    indexEditOriginalName: indexEditOriginalName,
    indexDetailsTarget: indexDetailsTarget,
    indexDetailsStats: indexDetailsStats,
    indexDetailsLoading: indexDetailsLoading,
    dropIndexTarget: dropIndexTarget,
    dropIndexConfirmText: dropIndexConfirmText,
    dropIndexError: dropIndexError,
    dropIndexBusy: dropIndexBusy,
    loadIndexes: loadIndexes,
    resetIndexForm: resetIndexForm,
    closeIndexesModal: closeIndexesModal,
    confirmCreateIndex: confirmCreateIndex,
    dropIndex: dropIndex,
    startEditIndex: startEditIndex,
    openIndexDetails: openIndexDetails,
    formatIndexSince: formatIndexSince,
    copyIndex: copyIndex,
    openDropIndexConfirm: openDropIndexConfirm,
    confirmDropIndex: confirmDropIndex,
    setIndexHidden: setIndexHidden,
    openIndexes: openIndexes,
  }
}
