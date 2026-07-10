<script setup>
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { mongoStringify } from '../utils/mongoFormat'
import { createJsonView, setJsonView } from '../utils/jsonView'

// Read-only JSON view: a CodeMirror editor (see utils/jsonView.js) that virtualizes
// lines and folds objects/arrays natively. The buffer is each document rendered at the
// top level (mongosh-style), one after another — no enclosing array wrapper; the editor
// draws a divider at each top-level closing brace. The parent mounts this only while the
// JSON view is actually on screen, so mount/unmount owns the editor's lifetime.
const props = defineProps({
  results: { type: Array, default: () => [] },
})

const hostEl = ref(null)
let view = null

const text = computed(() => {
  const results = props.results
  if (!results || !results.length) return ''
  return results.map((doc) => mongoStringify(doc)).join('\n')
})

// Build the editor once the host div is in the DOM, push fresh text while it stays,
// and tear it down when the buffer empties. If the previous editor's DOM was detached
// (host div swapped out and remounted), drop it and rebuild in place.
watch(text, async (t) => {
  if (!t.length) {
    if (view) { view.destroy(); view = null }
    return
  }
  await nextTick()
  if (!hostEl.value) return
  if (view && view.dom.parentElement !== hostEl.value) {
    view.destroy()
    view = null
  }
  if (view) setJsonView(view, t)
  else view = createJsonView(hostEl.value, t)
}, { immediate: true })

onBeforeUnmount(() => { if (view) { view.destroy(); view = null } })
</script>

<template>
  <div class="json-view">
    <div v-if="!results?.length" class="json-empty">No documents</div>
    <div v-else ref="hostEl" class="json-cm"></div>
  </div>
</template>

<style scoped>
.json-view { flex: 1; min-height: 0; display: flex; overflow: hidden; }
.json-cm { flex: 1; min-height: 0; overflow: hidden; }
.json-empty { padding: 32px; color: var(--text-faint); font-size: 12px; }
</style>
