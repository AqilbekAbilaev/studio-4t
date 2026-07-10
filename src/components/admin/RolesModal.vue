<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

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
    error.value = errMessage(e)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Roles — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>
      <div class="rm-body">
        <StateMessage v-if="loading" mode="loading" label="Loading roles…" />
        <StateMessage v-else-if="error" mode="error" :message="error" />
        <StateMessage v-else-if="!roles.length" mode="empty" label="No custom roles on this database" />
        <ul v-else class="rm-list">
          <li v-for="r in roles" :key="r">{{ r }}</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 70; }
.dialog {
  width: 420px; max-width: 92vw; background: var(--bg-window); border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex; flex-direction: column; overflow: hidden;
}
.dlg-title {
  height: 36px; flex: none; background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border); display: flex; align-items: center; padding: 0 10px; position: relative;
}
.dlg-title .t { position: absolute; left: 0; right: 0; text-align: center; font-size: 13px; color: var(--text-dim); font-weight: 500; pointer-events: none; }
.close-btn { margin-left: auto; background: none; border: none; color: var(--text-faint); cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; z-index: 1; }
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
.rm-body { padding: 16px; min-height: 140px; max-height: 70vh; overflow-y: auto; }
.rm-list { margin: 0; padding-left: 18px; font-size: 13px; color: var(--text); line-height: 1.7; user-select: text; }
</style>
