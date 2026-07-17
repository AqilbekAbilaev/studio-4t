<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'

// Manage Roles for a database: read-only listing of the custom (non-built-in) roles.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const roles = ref([])

onMounted(async () => {
  try {
    roles.value = await invoke('list_roles', { id: props.target.connId, database: props.target.dbName })
  } catch (e) {
    error.value = errText(e)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <BaseModal :title="`Roles — ${target.dbName}`" width="420px" max-width="92vw" @close="$emit('close')">
      <div class="rm-body">
        <StateMessage v-if="loading" mode="loading" label="Loading roles…" />
        <StateMessage v-else-if="error" mode="error" :message="error" />
        <StateMessage v-else-if="!roles.length" mode="empty" label="No custom roles on this database" />
        <ul v-else class="rm-list">
          <li v-for="r in roles" :key="r">{{ r }}</li>
        </ul>
      </div>
    </BaseModal>
</template>

<style scoped>
.rm-body { padding: 16px; min-height: 140px; max-height: 70vh; overflow-y: auto; }
.rm-list { margin: 0; padding-left: 18px; font-size: 13px; color: var(--text); line-height: 1.7; user-select: text; }
</style>
