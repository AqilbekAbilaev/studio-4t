<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import BaseInput from '../base/BaseInput.vue'
import Disclosure from '../base/Disclosure.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'
import BaseModalBody from '../base/BaseModalBody.vue'

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
  <BaseModal :title="`Search — ${target.dbName}`" width="680px" max-width="92vw" @close="$emit('close')">

      <BaseModalBody>
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
      </BaseModalBody>
    </BaseModal>
</template>

<style scoped>


.se-bar { display: flex; gap: 10px; }
.base-input.se-input { flex: 1; }

.se-results { overflow-y: auto; display: flex; flex-direction: column; gap: 6px; }
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
