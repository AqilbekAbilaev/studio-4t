<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage, errCode } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import StateMessage from './StateMessage.vue'

// Top-bar "Search in…" for the active database. Scans every collection for a
// value anywhere in a document (case-insensitive), the way Studio-3T does.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const term = ref('')
const results = ref(null)
const loading = ref(false)
const error = ref(null)
const errorCode = ref(null)
const expanded = ref({})

async function search() {
  const t = term.value.trim()
  if (!t) return
  loading.value = true
  error.value = null
  errorCode.value = null
  results.value = null
  expanded.value = {}
  try {
    results.value = await invoke('search_collections', {
      id: props.target.connId,
      database: props.target.dbName,
      term: t,
    })
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
}

function toggle(name) {
  expanded.value = { ...expanded.value, [name]: !expanded.value[name] }
}

function preview(doc) {
  return JSON.stringify(doc, null, 2)
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Search — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="se-body">
        <div class="se-bar">
          <input
            v-model="term"
            class="se-input"
            placeholder="Search all collections for a value…"
            spellcheck="false"
            @keydown.enter="search"
          />
          <button class="se-run" :disabled="loading || !term.trim()" @click="search">
            {{ loading ? 'Searching…' : 'Search' }}
          </button>
        </div>

        <StateMessage v-if="loading" mode="loading" label="Scanning collections…" />
        <StateMessage v-else-if="error" mode="error" :message="error" :code="errorCode" />
        <StateMessage
          v-else-if="results && !results.length"
          mode="empty"
          label="No matches found"
        />
        <div v-else-if="results" class="se-results">
          <div v-for="r in results" :key="r.collection" class="se-group">
            <button class="se-grp-head" @click="toggle(r.collection)">
              <BaseIcon :name="expanded[r.collection] ? 'caretDown' : 'caret'" :size="11" />
              <span class="se-coll">{{ r.collection }}</span>
              <span class="se-count">{{ r.matched }} match{{ r.matched === 1 ? '' : 'es' }} (scanned {{ r.scanned }})</span>
            </button>
            <div v-if="expanded[r.collection]" class="se-hits">
              <pre v-for="(doc, i) in r.hits" :key="i" class="se-doc">{{ preview(doc) }}</pre>
              <div v-if="r.matched > r.hits.length" class="se-more">
                +{{ r.matched - r.hits.length }} more not shown
              </div>
            </div>
          </div>
        </div>
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

.se-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 200px;
  max-height: 74vh;
  overflow: hidden;
}
.se-bar { display: flex; gap: 10px; }
.se-input {
  flex: 1;
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 10px;
  font-size: 13px;
}
.se-input:focus { outline: none; border-color: var(--accent); }
.se-run {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12.5px;
  cursor: pointer;
}
.se-run:hover { background: var(--accent-soft); }
.se-run:disabled { opacity: .6; cursor: default; }

.se-results { overflow-y: auto; display: flex; flex-direction: column; gap: 6px; }
.se-group { border: 1px solid var(--border-soft); border-radius: 6px; overflow: hidden; }
.se-grp-head {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-panel);
  border: none;
  color: var(--text);
  padding: 8px 10px;
  cursor: pointer;
  font-size: 12.5px;
  text-align: left;
}
.se-grp-head:hover { background: var(--bg-hover); }
.se-coll { font-family: var(--mono); }
.se-count { margin-left: auto; color: var(--text-faint); font-size: 12px; }
.se-hits {
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--bg-panel-2);
}
.se-doc {
  margin: 0;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-dim);
  white-space: pre;
  overflow-x: auto;
  max-height: 240px;
  overflow-y: auto;
  user-select: text;
}
.se-more { font-size: 12px; color: var(--text-faint); }
</style>
