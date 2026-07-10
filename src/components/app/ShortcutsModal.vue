<script setup>
import { computed } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'

// A read-only reference of the keyboard shortcuts the app actually handles
// (opened from Help → Keyboard Shortcuts). Keep this list in sync with the real
// key handlers — don't list aspirational bindings.
defineEmits(['close'])

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || '')
const mod = isMac ? '⌘' : 'Ctrl'

const GROUPS = computed(() => [
  {
    title: 'Query',
    items: [
      { keys: [`${mod}`, 'Enter'], desc: 'Run the current query' },
      { keys: ['Enter'], desc: 'Run from the filter / sort / projection field' },
    ],
  },
  {
    title: 'Results grid',
    items: [
      { keys: ['↑', '↓', '←', '→'], desc: 'Move the cell selection' },
      { keys: [`${mod}`, 'C'], desc: 'Copy the selected cell value' },
      { keys: [`${mod}`, 'J'], desc: 'Edit the selected document in a window' },
      { keys: ['Enter'], desc: 'Commit an inline cell edit' },
      { keys: ['Esc'], desc: 'Cancel an edit / clear the selection' },
    ],
  },
  {
    title: 'IntelliShell',
    items: [
      { keys: [`${mod}`, 'Enter'], desc: 'Run the shell command' },
      { keys: ['Enter'], desc: 'Insert a new line' },
    ],
  },
  {
    title: 'Text fields',
    items: [
      { keys: [`${mod}`, 'Z'], desc: 'Undo' },
      { keys: [`${mod}`, 'Shift', 'Z'], desc: 'Redo' },
      { keys: [`${mod}`, 'Y'], desc: 'Redo (alternate)' },
    ],
  },
  {
    title: 'General',
    items: [
      { keys: ['Double-click'], desc: 'Open a collection from the tree' },
      { keys: ['Esc'], desc: 'Close a menu or dialog' },
    ],
  },
])
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">Keyboard Shortcuts</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="sc-body">
        <section v-for="group in GROUPS" :key="group.title" class="sc-group">
          <h3 class="sc-group-title">{{ group.title }}</h3>
          <div v-for="item in group.items" :key="item.desc" class="sc-row">
            <span class="sc-keys">
              <template v-for="(k, i) in item.keys" :key="i">
                <kbd>{{ k }}</kbd><span v-if="i < item.keys.length - 1" class="sc-plus">+</span>
              </template>
            </span>
            <span class="sc-desc">{{ item.desc }}</span>
          </div>
        </section>
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
  width: 540px;
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

.sc-body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-height: 70vh;
  overflow-y: auto;
}
.sc-group-title {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--text-faint);
}
.sc-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 4px 0;
}
.sc-keys {
  flex: none;
  width: 170px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}
.sc-plus { color: var(--text-faint); font-size: 11px; }
.sc-desc { font-size: 13px; color: var(--text); }

kbd {
  font-family: var(--mono);
  font-size: 11.5px;
  line-height: 1;
  color: var(--text);
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-bottom-width: 2px;
  border-radius: 4px;
  padding: 4px 7px;
  min-width: 12px;
  text-align: center;
}
</style>
