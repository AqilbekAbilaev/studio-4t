<script setup>
import { ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import Menubar from "./components/menubar/menubar.vue"
import Connections from "./components/connections.vue";
import Dashboard from "./components/dashboard.vue";

const isPopupClosed = ref(false);
const dividerPosition = ref(30);
const isMenubarDisabled = ref(false)

function closeModal() {
  isPopupClosed.value = !isPopupClosed.value;
}

function calculateDividerPosition(e) {
  const percentage = (e.pageX / window.innerWidth) * 100

  if (percentage >= 10 && percentage <= 90) {
    dividerPosition.value = percentage.toFixed(2)
  }
}

function handleDragging(e) {
  document.addEventListener('mousemove', calculateDividerPosition)
}

function endDragging(e) {
  document.removeEventListener('mousemove', calculateDividerPosition)
}

document.addEventListener('keydown', (e) => {
  if (e.ctrlKey && e.key == 'd') {
    isMenubarDisabled.value = false
  }
})

</script>


<template @keyup.alt.enter="isMenubarDisabled = false">
  <Menubar v-if="!isMenubarDisabled" class="navbar" :isPopupClosed="isPopupClosed" @close-modal="closeModal"
    @disable-menubar="isMenubarDisabled = true" />
  <main class="main" :class="isMenubarDisabled ? 'main-full-height' : 'main-with-menubar'" @click="isPopupClosed = false" @mouseup="endDragging">
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
}

.main-full-height {
  height: 100%;
}

.main-with-menubar {
  height: calc(100% - 27px);
}

.divider {
  width: 4px;
  cursor: col-resize;
  resize: horizontal;
  position: absolute;
  background-color: transparent;
  left: 30%;
  height: 100%;
  /* z-index: 999; */
  /* border: 1px solid red; */
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
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
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
  /* standard syntax */
  -webkit-user-select: none;
  /* webkit (safari, chrome) browsers */
  -moz-user-select: none;
  /* mozilla browsers */
  -khtml-user-select: none;
  /* webkit (konqueror) browsers */
  -ms-user-select: none;
}
</style>
