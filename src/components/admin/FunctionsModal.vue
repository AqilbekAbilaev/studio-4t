<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Add / Edit Stored Functions for a database (its system.js documents).
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const loading = ref(true)
const busy = ref(false)
const error = ref(null)
const functions = ref([])
const pendingDrop = ref(null)

// The function being edited (null = list view). name is blank for a new one.
const editing = ref(null)   // { name, body } | null
const editError = ref(null)

async function load() {
  loading.value = true
  error.value = null
  try {
    functions.value = await invoke('list_functions', { id: props.target.connId, database: props.target.dbName })
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    loading.value = false
  }
}

onMounted(load)

function newFunction() {
  editError.value = null
  editing.value = { name: '', body: 'function () {\n  \n}' }
}

function editFunction(fn) {
  editError.value = null
  editing.value = { name: fn.name, body: fn.body }
}

async function saveFunction() {
  const name = editing.value.name.trim()
  if (!name) { editError.value = 'A function name is required'; return }
  busy.value = true
  editError.value = null
  try {
    await invoke('save_function', {
      id: props.target.connId,
      database: props.target.dbName,
      name: name,
      body: editing.value.body,
    })
    editing.value = null
    await load()
  } catch (e) {
    editError.value = errMessage(e)
  } finally {
    busy.value = false
  }
}

async function dropFunction(fn) {
  if (pendingDrop.value !== fn.name) { pendingDrop.value = fn.name; return }
  busy.value = true
  try {
    await invoke('drop_function', { id: props.target.connId, database: props.target.dbName, name: fn.name })
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
        <div class="t">Stored Functions — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="fn-body">
        <!-- Editor -->
        <template v-if="editing">
          <input v-model="editing.name" class="fn-input" placeholder="Function name" spellcheck="false" :disabled="busy" />
          <textarea v-model="editing.body" class="fn-input fn-code" spellcheck="false" placeholder="function () { … }"></textarea>
          <div v-if="editError" class="fn-error">{{ editError }}</div>
          <div class="fn-actions">
            <button class="btn" @click="editing = null">Back</button>
            <button class="btn primary" :disabled="!editing.name.trim() || busy" @click="saveFunction">Save</button>
          </div>
        </template>

        <!-- List -->
        <template v-else>
          <div class="fn-bar">
            <button class="btn primary" :disabled="busy" @click="newFunction"><BaseIcon name="plus" :size="12" /> New Function</button>
          </div>
          <StateMessage v-if="loading" mode="loading" label="Loading functions…" />
          <StateMessage v-else-if="error" mode="error" :message="error" />
          <StateMessage v-else-if="!functions.length" mode="empty" label="No stored functions on this database" />
          <div v-else class="fn-list">
            <div v-for="fn in functions" :key="fn.name" class="fn-row">
              <span class="fn-name">{{ fn.name }}</span>
              <span class="fn-row-act">
                <button class="btn" :disabled="busy" @click="editFunction(fn)">Edit</button>
                <button class="btn danger-btn" :class="{ armed: pendingDrop === fn.name }" :disabled="busy" @click="dropFunction(fn)">
                  {{ pendingDrop === fn.name ? 'Confirm' : 'Delete' }}
                </button>
              </span>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 70; }
.dialog {
  width: 620px; max-width: 92vw; background: var(--bg-window); border-radius: 10px;
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

.fn-body { padding: 14px 16px 16px; display: flex; flex-direction: column; gap: 10px; min-height: 220px; max-height: 74vh; overflow-y: auto; }
.fn-bar { display: flex; }
.fn-input {
  width: 100%; box-sizing: border-box; padding: 8px 10px; border-radius: 6px;
  border: 1px solid var(--border-soft); background: var(--bg-input); color: var(--text); font-size: 13px;
}
.fn-input:focus { outline: none; border-color: var(--accent); }
.fn-code { min-height: 220px; font-family: var(--mono); font-size: 12px; line-height: 1.5; resize: vertical; }
.fn-error { font-size: 12px; color: var(--danger-text); }
.fn-actions { display: flex; justify-content: flex-end; gap: 8px; }

.fn-list { display: flex; flex-direction: column; }
.fn-row { display: flex; align-items: center; justify-content: space-between; padding: 7px 6px; border-bottom: 1px solid var(--grid-line); }
.fn-name { font-family: var(--mono); font-size: 12.5px; color: var(--text); }
.fn-row-act { display: flex; gap: 6px; }

.btn { height: 28px; padding: 0 12px; border-radius: 6px; border: 1px solid var(--border-soft); background: var(--bg-input); color: var(--text); font-size: 12.5px; cursor: pointer; display: inline-flex; align-items: center; gap: 5px; }
.btn:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:disabled { opacity: .55; cursor: default; }
.danger-btn.armed { background: var(--danger-text); border-color: var(--danger-text); color: #fff; }
</style>
