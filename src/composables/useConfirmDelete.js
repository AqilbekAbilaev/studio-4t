import { ref } from 'vue'

export function useConfirmDelete() {
  const pendingId = ref(null)

  function confirmDelete(id) {
    if (pendingId.value !== id) {
      pendingId.value = id
      return false
    }
    pendingId.value = null
    return true
  }

  function reset() {
    pendingId.value = null
  }

  return { pendingId: pendingId, confirmDelete: confirmDelete, reset: reset }
}
