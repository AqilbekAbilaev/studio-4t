<script setup>
import BaseIcon from './BaseIcon.vue'

// A selectable card in a picker grid — icon + label, active/disabled states and an
// optional "soon" badge. The task-type picker in the Tasks modal. Owns its <button>.
defineProps({
  icon: { type: String, default: '' },
  label: { type: String, default: '' },
  active: { type: Boolean, default: false },
  disabled: { type: Boolean, default: false },
  soon: { type: Boolean, default: false },
})
const emit = defineEmits(['click'])
</script>

<template>
  <button class="select-card" :class="{ active: active, disabled: disabled }" :disabled="disabled" @click="emit('click')">
    <BaseIcon v-if="icon" :name="icon" :size="16" />
    <span>{{ label }}</span>
    <span v-if="soon" class="soon">soon</span>
  </button>
</template>

<style scoped>
.select-card {
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 7px;
  padding: 8px 10px;
  font-size: 12px;
  cursor: pointer;
  position: relative;
}
.select-card:hover:not(.disabled) { background: var(--bg-hover); }
.select-card.active {
  border-color: var(--accent);
  color: var(--text);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.select-card.disabled { opacity: .5; cursor: not-allowed; }
.soon {
  margin-left: auto;
  font-size: 9.5px;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--text-faint);
  border: 1px solid var(--border-soft);
  border-radius: 4px;
  padding: 0 4px;
}
</style>
