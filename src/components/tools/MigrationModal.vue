<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Top-bar "SQL Migration" for the active collection. Generates a CREATE TABLE +
// INSERT script from the collection, the way Studio-3T's SQL Migration does.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName, collName }
})
defineEmits(['close'])

const tableName = ref('')
const limit = ref(1000)
const sql = ref('')
const loading = ref(false)
const error = ref(null)
const errorCode = ref(null)
const copied = ref(false)

onMounted(() => {
  tableName.value = props.target.collName
  generate()
})

async function generate() {
  loading.value = true
  error.value = null
  errorCode.value = null
  copied.value = false
  try {
    const trimmed = String(limit.value).trim()
    sql.value = await invoke('generate_sql_migration', {
      id: props.target.connId,
      database: props.target.dbName,
      collection: props.target.collName,
      tableName: tableName.value.trim() || props.target.collName,
      limit: trimmed ? Number(trimmed) : null,
    })
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
    sql.value = ''
  } finally {
    loading.value = false
  }
}

async function copy() {
  try {
    await navigator.clipboard.writeText(sql.value)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1500)
  } catch (_) {}
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">SQL Migration — {{ target.dbName }}.{{ target.collName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="mg-body">
        <div class="mg-controls">
          <label class="mg-f">
            Table name
            <input v-model="tableName" class="mg-input" spellcheck="false" />
          </label>
          <label class="mg-f">
            Limit
            <input v-model="limit" type="number" min="1" class="mg-input num" />
          </label>
          <button class="mg-run" :disabled="loading" @click="generate">
            {{ loading ? 'Generating…' : 'Generate' }}
          </button>
          <button v-if="sql" class="mg-copy" @click="copy">
            <BaseIcon :name="copied ? 'check' : 'copy'" :size="12" />
            {{ copied ? 'Copied' : 'Copy' }}
          </button>
        </div>

        <StateMessage v-if="loading" mode="loading" label="Generating SQL…" />
        <StateMessage v-else-if="error" mode="error" :message="error" :code="errorCode" />
        <pre v-else-if="sql" class="mg-sql">{{ sql }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 70;
}
.dialog {
  width: 680px;
  max-width: 92vw;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
  position: relative;
}
.dlg-title .t {
  position: absolute;
  left: 0; right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}
.close-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  border-radius: 4px;
  z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }

.mg-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: 74vh;
  overflow: hidden;
}
.mg-controls {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}
.mg-f {
  font-size: 12px;
  color: var(--text-dim);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.mg-input {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 5px 8px;
  font-size: 12.5px;
}
.mg-input.num { width: 90px; }
.mg-input:focus { outline: none; border-color: var(--accent); }
.mg-run {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.mg-run:hover { background: var(--accent-soft); }
.mg-run:disabled { opacity: .6; cursor: default; }
.mg-copy {
  display: flex;
  align-items: center;
  gap: 5px;
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 5px;
  padding: 5px 8px;
  font-size: 11.5px;
  cursor: pointer;
  margin-left: auto;
}
.mg-copy:hover { background: var(--bg-hover); color: var(--text); }
.mg-sql {
  margin: 0;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  color: var(--text-dim);
  white-space: pre;
  overflow: auto;
  user-select: text;
}
</style>
