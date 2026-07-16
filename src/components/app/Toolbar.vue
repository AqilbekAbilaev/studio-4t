<script setup>
// The global toolbar strip. Presentational only: renders the TOOLS buttons and emits
// `tool` with the clicked action id; App.vue routes that into its handleTool dispatcher.
// `hidden` is driven by the View → Hide Global Toolbar toggle.
import BaseIcon from '../base/BaseIcon.vue'
import { TOOLS } from '../../constants/tools'

defineProps({
  hidden: { type: Boolean, default: false },
})
defineEmits(['tool'])
</script>

<template>
  <div class="toolbar" v-show="!hidden">
    <template v-for="(t, i) in TOOLS" :key="i">
      <div v-if="t.sep" class="tb-sep"></div>
      <button v-else class="tbtn" :title="t.label" @click="$emit('tool', t.name)">
        <span class="ic" :class="{ 'ic-badge': t.badge }">
          <BaseIcon :name="t.name" :size="22" />
          <i v-if="t.badge" class="dotmark" :style="{ background: t.badge }"></i>
        </span>
        <span class="lbl">{{ t.label }}</span>
        <BaseIcon v-if="t.drop" name="caretDown" :size="11" class="drop" />
      </button>
    </template>
  </div>
</template>

<style scoped>
.toolbar {
  flex: none;
  background: var(--bg-toolbar);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: stretch;
  padding: 6px 8px;
  gap: 2px;
}
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
}
.tbtn:hover { background: var(--bg-hover); }
.tbtn .ic { color: var(--text-dim); position: relative; }
.tbtn:hover .ic { color: var(--text); }
.tbtn .lbl { font-size: 10.5px; color: var(--text-dim); white-space: nowrap; }
.tbtn .drop { position: absolute; right: 2px; top: 3px; color: var(--text-faint); }
.tb-sep { width: 1px; flex: none; background: var(--border-soft); margin: 6px 4px; align-self: stretch; }
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
