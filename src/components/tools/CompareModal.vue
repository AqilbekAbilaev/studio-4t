<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Top-bar "Compare" for a database: diff two collections by _id, the way
// Studio-3T's Data Compare does.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
defineEmits(['close'])

const collections = ref([])
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
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Compare — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="cm-body">
        <StateMessage v-if="initErr" mode="error" :message="initErr" />
        <template v-else>
          <div class="cm-pick">
            <label class="cm-f">
              Source
              <select v-model="source" class="cm-select">
                <option v-for="c in collections" :key="'s'+c" :value="c">{{ c }}</option>
              </select>
            </label>
            <BaseIcon name="compare" :size="16" class="cm-vs" />
            <label class="cm-f">
              Target
              <select v-model="targetColl" class="cm-select">
                <option v-for="c in collections" :key="'t'+c" :value="c">{{ c }}</option>
              </select>
            </label>
            <button class="cm-run" :disabled="loading || !source || !targetColl" @click="compare">
              {{ loading ? 'Comparing…' : 'Compare' }}
            </button>
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
                <button class="cm-sec-head" @click="toggle('diff')">
                  <BaseIcon :name="expanded.diff ? 'caretDown' : 'caret'" :size="11" />
                  Differing ({{ result.differing_count }})
                </button>
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
                <button class="cm-sec-head" @click="toggle('src')">
                  <BaseIcon :name="expanded.src ? 'caretDown' : 'caret'" :size="11" />
                  Only in source ({{ result.only_in_source_count }})
                </button>
                <div v-if="expanded.src" class="cm-sec-body">
                  <pre v-for="(d, i) in result.only_in_source" :key="i" class="cm-doc">{{ j(d) }}</pre>
                  <div v-if="result.only_in_source_count > result.only_in_source.length" class="cm-more">+{{ result.only_in_source_count - result.only_in_source.length }} more not shown</div>
                </div>
              </div>

              <div class="cm-sec" v-if="result.only_in_target.length">
                <button class="cm-sec-head" @click="toggle('tgt')">
                  <BaseIcon :name="expanded.tgt ? 'caretDown' : 'caret'" :size="11" />
                  Only in target ({{ result.only_in_target_count }})
                </button>
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
  width: 760px;
  max-width: 94vw;
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
.cm-f { font-size: 12px; color: var(--text-dim); display: flex; flex-direction: column; gap: 4px; flex: 1; }
.cm-select {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 5px 8px;
  font-size: 12.5px;
  width: 100%;
}
.cm-vs { color: var(--text-faint); margin-bottom: 6px; }
.cm-run {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12.5px;
  cursor: pointer;
}
.cm-run:hover { background: var(--accent-soft); }
.cm-run:disabled { opacity: .6; cursor: default; }

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
.cm-sec-head {
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
.cm-sec-head:hover { background: var(--bg-hover); }
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
