<script setup>
// A single pane in the split workspace: the focus-tracking `pane-host` wrapper plus a
// QueryWorkspace. Centralizes the "only the focused pane reacts to global requests"
// gating (docMenu/history/browser/saveQuery) and re-emits the pane-scoped events with its
// own `paneId` so App.vue can bind the pane-aware handlers directly. Extracted to remove
// the two near-identical split-pane blocks that previously lived in App.vue's template.
import QueryWorkspace from '../query/QueryWorkspace.vue'

defineProps({
  paneId: { type: String, required: true },
  tabs: { type: Array, default: () => [] },
  activeTabId: { type: String, default: null },
  focused: { type: Boolean, default: false },
  tagOverrides: { type: Object, default: () => ({}) },
  vqbOpen: { type: Boolean, default: false },
  clipboardQuery: { type: [Object, Array, String], default: null },
  docMenuRequest: { type: [Object, Number], default: null },
  historyRequest: { type: [Object, Number], default: null },
  browserRequest: { type: [Object, Number], default: null },
  saveQueryRequest: { type: [Object, Number], default: null },
})

defineEmits([
  'focus', 'activate-tab', 'close-tab', 'tab-context',
  'run-query', 'run-aggregate', 'cancel-query',
  'toggle-vqb', 'open-vqb', 'close-vqb',
  'toast', 'copy-query', 'paste-query', 'follow-reference',
])
</script>

<template>
  <div class="pane-host" :class="{ focused: focused }" @mousedown.capture="$emit('focus', paneId)">
    <QueryWorkspace
      :tabs="tabs"
      :active-tab-id="activeTabId"
      :tag-overrides="tagOverrides"
      :vqb-open="focused && vqbOpen"
      :clipboard-query="clipboardQuery"
      :doc-menu-request="focused ? docMenuRequest : null"
      :history-request="focused ? historyRequest : null"
      :browser-request="focused ? browserRequest : null"
      :save-query-request="focused ? saveQueryRequest : null"
      @activate-tab="$emit('activate-tab', paneId, $event)"
      @close-tab="$emit('close-tab', paneId, $event)"
      @tab-context="$emit('tab-context', paneId, $event)"
      @run-query="$emit('run-query', $event)"
      @run-aggregate="$emit('run-aggregate', $event)"
      @cancel-query="$emit('cancel-query', $event)"
      @toggle-vqb="$emit('toggle-vqb')"
      @open-vqb="$emit('open-vqb')"
      @close-vqb="$emit('close-vqb')"
      @toast="$emit('toast', $event)"
      @copy-query="$emit('copy-query', $event)"
      @paste-query="$emit('paste-query', $event)"
      @follow-reference="$emit('follow-reference', $event)"
    />
  </div>
</template>

<style scoped>
.pane-host { display: flex; flex: 1; min-width: 0; min-height: 0; position: relative; }
.pane-host.focused::after {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  box-shadow: inset 0 0 0 1px var(--accent);
  z-index: 5;
}
</style>
