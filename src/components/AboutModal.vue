<script setup>
import { ref, onMounted } from 'vue'
import { getVersion, getName, getTauriVersion } from '@tauri-apps/api/app'
import BaseIcon from './BaseIcon.vue'

// Help → About. Shows the real application name/version reported by the Tauri
// runtime (never hardcoded), plus the build's Tauri version.
defineEmits(['close'])

const name = ref('Studio-4T')
const version = ref('')
const tauriVersion = ref('')

onMounted(async () => {
  try {
    name.value = await getName()
    version.value = await getVersion()
    tauriVersion.value = await getTauriVersion()
  } catch (e) {
    // Outside the Tauri runtime (e.g. plain vite dev) these are unavailable; leave
    // the fields blank rather than showing a misleading value.
  }
})
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">About</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="ab-body">
        <div class="ab-name">{{ name }}</div>
        <div class="ab-version" v-if="version">Version {{ version }}</div>
        <div class="ab-meta">
          <div v-if="tauriVersion">Built with Tauri {{ tauriVersion }}</div>
          <div>A cross-platform database GUI.</div>
        </div>
      </div>

      <div class="ab-footer">
        <span class="spacer"></span>
        <button class="btn primary" @click="$emit('close')">Close</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, .5);
  display: grid;
  place-items: center;
  z-index: 70;
}
.dialog {
  width: 380px;
  max-width: 92vw;
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.dlg-title {
  height: 36px;
  flex: none;
  background: linear-gradient(var(--dlg-titlebar-1), var(--dlg-titlebar-2));
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 10px;
  position: relative;
}
.dlg-title .t {
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

.ab-body {
  padding: 24px 20px;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ab-name { font-size: 18px; font-weight: 600; color: var(--text); }
.ab-version { font-size: 13px; color: var(--text-dim); }
.ab-meta { margin-top: 10px; font-size: 12px; color: var(--text-faint); line-height: 1.6; }

.ab-footer {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
}
.ab-footer .spacer { flex: 1; }
.btn {
  height: 30px;
  padding: 0 14px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
}
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
</style>
