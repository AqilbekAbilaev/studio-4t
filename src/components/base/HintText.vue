<script setup>
// Small secondary help text under a field / control. Default is the faintest tone;
// `dim` is the slightly brighter one used for explanatory notes. Content via slot
// (may include <code>). Single root, so a call site that needs a specific margin
// keeps its class. Not for inline status labels or bordered callout boxes.
defineProps({
  dim: { type: Boolean, default: false },
})
</script>

<template>
  <div class="hint-text" :class="{ dim: dim }"><slot /></div>
</template>

<style scoped>
.hint-text {
  font-size: 12px;
  line-height: 1.45;
  color: var(--text-faint);
}
.hint-text.dim { color: var(--text-dim); }
</style>
