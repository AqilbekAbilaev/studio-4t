<script setup>
const props = defineProps({
  activeTab:         { type: Object, required: true },
  pipelineErrorText: { type: String, default: null },
})
const emit = defineEmits(['run'])
</script>

<template>
  <div class="agg-editor">
    <textarea
      class="agg-input"
      :value="activeTab.pipeline"
      @input="activeTab.pipeline = $event.target.value"
      @keydown.ctrl.enter.prevent="emit('run')"
      @keydown.meta.enter.prevent="emit('run')"
      placeholder='[ { "$match": {} }, { "$limit": 20 } ]'
      spellcheck="false"
      autocorrect="off"
      autocapitalize="off"
    ></textarea>
    <div v-if="pipelineErrorText" class="qparse-error">{{ pipelineErrorText }}</div>
  </div>
</template>

<style scoped>
.agg-editor { padding: 8px 10px; border-bottom: 1px solid var(--border); flex: none; }
.agg-input {
  width: 100%;
  min-height: 96px;
  resize: vertical;
  box-sizing: border-box;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.5;
}
.agg-input:focus { outline: none; border-color: var(--accent); }
.qparse-error { color: var(--danger-text); font-size: 12px; padding: 4px 12px 6px; flex: none; }
</style>
