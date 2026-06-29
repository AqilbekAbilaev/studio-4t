<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  menu: { type: Object, required: true },
})
const emit = defineEmits(['close', 'pick'])

const menuEl = ref(null)
const hoveredItem = ref(null)
const pos = ref({ x: props.menu.x, y: props.menu.y })

const COLOR_TAGS = [
  { name: 'none',   color: 'transparent' },
  { name: 'blue',   color: '#3b82f6' },
  { name: 'green',  color: '#4caf78' },
  { name: 'purple', color: '#b07ddb' },
  { name: 'red',    color: '#e07a6b' },
  { name: 'orange', color: '#e0a35e' },
]

const MENUS = {
  connection: [
    { label: 'Server Info',       sub: 'list', subItems: ['Build Info', 'Host Info', 'Server Status', 'Replica Set Status'] },
    { label: 'Current Operations' },
    { sep: true },
    { label: 'Open IntelliShell', icon: 'shell',  shortcut: '⌘L' },
    { label: 'Search in…',        icon: 'search' },
    { sep: true },
    { label: 'Add Database…' },
    { sep: true },
    { label: 'Copy Name',  shortcut: '⌥⌘C' },
    { label: 'Export URI…' },
    { sep: true },
    { label: 'Import…' },
    { label: 'Export…' },
    { sep: true },
    { label: 'SQL Migration', sub: 'list', subItems: ['Migrate to SQL…', 'Schedule Migration…', 'Migration History'] },
    { sep: true },
    { label: 'Refresh Selected Item', shortcut: '⇧⌘R' },
    { label: 'Refresh All',           shortcut: '⌘R' },
    { label: 'Choose Color', icon: 'brush', sub: 'color' },
    { sep: true },
    { label: 'Disconnect',       shortcut: '⌃⌥D' },
    { label: 'Disconnect Others' },
    { label: 'Disconnect All' },
  ],
  database: [
    { label: 'Open IntelliShell', icon: 'shell',  shortcut: '⌘L' },
    { label: 'Search in…',        icon: 'search' },
    { sep: true },
    { label: 'Add Collection…' },
    { label: 'Add View…' },
    { sep: true },
    { label: 'Copy Name', shortcut: '⌥⌘C' },
    { label: 'Duplicate Database…' },
    { sep: true },
    { label: 'Import…' },
    { label: 'Export…' },
    { sep: true },
    { label: 'Refresh', shortcut: '⌘R' },
    { label: 'Drop Database…', danger: true },
  ],
  collection: [
    { label: 'Open Collection',        icon: 'collection', shortcut: '↵' },
    { label: 'Open IntelliShell',      icon: 'shell' },
    { label: 'Open Aggregation Editor',icon: 'aggregate' },
    { sep: true },
    { label: 'View Schema',    icon: 'schema' },
    { label: 'Indexes…' },
    { label: 'Collection Stats' },
    { sep: true },
    { label: 'Copy Name', shortcut: '⌥⌘C' },
    { label: 'Rename Collection…' },
    { label: 'Duplicate Collection…' },
    { sep: true },
    { label: 'Import…' },
    { label: 'Export…' },
    { sep: true },
    { label: 'Refresh', shortcut: '⌘R' },
    { label: 'Drop Collection…', danger: true },
  ],
  tab: [
    { label: 'Close Tab' },
    { label: 'Close Other Tabs' },
    { label: 'Close Tabs to the Left' },
    { label: 'Close Tabs to the Right' },
    { label: 'Close All Tabs' },
    { sep: true },
    { label: 'Duplicate Tab', icon: 'copy' },
    { label: 'Move Tab to the Front' },
    { label: 'Rename Tab…', icon: 'edit' },
    { sep: true },
    { label: 'Choose Color', icon: 'brush', sub: 'color' },
  ],
}

onMounted(() => {
  if (menuEl.value) {
    const rect = menuEl.value.getBoundingClientRect()
    const clampedX = Math.max(8, Math.min(props.menu.x, window.innerWidth  - rect.width  - 8))
    const clampedY = Math.max(8, Math.min(props.menu.y, window.innerHeight - rect.height - 8))
    pos.value = { x: clampedX, y: clampedY }
  }
  document.addEventListener('keydown', onKeyDown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeyDown)
})

function onKeyDown(e) {
  if (e.key === 'Escape') {
    emit('close')
  }
}

function colorLabel(name) {
  if (name === 'none') return 'No Color'
  return name[0].toUpperCase() + name.slice(1)
}
</script>

<template>
  <!-- transparent backdrop catches clicks outside the menu -->
  <div class="ctx-backdrop" @mousedown="emit('close')" @contextmenu.prevent="emit('close')"></div>

  <div
    ref="menuEl"
    class="ctx-menu"
    :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
    @contextmenu.prevent
  >
    <template v-for="(item, i) in MENUS[menu.type]" :key="i">
      <div v-if="item.sep" class="ctx-sep"></div>
      <div
        v-else
        class="ctx-item"
        :class="{ danger: item.danger }"
        @mouseenter="hoveredItem = item.label"
        @click="item.sub ? undefined : emit('pick', item.label)"
      >
        <span class="ctx-ic">
          <BaseIcon v-if="item.icon" :name="item.icon" :size="15" />
        </span>
        <span class="ctx-label">{{ item.label }}</span>
        <span v-if="item.shortcut" class="ctx-sc">{{ item.shortcut }}</span>
        <span v-if="item.sub" class="ctx-caret">
          <BaseIcon name="caret" :size="12" />
        </span>

        <!-- text list submenu (Server Info, SQL Migration) -->
        <div
          v-if="item.sub === 'list' && hoveredItem === item.label"
          class="ctx-sub"
        >
          <div
            v-for="sub in item.subItems"
            :key="sub"
            class="ctx-item"
            @click.stop="emit('pick', sub)"
          >
            <span class="ctx-ic"></span>
            <span class="ctx-label">{{ sub }}</span>
          </div>
        </div>

        <!-- color swatch submenu -->
        <div
          v-if="item.sub === 'color' && hoveredItem === item.label"
          class="ctx-sub"
        >
          <div
            v-for="tag in COLOR_TAGS"
            :key="tag.name"
            class="ctx-color-item"
            @click.stop="emit('pick', 'Choose Color:' + tag.name)"
          >
            <span
              class="ctx-color-sw"
              :style="{
                background: tag.color,
                border: tag.name === 'none' ? '1px solid var(--border-soft)' : 'none',
              }"
            ></span>
            <span>{{ colorLabel(tag.name) }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* position: fixed instead of absolute — our component mounts at root, not inside .window */
.ctx-backdrop { position: fixed; inset: 0; z-index: 90; }
.ctx-menu { position: fixed; z-index: 91; min-width: 248px; background: #2b2d31; border: 1px solid #16171a; border-radius: 8px; box-shadow: 0 18px 48px rgba(0,0,0,.6); padding: 5px; }
.ctx-item { position: relative; display: flex; align-items: center; gap: 10px; padding: 6px 12px 6px 10px; border-radius: 5px; font-size: 13px; color: var(--text); white-space: nowrap; cursor: default; }
.ctx-item:hover { background: var(--accent); color: #fff; }
.ctx-item:hover .ctx-ic, .ctx-item:hover .ctx-sc, .ctx-item:hover .ctx-caret { color: rgba(255,255,255,.85); }
.ctx-item.danger { color: #e87b6b; }
.ctx-item.danger:hover { background: #c0392b; color: #fff; }
.ctx-ic { width: 18px; flex: none; display: grid; place-items: center; color: var(--text-dim); }
.ctx-label { flex: 1; }
.ctx-sc { color: var(--text-faint); font-size: 12px; margin-left: 28px; letter-spacing: 1px; }
.ctx-caret { color: var(--text-faint); margin-left: 8px; }
.ctx-sep { height: 1px; background: #3a3c41; margin: 5px 8px; }
.ctx-sub { position: absolute; left: 100%; top: -5px; margin-left: 2px; min-width: 200px; background: #2b2d31; border: 1px solid #16171a; border-radius: 8px; box-shadow: 0 18px 48px rgba(0,0,0,.6); padding: 5px; }
.ctx-color-item { display: flex; align-items: center; gap: 10px; padding: 6px 12px; border-radius: 5px; font-size: 13px; color: var(--text); }
.ctx-color-item:hover { background: var(--accent); color: #fff; }
.ctx-color-sw { width: 14px; height: 14px; border-radius: 4px; flex: none; }
</style>
