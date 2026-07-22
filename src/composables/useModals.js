import { ref, reactive } from 'vue'

// Open-state for every top-level modal/dialog. Registry-driven modals (see
// constants/modalRegistry.js) live in a single `openModals` map — key present with
// its context payload iff open — opened/closed via openModal/closeModal. The remaining
// `*Target` refs and `show*` booleans are modals not yet migrated to the registry.
// The dispatchers (handleTool / handleMenuAction / handleContextAction) set these;
// AppModals.vue reads them to render the modals.
export function useModals() {
  // id -> context payload, key present iff that modal is open.
  const openModals = reactive({})
  function openModal(id, payload) { openModals[id] = payload || {} }
  function closeModal(id) { delete openModals[id] }
  function isModalOpen(id) { return id in openModals }

  const showConnectionManager = ref(false)
  const validatorTarget = ref(null)     // { connId, connName, dbName, collName } when the Validator modal is open
  const gridfsTarget = ref(null)        // { connId, connName, dbName } for the GridFS modal
  const gridfsRequest = ref(null)       // { action, nonce } signal to the open GridFS modal
  const showTasksModal = ref(false)     // Tasks panel (top-bar Tasks button / File → Open Tasks)
  const maskingTarget = ref(null)       // { connId, connName, dbName, collName } for the Data Masking modal
  const importWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Import wizard
  const exportWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Export wizard
  const reschemaTarget = ref(null)      // { connId, connName, dbName, collName } for the Reschema modal
  const showShortcuts = ref(false)      // Help → Keyboard Shortcuts reference
  const showAbout = ref(false)          // Help → About
  const showPreferences = ref(false)    // File → Preferences

  return {
    openModals: openModals,
    openModal: openModal,
    closeModal: closeModal,
    isModalOpen: isModalOpen,
    showConnectionManager: showConnectionManager,
    validatorTarget: validatorTarget,
    gridfsTarget: gridfsTarget,
    gridfsRequest: gridfsRequest,
    showTasksModal: showTasksModal,
    maskingTarget: maskingTarget,
    importWizardTarget: importWizardTarget,
    exportWizardTarget: exportWizardTarget,
    reschemaTarget: reschemaTarget,
    showShortcuts: showShortcuts,
    showAbout: showAbout,
    showPreferences: showPreferences,
  }
}
