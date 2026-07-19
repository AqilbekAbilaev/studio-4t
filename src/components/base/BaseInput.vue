<script setup>
// The app's canonical text field: owns the input chrome (bg-input, soft border,
// focus→accent, disabled dimming) plus the v-model / Enter wiring every call site
// was re-writing by hand. Single root, so `placeholder`, `class`, `disabled`,
// `readonly`, `autofocus` etc. fall through; a call site that needs a distinct
// look keeps its own class (it wins over the canonical rules here).
const props = defineProps({
  modelValue: { type: [String, Number], default: '' },
  // text/password/number/time/… — kept explicit so number inputs can emit a Number.
  type: { type: String, default: 'text' },
})
const emit = defineEmits(['update:modelValue', 'enter', 'blur', 'focus'])

function onInput(e) {
  const raw = e.target.value
  if (props.type === 'number') {
    emit('update:modelValue', raw === '' ? '' : Number(raw))
  } else {
    emit('update:modelValue', raw)
  }
}
</script>

<template>
  <input
    class="base-input"
    :type="type"
    :value="modelValue"
    @input="onInput"
    @keydown.enter="emit('enter', $event)"
    @blur="emit('blur', $event)"
    @focus="emit('focus', $event)"
  />
</template>

<style scoped>
.base-input {
  width: 100%;
  box-sizing: border-box;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 8px 11px;
  color: var(--text);
  font-size: 13px;
  outline: none;
}
.base-input:focus { border-color: var(--accent); }
.base-input:disabled { opacity: .5; cursor: not-allowed; }
.base-input::placeholder { color: var(--text-faint); }
</style>
