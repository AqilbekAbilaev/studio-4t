<script setup>
import { ref, computed } from 'vue'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  tabs:        { type: Array,  required: true },
  activeTabId: { type: String, required: true },
})
const emit = defineEmits(['activate-tab', 'close-tab', 'run-query'])

const activeTab = computed(() => props.tabs.find(t => t.id === props.activeTabId))

// per-tab local state for result sub-tab and view mode
const rtab     = ref('Result')
const viewMode = ref('table')
const viewMenu = ref(false)

// ── query helpers ──────────────────────────────────────────
function toStrictJson(raw) {
  const s = (raw || '').trim()
  if (!s || s === '{}') return '{}'
  return s.replace(/([{,]\s*)([a-zA-Z_$][a-zA-Z0-9_$.]*)\s*:/g, '$1"$2":')
}

function runQuery() {
  const tab = activeTab.value
  if (!tab || tab.kind !== 'collection') return
  emit('run-query', tab.id, {
    filter:     toStrictJson(tab.filter),
    projection: toStrictJson(tab.projection),
    sort:       toStrictJson(tab.sort),
    skip:       tab.skip || 0,
    limit:      tab.limit || 50,
  })
}

// ── table view helpers ─────────────────────────────────────
function guessType(key, val) {
  if (key === '_id' || (val && typeof val === 'object' && '$oid' in val)) return 'id'
  if (val && typeof val === 'object' && '$date' in val) return 'date'
  if (typeof val === 'number') return 'num'
  if (typeof val === 'boolean') return 'bool'
  if (val === null || val === undefined) return 'null'
  if (Array.isArray(val) || (typeof val === 'object')) return 'obj'
  return 'str'
}

const TYPE_ICON = { id: 'typeId', str: 'typeStr', num: 'typeNum', date: 'typeDate', bool: 'typeBool', null: 'typeNull', obj: 'typeObj' }
const TYPE_CLASS = { id: 'cell-oid', str: 'cell-str', num: 'cell-num', date: '', bool: 'cell-num', null: 'cell-faint', obj: 'cell-faint' }

function formatCell(key, val) {
  if (val === null || val === undefined) return ''
  if (typeof val === 'string') return val
  if (typeof val === 'number' || typeof val === 'boolean') return String(val)
  if (Array.isArray(val)) return `Array(${val.length})`
  if (typeof val === 'object') {
    if ('$oid' in val) return val.$oid
    if ('$date' in val) {
      const d = val.$date
      if (typeof d === 'string') return d
      if (typeof d === 'object' && '$numberLong' in d) return new Date(parseInt(d.$numberLong)).toISOString()
    }
    if ('$numberLong' in val) return val.$numberLong
    if ('$numberDecimal' in val) return val.$numberDecimal
    return '{…}'
  }
  return JSON.stringify(val)
}

function columns(results) {
  if (!results?.length) return []
  const seen = new Set()
  for (const doc of results) for (const k of Object.keys(doc)) seen.add(k)
  const rest = [...seen].filter(k => k !== '_id').sort()
  return seen.has('_id') ? ['_id', ...rest] : rest
}
</script>

<template>
  <div class="work">
    <!-- Tabs -->
    <div class="tabs">
      <button
        v-for="t in tabs"
        :key="t.id"
        class="tab"
        :class="{ active: t.id === activeTabId }"
        @click="emit('activate-tab', t.id)"
      >
        <span>{{ t.title }}</span>
        <span class="x" @click.stop="emit('close-tab', t.id)">
          <BaseIcon name="close" :size="12" />
        </span>
      </button>
    </div>

    <!-- Quickstart pane -->
    <template v-if="!activeTab || activeTab.kind === 'quickstart'">
      <div class="quickstart">
        <h1>Welcome to Studio-4T</h1>
        <p>The cross-database workspace. MongoDB today — PostgreSQL, MySQL and more on the roadmap.</p>
        <div class="qs-grid">
          <div v-for="[ic, title, desc] in [
            ['connect',   'Connect to a database',  'Open the Connection Manager and pick a server.'],
            ['shell',     'Open IntelliShell',       'Autocompleting query console with inline results.'],
            ['aggregate', 'Build an aggregation',   'Visual pipeline editor with live stage previews.'],
            ['import',    'Import / Export',         'Move data between collections, files and engines.'],
          ]" :key="title" class="qs-card">
            <BaseIcon :name="ic" :size="24" style="color:var(--accent);flex-shrink:0" />
            <div>
              <div class="qs-card-title">{{ title }}</div>
              <div class="qs-card-desc">{{ desc }}</div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Collection workspace -->
    <template v-else-if="activeTab.kind === 'collection'">
      <!-- Breadcrumb -->
      <div class="crumbs">
        <BaseIcon name="connect" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.connectionName }}</span>
        <BaseIcon name="caret"  :size="11" class="sep" />
        <BaseIcon name="dbSmall" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.dbName }}</span>
        <BaseIcon name="caret"  :size="11" class="sep" />
        <BaseIcon name="collSmall" :size="15" class="c-ic" />
        <span class="crumb">{{ activeTab.collectionName }}</span>
      </div>

      <!-- Query bar -->
      <div class="qbar">
        <button class="qbtn run" @click="runQuery" :disabled="activeTab.isRunning">
          <BaseIcon name="run" :size="16" class="ic" />
          {{ activeTab.isRunning ? 'Running…' : 'Run' }}
          <BaseIcon name="caretDown" :size="11" class="drop" />
        </button>
        <button class="qbtn" disabled><BaseIcon name="load"    :size="16" class="ic" /> Load query   <BaseIcon name="caretDown" :size="11" class="drop" /></button>
        <button class="qbtn" disabled><BaseIcon name="save"    :size="16" class="ic" /> Save query   <BaseIcon name="caretDown" :size="11" class="drop" /></button>
        <button class="qbtn" disabled><BaseIcon name="history" :size="16" class="ic" /> Query history</button>
        <button class="qbtn" disabled><BaseIcon name="copy"    :size="16" class="ic" /> Copy</button>
        <span class="qbar-spacer"></span>
        <button class="vqb-toggle" disabled>
          <BaseIcon name="aggregate" :size="15" /> Visual Query Builder
        </button>
      </div>

      <!-- Query fields grid -->
      <div class="qfields">
        <span class="qlabel">Query</span>
        <div class="qinput">
          <input
            class="qval"
            v-model="activeTab.filter"
            placeholder="{}"
            @keydown.enter.ctrl="runQuery"
            @keydown.enter.meta="runQuery"
          />
          <span class="qicons">
            <BaseIcon name="brush" :size="15" @click="activeTab.filter = ''" style="cursor:pointer" />
          </span>
        </div>
        <span class="qlabel">Sort</span>
        <div class="qinput">
          <input class="qval" v-model="activeTab.sort" placeholder="{}" />
        </div>
        <span></span>

        <span class="qlabel">Projection</span>
        <div class="qinput">
          <input class="qval" v-model="activeTab.projection" placeholder="{}" />
        </div>
        <span class="qlabel">Skip</span>
        <div class="qinput">
          <input class="qval qval-short" v-model.number="activeTab.skip" type="number" min="0" placeholder="0" />
        </div>
        <span></span>

        <span class="qlabel"></span>
        <span></span>
        <span class="qlabel">Limit</span>
        <div class="qinput">
          <input class="qval qval-short" v-model.number="activeTab.limit" type="number" min="1" max="1000" placeholder="50" />
        </div>
        <span></span>
      </div>

      <!-- Results -->
      <div class="results">
        <!-- Result sub-tabs -->
        <div class="rtabs">
          <button
            v-for="t in ['Result', 'Query Code', 'Explain']"
            :key="t"
            class="rtab"
            :class="{ active: rtab === t }"
            @click="rtab = t"
          >{{ t }}</button>
        </div>

        <!-- Result toolbar -->
        <div class="rtoolbar" v-if="rtab === 'Result'">
          <button class="icon-btn" @click="runQuery" :disabled="activeTab.isRunning">
            <BaseIcon name="refresh" :size="16" />
          </button>
          <button class="icon-btn" disabled><BaseIcon name="first" :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="prev"  :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="next"  :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="last"  :size="16" /></button>
          <span class="page-size">{{ activeTab.limit || 50 }} <BaseIcon name="caretDown" :size="12" /></span>
          <span class="docs-range">
            Documents {{ activeTab.results?.length ? `1 to ${activeTab.results.length}` : '-- to --' }}
          </span>
          <button class="icon-btn" disabled><BaseIcon name="lock"  :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="plus"  :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="copy"  :size="16" /></button>
          <button class="icon-btn" disabled><BaseIcon name="edit"  :size="16" /></button>
          <span class="rtoolbar-spacer"></span>

          <!-- View mode selector -->
          <div class="view-select-wrap">
            <span class="view-select" @click="viewMenu = !viewMenu">
              {{ { table: 'Table View', json: 'JSON View', tree: 'Tree View' }[viewMode] }}
              <BaseIcon name="caretDown" :size="12" />
            </span>
            <div v-if="viewMenu" class="view-menu">
              <div
                v-for="[k, label] in [['table','Table View'],['json','JSON View'],['tree','Tree View']]"
                :key="k"
                class="view-menu-item"
                :class="{ on: viewMode === k }"
                @click="viewMode = k; viewMenu = false"
              >
                <BaseIcon v-if="viewMode === k" name="check" :size="13" />
                <span>{{ label }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Error state -->
        <div v-if="activeTab.runError" class="run-error">{{ activeTab.runError }}</div>

        <!-- Table view -->
        <div v-else-if="rtab === 'Result' && viewMode === 'table'" class="grid-wrap">
          <div class="fieldpath">
            <span>{{ activeTab.collectionName }}</span>
          </div>
          <template v-if="!activeTab.hasRun || activeTab.isRunning">
            <table class="grid">
              <thead><tr><th style="min-width:320px">{Document id}</th></tr></thead>
            </table>
            <div class="empty-rows">
              <div v-for="i in 22" :key="i" class="erow"></div>
            </div>
          </template>
          <template v-else-if="activeTab.results?.length === 0">
            <table class="grid">
              <thead><tr><th style="min-width:320px">{Document id}</th></tr></thead>
            </table>
            <div class="empty-rows">
              <div v-for="i in 22" :key="i" class="erow"></div>
            </div>
          </template>
          <template v-else>
            <table class="grid">
              <thead>
                <tr>
                  <th v-for="col in columns(activeTab.results)" :key="col">
                    {{ col === '_id' ? '{Document id}' : col }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, i) in activeTab.results"
                  :key="i"
                  :class="{ selrow: activeTab.selectedRow === i }"
                  @click="activeTab.selectedRow = i"
                >
                  <td v-for="col in columns(activeTab.results)" :key="col">
                    <span class="tcell" :class="'t-' + guessType(col, row[col])">
                      <span class="ticon">
                        <BaseIcon :name="TYPE_ICON[guessType(col, row[col])] || 'typeStr'" :size="14" />
                      </span>
                      <span class="tval" :class="TYPE_CLASS[guessType(col, row[col])]">
                        {{ formatCell(col, row[col]) }}
                      </span>
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </template>
        </div>

        <!-- JSON view -->
        <div v-else-if="rtab === 'Result' && viewMode === 'json'" class="json-view">
          <div v-if="!activeTab.results?.length" style="padding:32px;color:var(--text-faint);font-size:12px">No documents</div>
          <div v-else class="json-doc" v-for="(doc, i) in activeTab.results" :key="i">{{ JSON.stringify(doc, null, 2) }}</div>
        </div>

        <!-- Other sub-tabs placeholder -->
        <div v-else class="empty-rows" style="padding:32px;color:var(--text-faint);font-size:12px;display:flex;align-items:center;justify-content:center">
          {{ rtab }} — coming soon
        </div>

        <!-- Footer -->
        <div class="rfooter">
          <span>{{ activeTab.selectedRow >= 0 ? '1 document selected' : '0 documents selected' }}</span>
          <span class="spacer"></span>
          <span class="fitem"><BaseIcon name="count" :size="14" /> Count Documents</span>
          <span class="fitem" v-if="activeTab.elapsedMs != null">
            <BaseIcon name="clock" :size="14" />
            {{ (activeTab.elapsedMs / 1000).toFixed(3) }}s
          </span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.work { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--bg-window); }

/* Tabs */
.tabs {
  display: flex;
  align-items: stretch;
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
  height: 36px;
  flex: none;
  padding-left: 6px;
  overflow-x: auto;
}
.tab {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  font-size: 12.5px;
  color: var(--text-dim);
  border-right: 1px solid var(--border);
  border-top: none;
  border-bottom: 2px solid transparent;
  border-left: none;
  background: none;
  max-width: 220px;
  white-space: nowrap;
  flex-shrink: 0;
}
.tab.active { background: var(--bg-window); color: var(--text); border-bottom-color: var(--accent); }
.tab .x { color: var(--text-faint); border-radius: 4px; padding: 1px; display: grid; place-items: center; }
.tab .x:hover { background: var(--bg-hover); color: var(--text); }

/* Quickstart */
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

/* Breadcrumb */
.crumbs {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 14px;
  font-size: 12.5px;
  color: var(--text-dim);
  border-bottom: 1px solid var(--border);
  flex: none;
}
.sep { color: var(--text-faint); }
.c-ic { color: var(--text-faint); }

/* Query bar */
.qbar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  flex: none;
  flex-wrap: wrap;
}
.qbtn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 6px;
  background: none;
  border: none;
  color: var(--text);
  font-size: 12.5px;
}
.qbtn:hover:not(:disabled) { background: var(--bg-hover); }
.qbtn.run .ic { color: var(--green); }
.qbtn .ic  { color: var(--text-dim); }
.qbtn .drop { color: var(--text-faint); }
.qbtn:disabled { opacity: .5; cursor: default; }
.qbar-spacer { flex: 1; }
.vqb-toggle {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  color: var(--text-dim);
  font-size: 12px;
}
.vqb-toggle:disabled { opacity: .4; }

/* Query fields */
.qfields {
  padding: 8px 12px 10px;
  display: grid;
  grid-template-columns: auto 1fr auto 1fr auto;
  gap: 8px 12px;
  align-items: center;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.qlabel { font-size: 12.5px; color: var(--text-dim); white-space: nowrap; }
.qinput {
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 6px 10px;
  font-family: var(--mono);
  font-size: 12.5px;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.qinput:focus-within { border-color: var(--accent); }
.qval {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  min-width: 0;
}
.qval::placeholder { color: var(--text-faint); }
.qval-short { max-width: 70px; }
.qicons { display: flex; gap: 4px; color: var(--text-faint); flex: none; }

/* Results */
.results { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.rtabs { display: flex; align-items: stretch; border-bottom: 1px solid var(--border); flex: none; }
.rtab {
  padding: 8px 16px;
  font-size: 12.5px;
  color: var(--text-dim);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
}
.rtab.active { color: var(--text); border-bottom-color: var(--accent); }

.rtoolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 8px;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.icon-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-dim);
  padding: 4px;
  display: grid;
  place-items: center;
}
.icon-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
.icon-btn:disabled { opacity: .4; }
.page-size {
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  padding: 3px 6px;
  font-size: 12px;
  color: var(--text);
}
.docs-range { font-size: 12px; color: var(--text-dim); margin: 0 4px; }
.rtoolbar-spacer { flex: 1; }

/* view mode */
.view-select-wrap { position: relative; }
.view-select {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  padding: 4px 9px;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
}
.view-menu {
  position: absolute;
  right: 0;
  top: 30px;
  width: 160px;
  background: #2a2c30;
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 14px 34px rgba(0,0,0,.55);
  z-index: 20;
  padding: 4px;
}
.view-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: 5px;
  font-size: 12.5px;
  color: var(--text-dim);
  cursor: pointer;
}
.view-menu-item:hover { background: var(--bg-hover); color: var(--text); }
.view-menu-item.on { color: var(--text); }
.view-menu-item:not(.on) span { margin-left: 21px; }

/* Grid */
.grid-wrap { flex: 1; overflow: auto; min-height: 0; background: var(--bg-window); }
.fieldpath {
  padding: 7px 12px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
  background: var(--bg-panel-2);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  z-index: 3;
}
table.grid {
  border-collapse: collapse;
  width: max-content;
  min-width: 100%;
  font-family: var(--mono);
  font-size: 12px;
}
table.grid th {
  position: sticky;
  top: 34px;
  background: var(--bg-toolbar);
  color: var(--text-dim);
  font-weight: 600;
  text-align: left;
  padding: 5px 12px;
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
  z-index: 2;
}
table.grid td {
  padding: 4px 12px;
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  color: var(--text);
  white-space: nowrap;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
}
table.grid tr:nth-child(even) td { background: var(--bg-row-alt); }
table.grid tr:hover td { background: var(--bg-hover); }
table.grid tr.selrow td { background: rgba(59,130,246,.18); }

.tcell { display: inline-flex; align-items: center; gap: 6px; }
.ticon { color: var(--text-faint); display: grid; place-items: center; flex: none; }
.cell-oid { color: var(--link); }
.cell-str { color: var(--cell-str); }
.cell-num { color: var(--cell-num); }
.cell-faint { color: var(--text-faint); }

.empty-rows { background: var(--bg-window); }
.erow { height: 27px; border-bottom: 1px solid var(--border); }
.erow:nth-child(even) { background: var(--bg-row-alt); }

/* JSON view */
.json-view { flex: 1; overflow: auto; padding: 12px 16px; }
.json-doc {
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.65;
  color: var(--text);
  white-space: pre;
  border-left: 2px solid var(--border-soft);
  padding: 8px 0 8px 14px;
  margin-bottom: 10px;
}

/* Footer */
.rfooter {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 6px 12px;
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-dim);
  flex: none;
  background: var(--bg-panel);
}
.spacer { flex: 1; }
.fitem { display: flex; align-items: center; gap: 6px; }
.run-error { padding: 10px 14px; color: #e07070; font-size: 12px; }
</style>
