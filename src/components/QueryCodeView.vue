<script setup>
import { ref, computed } from 'vue'
import BaseIcon from './base/BaseIcon.vue'
import { generateCode, LANGUAGES } from '../utils/queryCodegen'

// Query Code sub-tab: the active tab's query rendered as a copy-ready snippet in a
// chosen target language (Shell, drivers, …). Generation lives in utils/queryCodegen.
const props = defineProps({
  activeTab: { type: Object, required: true },
})

const emit = defineEmits(['toast'])

// Target language for the generated snippet (session-scoped, defaults to Shell).
const queryCodeLang = ref('shell')

const queryCode = computed(() => {
  const tab = props.activeTab
  if (!tab || tab.kind !== 'collection') return ''
  return generateCode({
    collection: tab.collectionName,
    mode: tab.mode,
    filter: tab.filter,
    projection: tab.projection,
    sort: tab.sort,
    skip: tab.skip,
    limit: tab.limit,
    pipeline: tab.pipeline,
  }, queryCodeLang.value)
})

function copyQueryCode() {
  navigator.clipboard.writeText(queryCode.value ?? '')
    .then(() => emit('toast', 'Query code copied to clipboard'))
    .catch(() => emit('toast', 'Copy to clipboard failed'))
}
</script>

<template>
  <div class="qcode">
    <div class="qcode-toolbar">
      <label class="qc-lang">
        <span class="qc-lang-label">Language</span>
        <select class="qc-select" v-model="queryCodeLang">
          <option v-for="lang in LANGUAGES" :key="lang.id" :value="lang.id">{{ lang.label }}</option>
        </select>
      </label>
      <span class="qc-spacer"></span>
      <button class="qcode-copy" type="button" @click="copyQueryCode">
        <BaseIcon name="copy" :size="14" /> Copy
      </button>
    </div>
    <div class="qcode-view">
      <pre class="qcode-pre"><span v-if="queryCodeLang === 'shell'" class="qcode-prompt">&gt;</span>{{ queryCodeLang === 'shell' ? ' ' + queryCode : queryCode }}</pre>
    </div>
  </div>
</template>

<style scoped>
.qcode { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }

/* Toolbar shell + select (mirrors the Explain toolbar) */
.qcode-toolbar { display: flex; align-items: center; padding: 8px 12px; border-bottom: 1px solid var(--border-soft); flex: 0 0 auto; }
.qc-spacer { flex: 1; }
.qc-lang { display: inline-flex; align-items: center; gap: 7px; }
.qc-lang-label { font-size: 11px; color: var(--text-dim); }
.qc-select {
  appearance: none;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  color: var(--text);
  font-size: 12px;
  padding: 4px 9px;
  cursor: pointer;
}
.qc-select:focus { outline: none; border-color: var(--accent); }

.qcode-copy {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  appearance: none;
  background: transparent;
  padding: 5px 12px;
  font-size: 12px;
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  color: var(--text-dim);
  cursor: pointer;
}
.qcode-copy:hover { background: var(--bg-hover); color: var(--text); }
.qcode-view { flex: 1; overflow: auto; padding: 16px 20px; }
.qcode-pre {
  font-family: var(--mono);
  font-size: 13px;
  line-height: 1.7;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-all;
  -webkit-user-select: text;
  user-select: text;
}
.qcode-prompt { color: var(--text-faint); margin-right: 8px; }
</style>
