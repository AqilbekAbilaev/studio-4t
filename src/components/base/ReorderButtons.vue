<script setup>
// Up/down reorder control for list rows (the iew-move / rs-icon arrows). Owns its
// two <button>s so feature code carries none; disable each end via props.
defineProps({
  upDisabled: { type: Boolean, default: false },
  downDisabled: { type: Boolean, default: false },
})
const emit = defineEmits(['up', 'down'])
</script>

<template>
  <span class="reorder">
    <button class="rb" title="Move up" :disabled="upDisabled" @click="emit('up')">↑</button>
    <button class="rb" title="Move down" :disabled="downDisabled" @click="emit('down')">↓</button>
  </span>
</template>

<style scoped>
.reorder { display: inline-flex; gap: 2px; }
.rb {
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 3px 5px;
  border-radius: 4px;
  font-size: 12px;
  line-height: 1;
}
.rb:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
.rb:disabled { opacity: .35; cursor: default; }
</style>
