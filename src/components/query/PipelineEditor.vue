<script setup>
import BaseTextarea from '../base/BaseTextarea.vue'
import FieldError from '../base/FieldError.vue'
const props = defineProps({
  activeTab:         { type: Object, required: true },
  pipelineErrorText: { type: String, default: null },
})
const emit = defineEmits(['run'])
</script>

<template>
  <div class="agg-editor">
    <BaseTextarea
      class="agg-input"
      :model-value="activeTab.pipeline"
      @update:model-value="activeTab.pipeline = $event"
      @keydown.ctrl.enter.prevent="emit('run')"
      @keydown.meta.enter.prevent="emit('run')"
      placeholder='[ { "$match": {} }, { "$limit": 20 } ]'
      spellcheck="false"
      autocorrect="off"
      autocapitalize="off"
    ></BaseTextarea>
    <FieldError :text="pipelineErrorText" class="qparse-error" />
  </div>
</template>

<style scoped>
.agg-editor { padding: 8px 10px; border-bottom: 1px solid var(--border); flex: none; }
.qparse-error { padding: 4px 12px 6px; flex: none; }
</style>
