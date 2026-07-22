import { reactive } from 'vue'

// Open-state for every top-level modal/dialog. Each modal is declared once in
// constants/modalRegistry.js and lives in the single `openModals` map — its key is
// present with its context payload iff it is open. The dispatchers (handleTool /
// handleMenuAction / handleContextAction) open/close via openModal/closeModal; AppModals.vue
// renders whatever is open. App-level singletons open with an empty payload (openModal('about')).
export function useModals() {
  // id -> context payload, key present iff that modal is open.
  const openModals = reactive({})
  function openModal(id, payload) { openModals[id] = payload || {} }
  function closeModal(id) { delete openModals[id] }
  function isModalOpen(id) { return id in openModals }

  return {
    openModals: openModals,
    openModal: openModal,
    closeModal: closeModal,
    isModalOpen: isModalOpen,
  }
}
