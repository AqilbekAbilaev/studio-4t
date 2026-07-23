<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import Disclosure from '../base/Disclosure.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import FormField from '../base/FormField.vue'

// The Compare tab diffs two collections in a database by _id, the way Studio-3T's Data
// Compare does. Each tab targets its own database and reloads the collection list on retarget.
const props = defineProps({
  activeTab: { type: Object, required: true },  // { connId, connName, dbName }
})

const collections = ref([])
const collectionOptions = computed(() => collections.value.map((c) => ({ value: c, label: c })))
const source = ref('')
const targetColl = ref('')
const result = ref(null)
const loading = ref(false)
const initErr = ref(null)
const error = ref(null)
const errorCode = ref(null)
const expanded = ref({})

async function loadCollections() {
  initErr.value = null
  result.value = null
  try {
    const dbs = await invoke('list_databases', { id: props.activeTab.connId })
    const db = (dbs || []).find(d => d.name === props.activeTab.dbName)
    collections.value = (db && db.collections) ? db.collections : []
    if (collections.value.length) source.value = collections.value[0]
    if (collections.value.length > 1) targetColl.value = collections.value[1]
    else if (collections.value.length) targetColl.value = collections.value[0]
  } catch (e) {
    initErr.value = errText(e)
  }
}

onMounted(loadCollections)
watch(() => props.activeTab.connId + ':' + props.activeTab.dbName, loadCollections)

async function compare() {
  if (!source.value || !targetColl.value) return
  loading.value = true
  error.value = null
  errorCode.value = null
  result.value = null
  expanded.value = {}
  try {
    result.value = await invoke('compare_collections', {
      id: props.activeTab.connId,
      database: props.activeTab.dbName,
      source: source.value,
      target: targetColl.value,
    })
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
}

function toggle(key) {
  expanded.value = { ...expanded.value, [key]: !expanded.value[key] }
}

function j(v) {
  return JSON.stringify(v, null, 2)
}
</script>

<template>
  <div class="compare-pane">
    <!-- Breadcrumb -->
    <div class="crumbs">
      <BaseIcon name="connect" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.connName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="dbSmall" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.dbName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="compare" :size="15" class="c-ic" />
      <span class="crumb">Compare</span>
    </div>

    <div class="cm-body">
      <StateMessage v-if="initErr" mode="error" :message="initErr" />
      <template v-else>
        <div class="cm-pick">
          <FormField label="Source">
            <BaseSelect v-model="source" class="cm-select" :options="collectionOptions" />
          </FormField>
          <BaseIcon name="compare" :size="16" class="cm-vs" />
          <FormField label="Target">
            <BaseSelect v-model="targetColl" class="cm-select" :options="collectionOptions" />
          </FormField>
          <BaseButton variant="primary" :disabled="loading || !source || !targetColl" @click="compare">
            {{ loading ? 'Comparing…' : 'Compare' }}
          </BaseButton>
        </div>

        <StateMessage v-if="loading" mode="loading" label="Comparing collections…" />
        <StateMessage v-else-if="error" mode="error" :message="error" :code="errorCode" />

        <template v-else-if="result">
          <div class="cm-summary">
            <div class="cm-card"><div class="cm-n">{{ result.identical_count }}</div><div class="cm-l">Identical</div></div>
            <div class="cm-card diff"><div class="cm-n">{{ result.differing_count }}</div><div class="cm-l">Differing</div></div>
            <div class="cm-card add"><div class="cm-n">{{ result.only_in_source_count }}</div><div class="cm-l">Only in source</div></div>
            <div class="cm-card rem"><div class="cm-n">{{ result.only_in_target_count }}</div><div class="cm-l">Only in target</div></div>
          </div>

          <div class="cm-sections">
            <div class="cm-sec" v-if="result.differing.length">
              <Disclosure :model-value="expanded.diff" @update:model-value="toggle('diff')">
                Differing ({{ result.differing_count }})
              </Disclosure>
              <div v-if="expanded.diff" class="cm-sec-body">
                <div v-for="p in result.differing" :key="p.id" class="cm-pair">
                  <div class="cm-pair-id">_id: {{ p.id }}</div>
                  <div class="cm-pair-cols">
                    <pre class="cm-doc">{{ j(p.source) }}</pre>
                    <pre class="cm-doc">{{ j(p.target) }}</pre>
                  </div>
                </div>
                <div v-if="result.differing_count > result.differing.length" class="cm-more">+{{ result.differing_count - result.differing.length }} more not shown</div>
              </div>
            </div>

            <div class="cm-sec" v-if="result.only_in_source.length">
              <Disclosure :model-value="expanded.src" @update:model-value="toggle('src')">
                Only in source ({{ result.only_in_source_count }})
              </Disclosure>
              <div v-if="expanded.src" class="cm-sec-body">
                <pre v-for="(d, i) in result.only_in_source" :key="i" class="cm-doc">{{ j(d) }}</pre>
                <div v-if="result.only_in_source_count > result.only_in_source.length" class="cm-more">+{{ result.only_in_source_count - result.only_in_source.length }} more not shown</div>
              </div>
            </div>

            <div class="cm-sec" v-if="result.only_in_target.length">
              <Disclosure :model-value="expanded.tgt" @update:model-value="toggle('tgt')">
                Only in target ({{ result.only_in_target_count }})
              </Disclosure>
              <div v-if="expanded.tgt" class="cm-sec-body">
                <pre v-for="(d, i) in result.only_in_target" :key="i" class="cm-doc">{{ j(d) }}</pre>
                <div v-if="result.only_in_target_count > result.only_in_target.length" class="cm-more">+{{ result.only_in_target_count - result.only_in_target.length }} more not shown</div>
              </div>
            </div>
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.compare-pane { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

.crumbs {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 14px; font-size: 12.5px; color: var(--text-dim);
  border-bottom: 1px solid var(--border); flex: none;
}
.sep { color: var(--text-faint); }
.c-ic { color: var(--text-faint); }

.cm-body { flex: 1; min-height: 0; overflow: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 12px; }
.cm-pick { display: flex; align-items: flex-end; gap: 12px; }
.cm-select { width: 100%; }
.cm-vs { color: var(--text-faint); margin-bottom: 6px; }

.cm-summary { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.cm-card {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  text-align: center;
}
.cm-card.diff { border-color: var(--warn); }
.cm-card.add { border-color: var(--green); }
.cm-card.rem { border-color: var(--danger-text); }
.cm-n { font-size: 20px; color: var(--text); }
.cm-l { font-size: 11px; color: var(--text-faint); text-transform: uppercase; letter-spacing: .04em; margin-top: 2px; }

.cm-sections { display: flex; flex-direction: column; gap: 8px; }
.cm-sec { border: 1px solid var(--border-soft); border-radius: 6px; overflow: hidden; }
.cm-sec-body { padding: 8px 10px; display: flex; flex-direction: column; gap: 8px; background: var(--bg-panel-2); }
.cm-pair { display: flex; flex-direction: column; gap: 4px; }
.cm-pair-id { font-family: var(--mono); font-size: 12px; color: var(--text-dim); }
.cm-pair-cols { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.cm-doc {
  margin: 0;
  font-family: var(--mono);
  font-size: 11.5px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-dim);
  white-space: pre;
  overflow: auto;
  max-height: 220px;
  user-select: text;
}
.cm-more { font-size: 12px; color: var(--text-faint); }
</style>
