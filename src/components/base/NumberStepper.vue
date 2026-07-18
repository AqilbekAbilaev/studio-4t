<script setup>
import BaseIcon from './BaseIcon.vue'

// Compact number field with up/down steppers — the limit/skip inputs in the query
// bar. Encapsulates the little stepper <button>s so feature code carries none.
const props = defineProps({
  modelValue: { type: Number, default: 0 },
  // Lower clamp; the field never goes below this.
  min: { type: Number, default: 0 },
  placeholder: { type: String, default: '' },
})
const emit = defineEmits(['update:modelValue', 'enter'])

function clamp(value) {
  return Math.max(props.min, value)
}
function onInput(event) {
  emit('update:modelValue', clamp(parseInt(event.target.value) || props.min))
}
function step(delta) {
  emit('update:modelValue', clamp((props.modelValue || props.min) + delta))
}
</script>

<template>
  <div class="numbox">
    <input
      :value="modelValue"
      :placeholder="placeholder"
      inputmode="numeric"
      @input="onInput"
      @keydown.enter.prevent="emit('enter')"
    />
    <div class="num-steppers">
      <button tabindex="-1" @click="step(1)">
        <BaseIcon name="caret" :size="9" style="transform: rotate(-90deg)" />
      </button>
      <button tabindex="-1" @click="step(-1)">
        <BaseIcon name="caret" :size="9" style="transform: rotate(90deg)" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.numbox {
  display: flex;
  align-items: stretch;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  width: 72px;
  overflow: hidden;
}
.numbox:focus-within { border-color: var(--accent); }
.numbox input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  padding: 5px 0 5px 9px;
}
.numbox input::placeholder { color: var(--text-faint); }
.num-steppers {
  display: flex;
  flex-direction: column;
  flex: none;
  border-left: 1px solid var(--border-soft);
}
.num-steppers button {
  flex: 1;
  width: 17px;
  display: grid;
  place-items: center;
  background: var(--bg-toolbar);
  border: none;
  color: var(--text-dim);
  padding: 0;
  cursor: pointer;
}
.num-steppers button:first-child { border-bottom: 1px solid var(--border-soft); }
.num-steppers button:hover { background: var(--bg-hover); color: var(--text); }
</style>
