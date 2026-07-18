<script setup>
import BaseIcon from './BaseIcon.vue'

// Disclosure toggle for the "raw JSON" panels in the admin/stats modals — a caret
// that flips open next to a label. This exact control was hand-copied into five
// modals; owning its <button> here removes that duplication.
defineProps({
  modelValue: { type: Boolean, default: false },
  label: { type: String, default: 'Raw JSON' },
})
const emit = defineEmits(['update:modelValue'])
</script>

<template>
  <button class="raw-toggle" @click="emit('update:modelValue', !modelValue)">
    <BaseIcon :name="modelValue ? 'caretDown' : 'caret'" :size="12" />
    {{ label }}
  </button>
</template>

<style scoped>
.raw-toggle {
  align-self: flex-start;
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 12.5px;
  cursor: pointer;
  padding: 2px 0;
}
.raw-toggle:hover { color: var(--text); }
</style>
