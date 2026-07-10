<script setup>
// Collection → Update Dialog. Runs an update against every document matching a
// query filter, using an operator update document ({ $set: … }). Query text is shell
// syntax, parsed to Extended JSON by the shared query parser (same as the query bar).
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../utils/errors'
import { parseField } from '../utils/queryParser'
import BaseIcon from './base/BaseIcon.vue'

const props = defineProps({
  activeTab: { type: Object, required: true },
})
const emit = defineEmits(['close', 'done'])

const filter = ref('{}')
const update = ref('{\n  "$set": {\n    \n  }\n}')
const error  = ref(null)
const busy   = ref(false)
// Scope guard, mirroring the Delete Dialog: the user must "Find matches" (a count)
// for the current query before Update enables, so a mass update (default {} matches
// all) can never run against a scope they haven't seen. Editing the query
// invalidates the count. The count reflects the filter only — the update operators
// don't change which documents are matched.
const matched = ref(null)          // count for `countedFilter`, or null before/after edits
const countedFilter = ref(null)    // the exact filter text `matched` was counted for

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

async function onRun() {
  // Guard: only update what was counted for the current, unchanged query.
  if (matched.value === null || countedFilter.value !== filter.value) return
  error.value = null
  const pf = parseField(filter.value)
  if (!pf.ok) { error.value = 'Query: ' + pf.error; return }
  const pu = parseField(update.value)
  if (!pu.ok) { error.value = 'Update: ' + pu.error; return }
  busy.value = true
  try {
    const modified = await invoke('update_many', {
      id:         props.activeTab.connectionId,
      database:   props.activeTab.dbName,
      collection: props.activeTab.collectionName,
      filter:     pf.ejson,
      update:     pu.ejson,
    })
    emit('done', `Updated ${modified} document${modified !== 1 ? 's' : ''}`)
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
        <div class="t">Update Documents</div>
        <button class="close-btn" @click="$emit('close')"><BaseIcon name="close" :size="14" /></button>
      </div>

      <div class="uw-body">
        <div class="uw-hint">Updates every document in
          <code>{{ activeTab.collectionName }}</code> that matches the query.</div>

        <label class="uw-row">
          <span class="uw-lbl">Query</span>
          <textarea class="uw-input" v-model="filter" spellcheck="false" autocomplete="off"></textarea>
        </label>

        <label class="uw-row">
          <span class="uw-lbl">Update (operators, e.g. <code>{ "$set": { "field": 1 } }</code>)</span>
          <textarea class="uw-input uw-tall" v-model="update" spellcheck="false" autocomplete="off"></textarea>
        </label>

        <div class="uw-count" v-if="matched !== null && countedFilter === filter">
          <BaseIcon name="count" :size="14" />
          {{ matched.toLocaleString() }} document{{ matched !== 1 ? 's' : '' }} match this query.
        </div>

        <div v-if="error" class="uw-error">{{ error }}</div>
      </div>

      <div class="uw-footer">
        <button class="btn" :disabled="busy" @click="onCount">Find matches</button>
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button
          class="btn primary"
          :disabled="busy || matched === null || countedFilter !== filter"
          @click="onRun"
        >{{ busy ? 'Updating…' : (matched !== null && countedFilter === filter ? `Update ${matched.toLocaleString()}` : 'Update') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 60; }
.dialog {
  width: 560px; max-width: 94vw;
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
.uw-body { padding: 14px 18px 8px; display: flex; flex-direction: column; gap: 12px; }
.uw-hint { font-size: 12.5px; color: var(--text-dim); }
.uw-hint code, .uw-lbl code { font-family: var(--mono); color: var(--text); }
.uw-row { display: flex; flex-direction: column; gap: 5px; }
.uw-lbl { font-size: 11px; text-transform: uppercase; letter-spacing: .4px; color: var(--text-faint); }
.uw-input {
  background: var(--bg-input); border: 1px solid var(--border); border-radius: 5px;
  color: var(--text); font-family: var(--mono); font-size: 12.5px; padding: 8px 10px;
  outline: none; resize: vertical; min-height: 60px; line-height: 1.5;
}
.uw-input:focus { border-color: var(--accent); }
.uw-tall { min-height: 120px; }
.uw-count { display: flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--text); }
.uw-error { font-size: 12px; color: var(--danger-text); }
.uw-footer {
  height: 48px; flex: none; border-top: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 16px; gap: 8px; margin-top: 8px;
}
.spacer { flex: 1; }
.btn {
  height: 28px; padding: 0 14px; border-radius: 5px; border: none;
  font-size: 13px; cursor: pointer; background: var(--bg-toolbar); color: var(--text);
}
.btn:hover { background: var(--bg-hover); }
.btn:disabled { opacity: .5; cursor: default; }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover:not(:disabled) { opacity: .88; }
</style>
