<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import BaseButton from '../base/BaseButton.vue'
import Disclosure from '../base/Disclosure.vue'
import BaseSelect from '../base/BaseSelect.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModal from '../base/BaseModal.vue'
import FormField from '../base/FormField.vue'

// Top-bar "Compare" for a database: diff two collections by _id, the way
// Studio-3T's Data Compare does.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

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

onMounted(async () => {
  try {
    const dbs = await invoke('list_databases', { id: props.target.connId })
    const db = (dbs || []).find(d => d.name === props.target.dbName)
    collections.value = (db && db.collections) ? db.collections : []
    if (collections.value.length) source.value = collections.value[0]
    if (collections.value.length > 1) targetColl.value = collections.value[1]
    else if (collections.value.length) targetColl.value = collections.value[0]
  } catch (e) {
    initErr.value = errText(e)
  }
})

async function compare() {
  if (!source.value || !targetColl.value) return
  loading.value = true
  error.value = null
  errorCode.value = null
  result.value = null
  expanded.value = {}
  try {
    result.value = await invoke('compare_collections', {
      id: props.target.connId,
      database: props.target.dbName,
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
  <BaseModal :title="`Compare — ${target.dbName}`" width="760px" max-width="94vw" @close="$emit('close')">

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
    </BaseModal>
</template>

<style scoped>

.cm-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 220px;
  max-height: 76vh;
  overflow: hidden;
}
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

.cm-sections { overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }
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
