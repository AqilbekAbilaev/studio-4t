<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import Disclosure from '../base/Disclosure.vue'
import StateMessage from '../base/StateMessage.vue'

// The Search tab scans every collection in a database for a value anywhere in a document
// (case-insensitive), the way Studio-3T's "Search in…" does. Each tab targets its own database.
const props = defineProps({
  activeTab: { type: Object, required: true },  // { connId, connName, dbName }
})

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
      id: props.activeTab.connId,
      database: props.activeTab.dbName,
      term: t,
    })
  } catch (e) {
    error.value = errText(e)
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
  <div class="search-pane">
    <!-- Breadcrumb -->
    <div class="crumbs">
      <BaseIcon name="connect" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.connName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="dbSmall" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.dbName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="search" :size="15" class="c-ic" />
      <span class="crumb">Search</span>
    </div>

    <div class="se-body">
      <div class="se-bar">
        <BaseInput
          v-model="term"
          class="se-input"
          placeholder="Search all collections for a value…"
          @enter="search"
        />
        <BaseButton variant="primary" :disabled="loading || !term.trim()" @click="search">
          {{ loading ? 'Searching…' : 'Search' }}
        </BaseButton>
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
          <Disclosure :model-value="expanded[r.collection]" @update:model-value="toggle(r.collection)">
            <span class="se-coll">{{ r.collection }}</span>
            <span class="se-count">{{ r.matched }} match{{ r.matched === 1 ? '' : 'es' }} (scanned {{ r.scanned }})</span>
          </Disclosure>
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
</template>

<style scoped>
.search-pane { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

.crumbs {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 14px; font-size: 12.5px; color: var(--text-dim);
  border-bottom: 1px solid var(--border); flex: none;
}
.sep { color: var(--text-faint); }
.c-ic { color: var(--text-faint); }

.se-body { flex: 1; min-height: 0; overflow: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 12px; }
.se-bar { display: flex; gap: 10px; }
.base-input.se-input { flex: 1; }

.se-results { display: flex; flex-direction: column; gap: 6px; }
.se-group { border: 1px solid var(--border-soft); border-radius: 6px; overflow: hidden; }
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
