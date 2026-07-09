<script setup>
import { ref } from 'vue'
import BaseIcon from './BaseIcon.vue'

// Hosts two workspace panes side by side (vertical) or stacked (horizontal) with a
// draggable divider. The panes themselves are passed in via the #a / #b slots; this
// component only owns the layout, the divider drag, and the "unsplit" affordance.
const props = defineProps({
  orientation: { type: String, default: 'vertical' }, // 'vertical' | 'horizontal'
})
const emit = defineEmits(['unsplit'])

// First pane's share of the container (0..1). The second pane takes the rest.
const ratio = ref(0.5)
const dragging = ref(false)
const rootRef = ref(null)

function startDrag(e) {
  e.preventDefault()
  dragging.value = true
  const root = rootRef.value
  const onMove = (ev) => {
    const rect = root.getBoundingClientRect()
    const frac = props.orientation === 'horizontal'
      ? (ev.clientY - rect.top) / rect.height
      : (ev.clientX - rect.left) / rect.width
    ratio.value = Math.max(0.15, Math.min(0.85, frac))
  }
  const onUp = () => {
    dragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  document.body.style.cursor = props.orientation === 'horizontal' ? 'row-resize' : 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}
</script>

<template>
  <div ref="rootRef" class="split" :class="orientation">
    <div class="split-pane" :style="{ flex: `0 0 ${ratio * 100}%` }">
      <slot name="a" />
    </div>
    <div class="split-gutter" :class="{ dragging }" @mousedown="startDrag">
      <span class="gutter-grip"></span>
      <button class="gutter-close" title="Close split" @mousedown.stop @click="emit('unsplit')">
        <BaseIcon name="close" :size="11" />
      </button>
    </div>
    <div class="split-pane" style="flex: 1 1 0">
      <slot name="b" />
    </div>
  </div>
</template>

<style scoped>
.split { flex: 1; display: flex; min-width: 0; min-height: 0; }
.split.vertical { flex-direction: row; }
.split.horizontal { flex-direction: column; }

/* Each pane clips its own overflow so the two workspaces scroll independently. */
.split-pane { display: flex; min-width: 0; min-height: 0; overflow: hidden; }

.split-gutter {
  flex: none;
  position: relative;
  background: var(--border);
  display: flex;
  align-items: center;
  justify-content: center;
}
.split.vertical .split-gutter { width: 3px; cursor: col-resize; }
.split.horizontal .split-gutter { height: 3px; cursor: row-resize; }

.gutter-grip { background: transparent; border-radius: 1px; transition: background 0.12s; }
.split.vertical .gutter-grip { width: 2px; height: 32px; }
.split.horizontal .gutter-grip { height: 2px; width: 32px; }
.split-gutter:hover .gutter-grip,
.split-gutter.dragging .gutter-grip { background: var(--accent); }

/* Small close-split button, centered on the gutter; revealed on hover. */
.gutter-close {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: 1px solid var(--border-soft);
  border-radius: 50%;
  background: var(--bg-panel);
  color: var(--text-dim);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.12s, color 0.12s, border-color 0.12s;
}
.split-gutter:hover .gutter-close { opacity: 1; }
.gutter-close:hover { color: var(--accent); border-color: var(--accent); }
</style>
