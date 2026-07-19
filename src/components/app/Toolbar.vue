<script setup>
// The global toolbar strip. Presentational only: renders the TOOLS buttons and emits
// `tool` with the clicked action id; App.vue routes that into its handleTool dispatcher.
// `hidden` is driven by the View → Hide Global Toolbar toggle.
import ToolbarButton from '../base/ToolbarButton.vue'
import { TOOLS } from '../../constants/tools'

defineProps({
  hidden: { type: Boolean, default: false },
})
defineEmits(['tool'])
</script>

<template>
  <div class="toolbar" v-show="!hidden">
    <template v-for="(t, i) in TOOLS" :key="i">
      <div v-if="t.sep" class="tb-sep"></div>
      <ToolbarButton v-else :icon="t.name" :label="t.label" :badge="t.badge" :drop="t.drop" :title="t.label" @click="$emit('tool', t.name)" />
    </template>
  </div>
</template>

<style scoped>
.toolbar {
  flex: none;
  background: var(--bg-toolbar);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: stretch;
  padding: 6px 8px;
  gap: 2px;
}
.tb-sep { width: 1px; flex: none; background: var(--border-soft); margin: 6px 4px; align-self: stretch; }
</style>
