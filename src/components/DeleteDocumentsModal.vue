<script setup>
// Collection → Delete Dialog. Deletes every document matching a query filter. To
// scope the danger precisely, the user must first "Find matches" (a count) for the
// current query; only then does the Delete button enable, labelled with the exact
// number that will be removed. Editing the query invalidates the count so the delete
// can never run against a scope the user hasn't seen.
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { parseField } from '../utils/queryParser'
import BaseIcon from './base/BaseIcon.vue'

const props = defineProps({
  activeTab: { type: Object, required: true },
})
const emit = defineEmits(['close', 'done'])

const filter  = ref('{}')
const error   = ref(null)
const busy    = ref(false)
const matched = ref(null)          // count for `countedFilter`, or null before/after edits
const countedFilter = ref(null)    // the exact filter text `matched` was counted for

// Any edit to the query invalidates a prior count — the Delete button re-locks.
watch(filter, () => { matched.value = null; countedFilter.value = null })

async function onCount() {
  error.value = null
  const pf = parseField(filter.value)
  if (!pf.ok) { error.value = 'Query: ' + pf.error; return }
  busy.value = true
  try {
    const total = await invoke('count_documents', {
      id:         props.activeTab.connectionId,
      database:   props.activeTab.dbName,
      collection: props.activeTab.collectionName,
      filter:     pf.ejson,
    })
    matched.value = total
    countedFilter.value = filter.value
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    busy.value = false
  }
}

async function onDelete() {
  // Guard: only delete what was counted for the current, unchanged query.
  if (matched.value === null || countedFilter.value !== filter.value) return
  error.value = null
  const pf = parseField(filter.value)
  if (!pf.ok) { error.value = 'Query: ' + pf.error; return }
  busy.value = true
  try {
    const deleted = await invoke('delete_many', {
      id:         props.activeTab.connectionId,
      database:   props.activeTab.dbName,
      collection: props.activeTab.collectionName,
      filter:     pf.ejson,
    })
    emit('done', `Deleted ${deleted} document${deleted !== 1 ? 's' : ''}`)
  } catch (e) {
    error.value = errMessage(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Delete Documents</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="dw-body">
        <div class="dw-hint">Deletes every document in
          <code>{{ activeTab.collectionName }}</code> that matches the query. This cannot be undone.</div>

        <label class="dw-row">
          <span class="dw-lbl">Query <span class="dw-note">(empty <code>{}</code> matches all)</span></span>
          <textarea class="dw-input" v-model="filter" spellcheck="false" autocomplete="off"></textarea>
        </label>

        <div class="dw-count" v-if="matched !== null && countedFilter === filter">
          <BaseIcon name="count" :size="14" />
          {{ matched.toLocaleString() }} document{{ matched !== 1 ? 's' : '' }} match this query.
        </div>

        <div v-if="error" class="dw-error">{{ error }}</div>
      </div>

      <div class="dw-footer">
        <button class="btn" :disabled="busy" @click="onCount">Find matches</button>
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button
          class="btn danger"
          :disabled="busy || matched === null || countedFilter !== filter"
          @click="onDelete"
        >{{ matched !== null && countedFilter === filter ? `Delete ${matched.toLocaleString()}` : 'Delete' }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 60; }
.dialog {
  width: 540px; max-width: 94vw;
  background: var(--bg-window); border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex; flex-direction: column; overflow: hidden;
}
.dlg-title {
  height: 36px; flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 10px; position: relative;
}
.dlg-title .t {
  position: absolute; left: 0; right: 0; text-align: center;
  font-size: 13px; color: var(--text-dim); font-weight: 500; pointer-events: none;
}
.close-btn {
  margin-left: auto; background: none; border: none; color: var(--text-faint);
  cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
.dw-body { padding: 14px 18px 8px; display: flex; flex-direction: column; gap: 12px; }
.dw-hint { font-size: 12.5px; color: var(--text-dim); line-height: 1.5; }
.dw-hint code, .dw-lbl code, .dw-note code { font-family: var(--mono); color: var(--text); }
.dw-row { display: flex; flex-direction: column; gap: 5px; }
.dw-lbl { font-size: 11px; text-transform: uppercase; letter-spacing: .4px; color: var(--text-faint); }
.dw-note { text-transform: none; letter-spacing: 0; }
.dw-input {
  background: var(--bg-input); border: 1px solid var(--border); border-radius: 5px;
  color: var(--text); font-family: var(--mono); font-size: 12.5px; padding: 8px 10px;
  outline: none; resize: vertical; min-height: 60px; line-height: 1.5;
}
.dw-input:focus { border-color: var(--accent); }
.dw-count { display: flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--text); }
.dw-error { font-size: 12px; color: var(--danger-text); }
.dw-footer {
  height: 48px; flex: none; border-top: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 16px; gap: 8px; margin-top: 8px;
}
.spacer { flex: 1; }
.btn {
  height: 28px; padding: 0 14px; border-radius: 5px; border: none;
  font-size: 13px; cursor: pointer; background: var(--bg-toolbar); color: var(--text);
}
.btn:hover:not(:disabled) { background: var(--bg-hover); }
.btn:disabled { opacity: .5; cursor: default; }
.btn.danger { background: var(--danger); color: #fff; }
.btn.danger:hover:not(:disabled) { background: var(--danger-hover); }
</style>
