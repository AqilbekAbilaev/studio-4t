<script setup>
import { computed, ref } from 'vue'
import ExplainGraph from './ExplainGraph.vue'
import JsonDoc from './JsonDoc.vue'
import { buildExplainTree } from '../utils/explainTree'

// Explain sub-tab: the query's execution plan, shown either as a stage graph or the raw
// plan document. Parsing lives in buildExplainTree; ExplainGraph only draws the tree.
const props = defineProps({
  activeTab: { type: Object, required: true },
})

const emit = defineEmits(['explain-verbosity'])

// The explain document normalized into a render-ready node-graph tree (Result root +
// stage tree).
const explainTree = computed(() =>
  buildExplainTree(
    props.activeTab && props.activeTab.explainResult,
    props.activeTab && props.activeTab.explainStorage,
  )
)

// Sub-tab view mode: 'graph' (default) or 'json' (the raw plan document).
const explainView = ref('graph')
</script>

<template>
  <div class="explain-view">
    <div v-if="activeTab.explainRunning" class="explain-msg">Running explain…</div>
    <div v-else-if="activeTab.explainError" class="run-error">{{ activeTab.explainError }}</div>
    <template v-else-if="activeTab.explainResult">
      <div class="explain-toolbar">
        <div class="explain-toggle" role="group" aria-label="Explain view">
          <button
            type="button"
            class="et-btn"
            :class="{ on: explainView === 'graph' }"
            :aria-pressed="explainView === 'graph'"
            @click="explainView = 'graph'"
          >Graph</button>
          <button
            type="button"
            class="et-btn"
            :class="{ on: explainView === 'json' }"
            :aria-pressed="explainView === 'json'"
            @click="explainView = 'json'"
          >View JSON</button>
        </div>
        <span class="et-spacer"></span>
        <label class="et-verbosity">
          <span class="et-verbosity-label">Detail</span>
          <select
            class="et-select"
            :value="activeTab.explainVerbosity || 'executionStats'"
            @change="emit('explain-verbosity', $event.target.value)"
          >
            <option value="executionStats">Execution stats</option>
            <option value="queryPlanner">Query planner</option>
            <option value="allPlansExecution">All plans</option>
          </select>
        </label>
      </div>
      <ExplainGraph v-if="explainView === 'graph'" :tree="explainTree" />
      <JsonDoc v-else class="json-doc" :value="activeTab.explainResult" />
    </template>
    <div v-else class="explain-msg">Run a query, then this tab shows its execution plan.</div>
  </div>
</template>

<style scoped>
.explain-view { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
.explain-view > .json-doc { flex: 1; overflow: auto; padding: 12px 16px; }
.explain-msg { padding: 32px; color: var(--text-faint); font-size: 12px; display: flex; align-items: center; justify-content: center; }
.run-error { padding: 10px 14px; color: var(--danger-text); font-size: 12px; }

/* Graph / View JSON toggle + verbosity select */
.explain-toolbar { display: flex; align-items: center; padding: 8px 12px; border-bottom: 1px solid var(--border-soft); flex: 0 0 auto; }
.explain-toggle { display: inline-flex; border: 1px solid var(--border-soft); border-radius: 7px; overflow: hidden; }
.et-btn {
  appearance: none;
  border: none;
  background: transparent;
  padding: 5px 12px;
  font-size: 12px;
  color: var(--text-dim);
  cursor: pointer;
}
.et-btn + .et-btn { border-left: 1px solid var(--border-soft); }
.et-btn:hover { background: var(--bg-hover); color: var(--text); }
.et-btn.on { background: var(--accent); color: #fff; }
.et-spacer { flex: 1; }
.et-verbosity { display: inline-flex; align-items: center; gap: 7px; }
.et-verbosity-label { font-size: 11px; color: var(--text-dim); }
.et-select {
  appearance: none;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  color: var(--text);
  font-size: 12px;
  padding: 4px 9px;
  cursor: pointer;
}
.et-select:focus { outline: none; border-color: var(--accent); }
</style>
