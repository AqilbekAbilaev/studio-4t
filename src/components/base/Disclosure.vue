<script setup>
import BaseIcon from './BaseIcon.vue'

// Collapsible section header — a full-width row with a caret that flips open plus
// slotted label content. The cm-sec-head / se-grp-head pattern from the compare and
// search-results panels. Owns its <button> so feature code carries none.
defineProps({
  modelValue: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])
</script>

<template>
  <button class="disclosure" @click="emit('update:modelValue', !modelValue)">
    <BaseIcon :name="modelValue ? 'caretDown' : 'caret'" :size="11" />
    <slot />
  </button>
</template>

<style scoped>
.disclosure {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-panel);
  border: none;
  color: var(--text);
  padding: 8px 10px;
  cursor: pointer;
  font-size: 12.5px;
  text-align: left;
}
.disclosure:hover { background: var(--bg-hover); }
</style>
