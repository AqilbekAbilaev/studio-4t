<script setup>
// Displays a keyboard shortcut as a row of kbd chips; clicking starts capturing a
// new binding. The shortcut rows in the Shortcuts modal. Owns its <button>.
defineProps({
  // Key tokens, e.g. ['Ctrl', 'K'].
  keys: { type: Array, default: () => [] },
})
const emit = defineEmits(['click'])
</script>

<template>
  <button class="keybind" title="Click, then press the new shortcut" @click="emit('click')">
    <span class="keys">
      <template v-for="(k, i) in keys" :key="i">
        <kbd>{{ k }}</kbd><span v-if="i < keys.length - 1" class="plus">+</span>
      </template>
    </span>
  </button>
</template>

<style scoped>
.keybind {
  min-width: 150px;
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 5px 8px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 12px;
}
.keybind:hover { border-color: var(--border-soft); }
.keys { flex: none; display: flex; align-items: center; flex-wrap: wrap; gap: 4px; }
.plus { color: var(--text-faint); font-size: 11px; }
kbd {
  font-family: var(--mono);
  font-size: 11.5px;
  line-height: 1;
  color: var(--text);
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
  border-bottom-width: 2px;
  border-radius: 4px;
  padding: 4px 7px;
  min-width: 12px;
  text-align: center;
}
</style>
