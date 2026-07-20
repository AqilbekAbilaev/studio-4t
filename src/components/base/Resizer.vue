<script setup>
// A drag-to-resize divider. `v-model` is the size it drives (sidebar width or dock
// height); the component tracks the drag on document-level mousemove/mouseup, clamps to
// [min, max], and toggles its own `dragging` class for the accent highlight. `axis` picks
// a vertical bar with horizontal drag ('x', e.g. the sidebar) or a horizontal bar with
// vertical drag ('y', e.g. the operations dock); `invert` is for a bottom-anchored dock,
// where dragging up (a smaller clientY) grows it.
import { ref } from 'vue'

const props = defineProps({
  modelValue: { type: Number, required: true },
  axis: { type: String, default: 'x' },
  min: { type: Number, default: 0 },
  max: { type: Number, default: Infinity },
  invert: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])

const dragging = ref(false)

function onMouseDown(e) {
  e.preventDefault()
  const horizontal = props.axis === 'x'
  const start = horizontal ? e.clientX : e.clientY
  const startVal = props.modelValue
  dragging.value = true
  const onMove = (ev) => {
    const cur = horizontal ? ev.clientX : ev.clientY
    const delta = props.invert ? (start - cur) : (cur - start)
    emit('update:modelValue', Math.max(props.min, Math.min(props.max, startVal + delta)))
  }
  const onUp = () => {
    dragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  document.body.style.cursor = horizontal ? 'col-resize' : 'row-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}
</script>

<template>
  <div
    :class="[axis === 'x' ? 'resizer' : 'resizer-h', { dragging: dragging }]"
    title="Drag to resize"
    @mousedown="onMouseDown"
  >
    <span :class="axis === 'x' ? 'resizer-grip' : 'resizer-grip-h'"></span>
  </div>
</template>

<style scoped>
.resizer {
  width: 3px;
  flex: none;
  cursor: col-resize;
  background: var(--border);
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}
.resizer-grip {
  width: 2px;
  height: 32px;
  background: transparent;
  border-radius: 1px;
  cursor: col-resize;
  transition: background 0.12s;
}
.resizer:hover .resizer-grip,
.resizer.dragging .resizer-grip { background: var(--accent); }

.resizer-h {
  height: 3px;
  flex: none;
  cursor: row-resize;
  background: var(--border);
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}
.resizer-grip-h {
  width: 32px;
  height: 2px;
  background: transparent;
  border-radius: 1px;
  cursor: row-resize;
  transition: background 0.12s;
}
.resizer-h:hover .resizer-grip-h,
.resizer-h.dragging .resizer-grip-h { background: var(--accent); }
</style>
