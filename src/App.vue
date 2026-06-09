<script setup>
import { ref } from "vue"
import Connections from "./components/connections.vue";
import Dashboard from "./components/dashboard.vue";

const dividerPosition = ref(30);

function calculateDividerPosition(e) {
  const percentage = (e.pageX / window.innerWidth) * 100
  if (percentage >= 10 && percentage <= 90) {
    dividerPosition.value = percentage.toFixed(2)
  }
}

function handleDragging() {
  document.addEventListener('mousemove', calculateDividerPosition)
}

function endDragging() {
  document.removeEventListener('mousemove', calculateDividerPosition)
}
</script>

<template>
  <main class="main" @mouseup="endDragging">
    <Connections :style="{ width: dividerPosition + '%' }" />
    <div class="divider" @mousedown.prevent="handleDragging" :style="{ left: dividerPosition + '%' }"></div>
    <Dashboard :style="{ width: (100 - dividerPosition) + '%' }" />
  </main>
</template>

<style scoped>
.main {
  background-color: #1e1e1e;
  display: flex;
  align-items: start;
  justify-content: space-between;
  position: relative;
  height: 100%;
}

.divider {
  width: 4px;
  cursor: col-resize;
  position: absolute;
  background-color: transparent;
  left: 30%;
  height: 100%;
}
</style>

<style>
:root {
  color: #0f0f0f;
  background-color: #f6f6f6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
  height: 100%;
}

body {
  height: 100%;
}

#app {
  height: 100%;
}

*,
::before,
::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu,
    Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
  font-size: 14px;
  font-weight: normal;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }
}

.pointer {
  cursor: pointer;
  pointer-events: auto;
  user-select: none;
  -webkit-user-select: none;
  -moz-user-select: none;
}
</style>
