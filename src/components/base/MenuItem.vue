<script setup>
// A dropdown menu row — full-width, hover-highlighted, with arbitrary slotted
// content (icon + label, or code + hint). The default-item / datetag-item rows from
// the query-bar menus. Owns its <button> so feature code carries none.
const emit = defineEmits(['click'])
</script>

<template>
  <button class="menu-item" @click="emit('click')"><slot /></button>
</template>

<style scoped>
.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  border-radius: 5px;
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 12.5px;
  cursor: pointer;
  text-align: left;
}
.menu-item:hover { background: var(--bg-hover); color: var(--text); }
</style>
