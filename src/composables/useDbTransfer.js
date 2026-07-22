import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { errText } from '../utils/errors'

// Import / export flows. The per-collection wizard (stepped, with column→field mapping
// and a live preview) just sets its modal target; the database-level Export/Import
// Collections… run the plain per-collection commands in a loop over a chosen folder/files.
// `showToast` and `connectionTreeRef` are injected; `openModal` is the registry opener
// from useModals so the export/import wizards open through the same path as every other
// registry-driven modal.
export function useDbTransfer({ showToast, connectionTreeRef, openModal }) {
  // Open the stepped Import / Export wizard for a single collection. `nodeData` is the
  // sidebar/tab shape ({ connId, connName, dbName, collName }); the wizard maps
  // columns→fields with per-field type coercion and shows a live preview before it runs.
  function openExportWizard(nodeData) {
    openModal('export', {
      connId: nodeData.connId,
      connName: nodeData.connName,
      dbName: nodeData.dbName,
      collName: nodeData.collName,
    })
  }

  function openImportWizard(nodeData) {
    openModal('import', {
      connId: nodeData.connId,
      connName: nodeData.connName,
      dbName: nodeData.dbName,
      collName: nodeData.collName,
    })
  }

  // After a wizard import, refresh the connection so a newly-populated collection shows
  // up in the sidebar.
  async function onWizardImported(connId) {
    await connectionTreeRef.value.refreshConn(connId)
  }

  // Database → Export Collections…: export every collection in the database to a chosen
  // folder, one JSON file per collection. Reuses the per-collection command.
  async function exportDatabase(nodeData) {
    let dir
    try {
      dir = await openDialog({ directory: true, title: `Export all collections in ${nodeData.dbName}` })
    } catch (e) {
      showToast('Export failed: ' + errText(e))
      return
    }
    if (!dir) return  // user cancelled
    let collections = []
    try {
      const dbs = await invoke('list_databases', { id: nodeData.connId })
      collections = (dbs.find(d => d.name === nodeData.dbName)?.collections) || []
    } catch (e) {
      showToast('Export failed: ' + errText(e))
      return
    }
    if (!collections.length) { showToast('No collections to export'); return }
    let done = 0
    let failed = 0
    for (const coll of collections) {
      try {
        await invoke('export_collection', {
          id: nodeData.connId,
          database: nodeData.dbName,
          collection: coll,
          path: `${dir}/${coll}.json`,
          format: 'json',
        })
        done++
      } catch (_) {
        failed++
      }
    }
    showToast(`Exported ${done} collection${done !== 1 ? 's' : ''}${failed ? `, ${failed} failed` : ''}`)
  }

  // Database → Import Collections…: import one or more JSON/CSV files into the database,
  // each into a collection named after the file. Reuses the per-file command.
  async function importDatabase(nodeData) {
    let paths
    try {
      paths = await openDialog({
        multiple: true,
        filters: [{ name: 'JSON or CSV', extensions: ['json', 'csv'] }],
      })
    } catch (e) {
      showToast('Import failed: ' + errText(e))
      return
    }
    if (!paths || !paths.length) return  // user cancelled
    let done = 0
    let failed = 0
    for (const path of paths) {
      const p = String(path)
      const base = p.split(/[\\/]/).pop() || p
      const collection = base.replace(/\.(json|csv)$/i, '')
      const format = p.toLowerCase().endsWith('.csv') ? 'csv' : 'json'
      try {
        await invoke('import_collection', {
          id: nodeData.connId,
          database: nodeData.dbName,
          collection: collection,
          path: p,
          format: format,
        })
        done++
      } catch (_) {
        failed++
      }
    }
    await connectionTreeRef.value.refreshConn(nodeData.connId)
    showToast(`Imported ${done} file${done !== 1 ? 's' : ''}${failed ? `, ${failed} failed` : ''}`)
  }

  return {
    openExportWizard: openExportWizard,
    openImportWizard: openImportWizard,
    onWizardImported: onWizardImported,
    exportDatabase: exportDatabase,
    importDatabase: importDatabase,
  }
}
