<script setup>
// A labeled form row: a label above a control (the default slot), with an optional
// hint below. Replaces the repeated `<div class="x-field"><label>…</label><control/></div>`
// wrapper. `uppercase` selects the faint small-caps label style some dialogs use.
// Single root, so a call site keeps its class (e.g. flex:1 in a two-column row).
import HintText from './HintText.vue'
defineProps({
  label: { type: String, default: '' },
  hint: { type: String, default: '' },
  uppercase: { type: Boolean, default: false },
})
</script>

<template>
  <div class="form-field">
    <label v-if="label || $slots.label" class="form-field-label" :class="{ upper: uppercase }">
      <slot name="label">{{ label }}</slot>
    </label>
    <slot />
    <HintText v-if="hint">{{ hint }}</HintText>
  </div>
</template>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.form-field-label {
  font-size: 12px;
  color: var(--text-dim);
}
.form-field-label.upper {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
</style>
