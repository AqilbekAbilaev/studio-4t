<script setup>
import { computed } from 'vue'
import { mongoStringify } from '../../utils/mongoFormat'
import { jsonViewerExtensions } from '../../utils/jsonView'
import CodeEditor from '../base/CodeEditor.vue'

// Read-only JSON view: a CodeEditor (readonly, highlight="json") that virtualizes lines
// and folds objects/arrays natively. The buffer is each document rendered at the top level
// (mongosh-style), one after another — no enclosing array wrapper; a divider is drawn at
// each top-level document (see utils/jsonView.js).
const props = defineProps({
  results: { type: Array, default: () => [] },
})

const text = computed(() => {
  const results = props.results
  if (!results || !results.length) return ''
  return results.map((doc) => mongoStringify(doc)).join('\n')
})

// Stable extension set for the single editor instance.
const jsonExtensions = jsonViewerExtensions()
</script>

<template>
  <div class="json-view">
    <div v-if="!results?.length" class="json-empty">No documents</div>
    <CodeEditor v-else class="json-cm" :model-value="text" readonly highlight="json" :extensions="jsonExtensions" />
  </div>
</template>

<style scoped>
.json-view { flex: 1; min-height: 0; display: flex; overflow: hidden; }
.json-cm { flex: 1; min-height: 0; overflow: hidden; }
.json-empty { padding: 32px; color: var(--text-faint); font-size: 12px; }
</style>
