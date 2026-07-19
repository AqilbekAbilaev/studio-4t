<script setup>
// Underline tab strip — a row of tabs with an accent underline under the active one.
// The recurring rtab/uw-tab pattern from the result views and query modals. Owns the
// tab <button>s so feature code carries none.
defineProps({
  modelValue: { type: [String, Number], default: '' },
  // [{ value, label }]
  options: { type: Array, default: () => [] },
  // Disable the whole strip (e.g. while an alternate editor is active).
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])
</script>

<template>
  <div class="tabstrip">
    <button
      v-for="opt in options"
      :key="opt.value"
      class="tab"
      :class="{ active: modelValue === opt.value }"
      :disabled="disabled"
      @click="emit('update:modelValue', opt.value)"
    >{{ opt.label }}</button>
  </div>
</template>

<style scoped>
.tabstrip { display: flex; }
.tab {
  padding: 7px 16px;
  font-size: 12.5px;
  color: var(--text-dim);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
}
.tab.active { color: var(--text); border-bottom-color: var(--accent); }
.tab:disabled { color: var(--text-faint); cursor: default; }
</style>
