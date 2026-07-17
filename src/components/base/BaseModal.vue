<script setup>
import { computed } from 'vue'
import BaseIcon from './BaseIcon.vue'

// Shared modal shell: a fixed backdrop + a centered dialog box with a titled bar
// (centered title, single close ✕ on the right — no traffic lights, per the design
// handoff) and a default slot for the body/footer. Backdrop click and the ✕ both emit
// `close`. Sizing is driven by props so callers range from a 400px confirm box to a
// larger scrollable viewer.
const props = defineProps({
  title:     { type: String, default: '' },
  width:     { type: String, default: '400px' },
  maxWidth:  { type: String, default: '' },
  height:    { type: String, default: '' },
  maxHeight: { type: String, default: '' },
})

defineEmits(['close'])

const dialogStyle = computed(() => ({
  width:     props.width,
  maxWidth:  props.maxWidth || undefined,
  height:    props.height || undefined,
  maxHeight: props.maxHeight || undefined,
}))
</script>

<template>
  <div class="bm-overlay" @mousedown.self="$emit('close')">
    <div class="bm-dialog" :style="dialogStyle">
      <div class="bm-title">
        <div class="t"><slot name="title">{{ title }}</slot></div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>
      <slot />
    </div>
  </div>
</template>

<style scoped>
.bm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 60;
}
.bm-dialog {
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.bm-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
  position: relative;
}
.bm-title .t {
  position: absolute;
  left: 0; right: 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 500;
  pointer-events: none;
}
.close-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  border-radius: 4px;
  z-index: 1;
}
.close-btn:hover { background: var(--bg-hover); color: var(--text); }
</style>
