<script setup>
import TreeView from './base/TreeView.vue'

// Tree view: each result document rendered as an expandable Key / Value / Type tree
// under a sticky column header. TreeView owns the recursive row rendering.
defineProps({
  results: { type: Array, default: () => [] },
})
</script>

<template>
  <div class="tree-view">
    <div v-if="!results?.length" class="tree-empty">No documents</div>
    <template v-else>
      <div class="tree-head">
        <span class="th-key">Key</span>
        <span class="th-val">Value</span>
        <span class="th-type">Type</span>
      </div>
      <div class="tree-body">
        <TreeView
          v-for="(doc, i) in results"
          :key="i"
          :label="`(${i + 1})`"
          :value="doc"
          :depth="0"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.tree-view { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: auto; background: var(--bg-window); }
.tree-empty { padding: 32px; color: var(--text-faint); font-size: 12px; }
.tree-head {
  display: grid;
  grid-template-columns: minmax(220px, 1.4fr) minmax(160px, 2fr) 110px;
  position: sticky;
  top: 0;
  z-index: 2;
  height: 26px;
  align-items: center;
  background: var(--bg-toolbar);
  color: var(--text-dim);
  font-weight: 600;
  font-size: 11px;
  border-bottom: 1px solid var(--border);
}
.tree-head span { padding: 0 8px; border-right: 1px solid var(--border); height: 100%; display: flex; align-items: center; }
.tree-head .th-type { border-right: none; }
</style>
