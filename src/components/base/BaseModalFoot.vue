<script setup>
// Modal footer bar: flex row with border-top, left slot for optional controls
// and a default slot for action buttons (typically Cancel + Save/Delete).
// The spacer between left and default slots is built in.
</script>

<template>
  <div class="base-modal-foot">
    <div v-if="$slots.left" class="foot-left">
      <slot name="left" />
    </div>
    <span v-if="$slots.left" class="spacer"></span>
    <div class="foot-right">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.base-modal-foot {
  display: flex;
  align-items: center;
  height: 48px;
  flex: none;
  border-top: 1px solid var(--border);
  padding: 0 16px;
  gap: 8px;
}
.foot-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.spacer { flex: 1; }
.foot-right {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
</style>
