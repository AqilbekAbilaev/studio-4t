<script setup>
import BaseIcon from './BaseIcon.vue'

defineProps({
  tabs:        { type: Array,  required: true },
  activeTabId: { type: String, required: true },
})
const emit = defineEmits(['activate-tab', 'close-tab'])
</script>

<template>
  <div class="tabs">
    <button
      v-for="t in tabs"
      :key="t.id"
      class="tab"
      :class="{ active: t.id === activeTabId }"
      @click="emit('activate-tab', t.id)"
    >
      <span>{{ t.title }}</span>
      <span class="x" @click.stop="emit('close-tab', t.id)">
        <BaseIcon name="close" :size="12" />
      </span>
    </button>
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  align-items: stretch;
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
  height: 32px;
  flex: none;
  padding-left: 6px;
  overflow-x: auto;
}
.tab {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  font-size: 12.5px;
  color: var(--text-dim);
  border-right: 1px solid var(--border);
  border-top: none;
  border-bottom: 2px solid transparent;
  border-left: none;
  background: none;
  max-width: 220px;
  white-space: nowrap;
  flex-shrink: 0;
}
.tab.active { background: var(--bg-window); color: var(--text); border-bottom-color: var(--accent); }
.tab .x { color: var(--text-faint); border-radius: 4px; padding: 1px; display: grid; place-items: center; }
.tab .x:hover { background: var(--bg-hover); color: var(--text); }
</style>
