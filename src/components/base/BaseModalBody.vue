<script setup>
defineProps({
  scrollable: { type: Boolean, default: true },
})
</script>

<template>
  <div class="base-modal-body" :class="{ scrollable: scrollable }">
    <slot />
  </div>
</template>

<style scoped>
.base-modal-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
}
.base-modal-body.scrollable {
  overflow: auto;
  max-height: 74vh;
}
</style>
