import { ref } from 'vue'

// Open-state for every top-level modal/dialog: a `*Target` ref holds the context
// object while its modal is open (null when closed), and a `show*` boolean toggles
// the context-less panels. The dispatchers (handleTool / handleMenuAction /
// handleContextAction) set these; AppModals.vue reads them to render the modals.
export function useModals() {
  const showConnectionManager = ref(false)
  const serverStatusTarget = ref(null)  // { connId, connName } when the Server Status modal is open
  const dbStatsTarget = ref(null)       // { connId, connName, dbName } when the Database Statistics modal is open
  const currentOpsTarget = ref(null)    // { connId, connName } when the Current Operations modal is open
  const profilerTarget = ref(null)      // { connId, connName, dbName } when the Query Profiler modal is open
  const validatorTarget = ref(null)     // { connId, connName, dbName, collName } when the Validator modal is open
  const usersTarget = ref(null)         // { connId, connName, dbName } when the Users modal is open
  const rolesTarget = ref(null)         // { connId, connName, dbName } when the Roles modal is open
  const functionsTarget = ref(null)     // { connId, connName, dbName } when the Stored Functions modal is open
  const mapReduceTarget = ref(null)     // { connId, connName, dbName, collName } when the Map-Reduce modal is open
  const serverChartsTarget = ref(null)  // { connId, connName } when the Server Status Charts modal is open
  const migrationTarget = ref(null)     // { connId, connName, dbName, collName } for the SQL Migration modal
  const searchTarget = ref(null)        // { connId, connName, dbName } for the Global Search modal
  const gridfsTarget = ref(null)        // { connId, connName, dbName } for the GridFS modal
  const gridfsRequest = ref(null)       // { action, nonce } signal to the open GridFS modal
  const compareTarget = ref(null)       // { connId, connName, dbName } for the Data Compare modal
  const schemaTarget = ref(null)  // { connId, connName, dbName, collName } when the Schema modal is open
  const historyTarget = ref(null) // { connId, connName, dbName, collName } for the Collection History modal
  const showTasksModal = ref(false)     // Tasks panel (top-bar Tasks button / File → Open Tasks)
  const maskingTarget = ref(null)       // { connId, connName, dbName, collName } for the Data Masking modal
  const importWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Import wizard
  const exportWizardTarget = ref(null)  // { connId, connName, dbName, collName } for the Export wizard
  const reschemaTarget = ref(null)      // { connId, connName, dbName, collName } for the Reschema modal
  const statsTarget = ref(null)         // { connId, connName, dbName, collName } for the Collection Stats modal
  const serverInfoTarget = ref(null)    // { connId, connName, kind, title } for Build/Host/Replica info
  const showShortcuts = ref(false)      // Help → Keyboard Shortcuts reference
  const showAbout = ref(false)          // Help → About
  const showPreferences = ref(false)    // File → Preferences

  return {
    showConnectionManager: showConnectionManager,
    serverStatusTarget: serverStatusTarget,
    dbStatsTarget: dbStatsTarget,
    currentOpsTarget: currentOpsTarget,
    profilerTarget: profilerTarget,
    validatorTarget: validatorTarget,
    usersTarget: usersTarget,
    rolesTarget: rolesTarget,
    functionsTarget: functionsTarget,
    mapReduceTarget: mapReduceTarget,
    serverChartsTarget: serverChartsTarget,
    migrationTarget: migrationTarget,
    searchTarget: searchTarget,
    gridfsTarget: gridfsTarget,
    gridfsRequest: gridfsRequest,
    compareTarget: compareTarget,
    schemaTarget: schemaTarget,
    historyTarget: historyTarget,
    showTasksModal: showTasksModal,
    maskingTarget: maskingTarget,
    importWizardTarget: importWizardTarget,
    exportWizardTarget: exportWizardTarget,
    reschemaTarget: reschemaTarget,
    statsTarget: statsTarget,
    serverInfoTarget: serverInfoTarget,
    showShortcuts: showShortcuts,
    showAbout: showAbout,
    showPreferences: showPreferences,
  }
}
