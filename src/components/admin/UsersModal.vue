<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Manage Users for a database: list, create, and drop users (via usersInfo /
// createUser / dropUser).
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const loading = ref(true)
const busy = ref(false)
const error = ref(null)
const users = ref([])
const pendingDrop = ref(null)  // username armed for a confirming second click

const showCreate = ref(false)
const newName = ref('')
const newPassword = ref('')
const newRoles = ref('read')   // comma-separated role names or role@db
const createError = ref(null)

async function load() {
  loading.value = true
  error.value = null
  try {
    users.value = await invoke('list_users', { id: props.target.connId, database: props.target.dbName })
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    loading.value = false
  }
}

onMounted(load)

async function createUser() {
  const name = newName.value.trim()
  if (!name || !newPassword.value) return
  const roles = newRoles.value.split(',').map(r => r.trim()).filter(Boolean)
  busy.value = true
  createError.value = null
  try {
    await invoke('create_user', {
      id: props.target.connId,
      database: props.target.dbName,
      username: name,
      password: newPassword.value,
      roles: roles,
    })
    showCreate.value = false
    newName.value = ''
    newPassword.value = ''
    newRoles.value = 'read'
    await load()
  } catch (e) {
    createError.value = errMessage(e)
  } finally {
    busy.value = false
  }
}

async function dropUser(user) {
  if (pendingDrop.value !== user.user) { pendingDrop.value = user.user; return }
  busy.value = true
  try {
    await invoke('drop_user', { id: props.target.connId, database: props.target.dbName, username: user.user })
    await load()
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    busy.value = false
    pendingDrop.value = null
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Users — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="um-body">
        <div class="um-bar">
          <button class="btn primary" :disabled="busy" @click="showCreate = !showCreate">
            <BaseIcon name="plus" :size="12" /> Add User
          </button>
        </div>

        <div v-if="showCreate" class="um-create">
          <input v-model="newName" class="um-input" placeholder="Username" spellcheck="false" />
          <input v-model="newPassword" class="um-input" type="password" placeholder="Password" />
          <input v-model="newRoles" class="um-input" placeholder="Roles (comma-separated, e.g. readWrite, read@other)" spellcheck="false" />
          <div class="um-create-actions">
            <button class="btn" @click="showCreate = false">Cancel</button>
            <button class="btn primary" :disabled="!newName.trim() || !newPassword || busy" @click="createUser">Create</button>
          </div>
          <div v-if="createError" class="um-error">{{ createError }}</div>
        </div>

        <StateMessage v-if="loading" mode="loading" label="Loading users…" />
        <StateMessage v-else-if="error" mode="error" :message="error" />
        <StateMessage v-else-if="!users.length" mode="empty" label="No users on this database" />
        <table v-else class="um-table">
          <thead>
            <tr><th>User</th><th>Database</th><th>Roles</th><th></th></tr>
          </thead>
          <tbody>
            <tr v-for="u in users" :key="u.user + '@' + u.db">
              <td>{{ u.user }}</td>
              <td>{{ u.db }}</td>
              <td class="um-roles">{{ u.roles.join(', ') || '—' }}</td>
              <td class="um-act">
                <button
                  class="btn danger-btn"
                  :class="{ armed: pendingDrop === u.user }"
                  :disabled="busy"
                  @click="dropUser(u)"
                >{{ pendingDrop === u.user ? 'Confirm' : 'Drop' }}</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 70; }
.dialog {
  width: 660px; max-width: 92vw; background: var(--bg-window); border-radius: 10px;
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

.um-body { padding: 14px 16px 16px; display: flex; flex-direction: column; gap: 12px; min-height: 200px; max-height: 74vh; overflow-y: auto; }
.um-bar { display: flex; }
.um-create { display: flex; flex-direction: column; gap: 8px; padding: 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; }
.um-create-actions { display: flex; justify-content: flex-end; gap: 8px; }
.um-input {
  width: 100%; box-sizing: border-box; padding: 7px 10px; border-radius: 6px;
  border: 1px solid var(--border-soft); background: var(--bg-window); color: var(--text); font-size: 13px;
}
.um-input:focus { outline: none; border-color: var(--accent); }
.um-error { font-size: 12px; color: var(--danger-text); }

.um-table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
.um-table th { text-align: left; font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; padding: 6px 8px; border-bottom: 1px solid var(--border); }
.um-table td { padding: 6px 8px; border-bottom: 1px solid var(--grid-line); color: var(--text); user-select: text; }
.um-roles { color: var(--text-dim); }
.um-act { text-align: right; }

.btn { height: 28px; padding: 0 12px; border-radius: 6px; border: 1px solid var(--border-soft); background: var(--bg-input); color: var(--text); font-size: 12.5px; cursor: pointer; display: inline-flex; align-items: center; gap: 5px; }
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:disabled { opacity: .55; cursor: default; }
.danger-btn.armed { background: var(--danger-text); border-color: var(--danger-text); color: #fff; }
</style>
