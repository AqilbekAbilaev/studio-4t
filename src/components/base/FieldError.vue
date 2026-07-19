<script setup>
// Inline validation / operation error text (danger-colored, 12px). Self-hides when
// `text` is empty, so it replaces the `<div v-if="err" class="x-error">{{ err }}</div>`
// pattern outright. `spaced` adds the top gap the in-dialog errors used. Single root,
// so a call site that needs extra layout (padding in an editor chrome) keeps its class.
defineProps({
  text: { type: String, default: '' },
  spaced: { type: Boolean, default: false },
})
</script>

<template>
  <div v-if="text" class="field-error" :class="{ spaced: spaced }">{{ text }}</div>
</template>

<style scoped>
.field-error {
  font-size: 12px;
  color: var(--danger-text);
}
.field-error.spaced { margin-top: 6px; }
</style>
