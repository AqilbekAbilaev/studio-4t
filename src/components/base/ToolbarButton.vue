<script setup>
import BaseIcon from './BaseIcon.vue'

// A global-toolbar button: a stacked icon + label, with an optional badge dot and an
// optional drop caret, plus a dimmed "off" state. Used by the app toolbar and the
// Connection Manager toolbar. Owns its <button> so those toolbars carry none.
defineProps({
  icon: { type: String, required: true },
  label: { type: String, default: '' },
  // A colour string paints a small badge dot on the icon; falsy shows none.
  badge: { type: [String, Boolean], default: '' },
  drop: { type: Boolean, default: false },
  // Dimmed / inactive (no action available or nothing selected).
  off: { type: Boolean, default: false },
})
const emit = defineEmits(['click'])
</script>

<template>
  <button class="tbtn" :class="{ 'tbtn-off': off }" @click="emit('click')">
    <span class="ic" :class="{ 'ic-badge': badge }">
      <BaseIcon :name="icon" :size="22" />
      <i v-if="badge" class="dotmark" :style="{ background: badge }"></i>
    </span>
    <span class="lbl">{{ label }}</span>
    <BaseIcon v-if="drop" name="caretDown" :size="11" class="drop" />
  </button>
</template>

<style scoped>
.tbtn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 5px 9px;
  border: none;
  background: none;
  border-radius: 6px;
  color: var(--text);
  min-width: 54px;
  position: relative;
  cursor: pointer;
}
.tbtn:hover { background: var(--bg-hover); }
.tbtn .ic { color: var(--text-dim); position: relative; }
.tbtn:hover .ic { color: var(--text); }
.tbtn .lbl { font-size: 10.5px; color: var(--text-dim); white-space: nowrap; }
.tbtn .drop { position: absolute; right: 2px; top: 3px; color: var(--text-faint); }
.tbtn-off { opacity: .4; cursor: default; }
.tbtn-off:hover { background: none; }
.ic-badge { position: relative; }
.dotmark {
  position: absolute;
  right: -1px;
  bottom: 1px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1.5px solid var(--bg-toolbar);
}
</style>
