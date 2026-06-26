<script setup>
import BaseIcon from './BaseIcon.vue'

const cards = [
  ['connect',   'Connect to a database', 'Open the Connection Manager and pick a server.'],
  ['shell',     'Open IntelliShell',     'Autocompleting query console with inline results.'],
  ['aggregate', 'Build an aggregation',  'Visual pipeline editor with live stage previews.'],
  ['import',    'Import / Export',       'Move data between collections, files and engines.'],
]
</script>

<template>
  <div class="quickstart">
    <h1>Welcome to Studio-4T</h1>
    <p>The cross-database workspace. MongoDB today — PostgreSQL, MySQL and more on the roadmap.</p>
    <div class="qs-grid">
      <div v-for="[ic, title, desc] in cards" :key="title" class="qs-card">
        <BaseIcon :name="ic" :size="24" style="color:var(--accent);flex-shrink:0" />
        <div>
          <div class="qs-card-title">{{ title }}</div>
          <div class="qs-card-desc">{{ desc }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.quickstart { flex: 1; overflow: auto; padding: 48px 56px; }
.quickstart h1 { font-size: 24px; font-weight: 600; margin-bottom: 6px; }
.quickstart p  { color: var(--text-dim); font-size: 13.5px; margin-bottom: 28px; }
.qs-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; max-width: 700px; }
.qs-card {
  background: var(--bg-panel);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  padding: 16px;
  display: flex;
  gap: 13px;
}
.qs-card-title { font-size: 13.5px; font-weight: 600; margin-bottom: 4px; }
.qs-card-desc  { font-size: 12px; color: var(--text-dim); }
</style>
