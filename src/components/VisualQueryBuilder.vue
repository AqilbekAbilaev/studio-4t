<script setup>
import { ref, computed, watch } from 'vue'
import BaseIcon from './BaseIcon.vue'
import {
  OPERATORS,
  detectType,
  generateFilter,
  generateSort,
  generateProjection,
} from '../utils/vqbGenerator'

const props = defineProps({
  tabs:            { type: Array,  required: true },
  activeTabId:     { type: String, required: true },
  draggedField:    { type: String, default: '' },
  // Set by QueryWorkspace when a result cell is dropped on a VQB section.
  // Carries { field, section, nonce } — nonce makes each drop a fresh object
  // so the watcher fires even when the same field lands on the same section twice.
  vqbDrop:         { type: Object, default: null },
  // Which section ('query' | 'proj' | 'sort') the pointer is currently over
  // during a drag, so we can highlight it as the drop target.
  dragOverSection: { type: String, default: null },
})
const activeTab = computed(() => props.tabs.find(t => t.id === props.activeTabId))

// ── query section ─────────────────────────────────────────
const queryEnabled = ref(true)
const logic        = ref('$and')
const conditions   = ref([])

// ── projection section ────────────────────────────────────
const projEnabled = ref(false)
const projFields  = ref([])
const projInput   = ref('')

// ── sort section ──────────────────────────────────────────
const sortEnabled = ref(false)
const sortFields  = ref([])
const sortInput   = ref('')

watch(() => props.activeTabId, () => {
  conditions.value = []
  projFields.value = []
  sortFields.value = []
  projInput.value  = ''
  sortInput.value  = ''
})

// Single watcher covers all state — fires applyAndRun whenever anything changes.
// This is more reliable than @input/@change/@click handlers on every element.
watch(
  [conditions, logic, queryEnabled, projFields, projEnabled, sortFields, sortEnabled],
  applyAndRun,
  { deep: true }
)

// When a column header is clicked in QueryWorkspace, its name lands here and
// is added straight to the Query section.
watch(() => props.draggedField, (field) => {
  if (!field) return
  conditions.value.push({ id: uid(), field: field, op: 'eq', value: '', enabled: true })
  applyAndRun()
})

// When a result cell is dragged and dropped onto one of the sections, add the
// dropped field to that section (Query / Projection / Sort).
watch(() => props.vqbDrop, (drop) => {
  if (!drop) return
  const field = (drop.field || '').trim()
  if (!field) return
  if (drop.section === 'query') {
    queryEnabled.value = true
    conditions.value.push({ id: uid(), field: field, op: 'eq', value: '', enabled: true })
  } else if (drop.section === 'proj') {
    projEnabled.value = true
    projFields.value.push({ id: uid(), field: field, include: true })
  } else if (drop.section === 'sort') {
    sortEnabled.value = true
    sortFields.value.push({ id: uid(), field: field, dir: 1 })
  }
  applyAndRun()
})

function uid() { return Math.random().toString(36).slice(2, 10) }

// ── core: generate fields only, no auto-run ──────────────
// VQB updates the filter/sort/projection text in the query bar.
// The user clicks Run in QueryWorkspace to execute the query.
let timer = null
function applyAndRun() {
  clearTimeout(timer)
  timer = setTimeout(() => {
    const tab = activeTab.value
    if (!tab) return

    const filterStr = queryEnabled.value
      ? generateFilter(conditions.value, logic.value)
      : '{}'
    const sortStr = sortEnabled.value
      ? generateSort(sortFields.value)
      : '{}'
    const projStr = projEnabled.value
      ? generateProjection(projFields.value)
      : '{}'

    tab.filter     = filterStr === '{}' ? '' : filterStr
    tab.sort       = sortStr   === '{}' ? '' : sortStr
    tab.projection = projStr   === '{}' ? '' : projStr
  }, 400)
}

// ── condition helpers ─────────────────────────────────────
function addCondition() {
  conditions.value.push({ id: uid(), field: '', op: 'eq', value: '', enabled: true })
}
function removeCondition(id) {
  conditions.value = conditions.value.filter(c => c.id !== id)
  applyAndRun()
}
function clearAll() {
  conditions.value = []
  applyAndRun()
}
function opNoValue(op) {
  const found = OPERATORS.find(o => o.value === op)
  return found ? found.noValue : false
}

// ── projection helpers ────────────────────────────────────
function addProjField() {
  const f = projInput.value.trim()
  if (!f) return
  projFields.value.push({ id: uid(), field: f, include: true })
  projInput.value = ''
  applyAndRun()
}
function removeProjField(id) {
  projFields.value = projFields.value.filter(f => f.id !== id)
  applyAndRun()
}

// ── sort helpers ──────────────────────────────────────────
function addSortField() {
  const f = sortInput.value.trim()
  if (!f) return
  sortFields.value.push({ id: uid(), field: f, dir: 1 })
  sortInput.value = ''
  applyAndRun()
}
function removeSortField(id) {
  sortFields.value = sortFields.value.filter(f => f.id !== id)
  applyAndRun()
}
</script>

<template>
  <div class="vqb">

    <!-- ── Query section ─────────────────────────────────── -->
    <div class="vqb-section" data-vqb-drop="query"
         :class="{ 'drop-target': props.dragOverSection === 'query' }">
      <div class="vqb-head">
        Query
        <span class="cb" :class="{ on: queryEnabled }"
              @click="queryEnabled = !queryEnabled; applyAndRun()">
          <BaseIcon v-if="queryEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body" v-if="queryEnabled">
        <div class="vqb-row1">
          <div class="vqb-select" @click="logic = logic === '$and' ? '$or' : '$and'; applyAndRun()">
            {{ logic === '$and' ? 'Match all of ($and)' : 'Match any of ($or)' }}
            <BaseIcon name="caretDown" :size="12" />
          </div>
          <button class="vqb-clear" @click="clearAll">Clear</button>
        </div>

        <div class="cond" v-for="c in conditions" :key="c.id">
          <div class="cond-line">
            <input
              class="pill cond-field"
              v-model="c.field"
              placeholder="field"
              @input="applyAndRun"
              spellcheck="false"
            />
            <div class="op-select grow">
              <select class="pill" v-model="c.op" @change="applyAndRun">
                <option v-for="op in OPERATORS" :key="op.value" :value="op.value">
                  {{ op.label }}
                </option>
              </select>
              <BaseIcon name="caretDown" :size="12" class="op-caret" />
            </div>
            <button class="icon-btn sm" @click="removeCondition(c.id)">
              <BaseIcon name="trash" :size="13" />
            </button>
          </div>
          <div class="cond-line" v-if="!opNoValue(c.op)">
            <span class="pill type-pill">{{ detectType(c.value) }}</span>
            <input
              class="pill grow cond-val"
              v-model="c.value"
              :placeholder="c.op === 'in' || c.op === 'nin' ? 'val1, val2, …' : 'value'"
              @input="applyAndRun"
              spellcheck="false"
            />
            <span class="cb sm" :class="{ on: c.enabled }"
                  @click="c.enabled = !c.enabled; applyAndRun()">
              <BaseIcon v-if="c.enabled" name="check" :size="11" />
            </span>
          </div>
        </div>

        <div class="dropzone" @click="addCondition">
          <BaseIcon name="plus" :size="14" />
          Drag a cell here or click to add manually
        </div>
      </div>
    </div>

    <!-- ── Projection section ────────────────────────────── -->
    <div class="vqb-section" data-vqb-drop="proj"
         :class="{ 'drop-target': props.dragOverSection === 'proj' }">
      <div class="vqb-head">
        Projection
        <span class="cb" :class="{ on: projEnabled }"
              @click="projEnabled = !projEnabled; applyAndRun()">
          <BaseIcon v-if="projEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body" v-if="projEnabled">
        <div class="sp-row" v-for="f in projFields" :key="f.id">
          <span class="pill sp-field">{{ f.field }}</span>
          <button
            class="dir-toggle"
            :class="f.include ? 'inc' : 'exc'"
            @click="f.include = !f.include; applyAndRun()"
          >{{ f.include ? 'Include' : 'Exclude' }}</button>
          <button class="icon-btn sm" @click="removeProjField(f.id)">
            <BaseIcon name="trash" :size="13" />
          </button>
        </div>
        <div class="add-field-row">
          <input
            class="add-field-input"
            v-model="projInput"
            placeholder="field name"
            @keydown.enter.prevent="addProjField"
            spellcheck="false"
          />
          <button class="add-field-btn" @click="addProjField">Add</button>
        </div>
      </div>
      <div class="vqb-body" v-else>
        <div class="dropzone" @click="projEnabled = true; applyAndRun()">
          <BaseIcon name="plus" :size="14" />
          Drag a cell here or click to enable projection
        </div>
      </div>
    </div>

    <!-- ── Sort section ──────────────────────────────────── -->
    <div class="vqb-section" data-vqb-drop="sort"
         :class="{ 'drop-target': props.dragOverSection === 'sort' }">
      <div class="vqb-head">
        Sort
        <span class="cb" :class="{ on: sortEnabled }"
              @click="sortEnabled = !sortEnabled; applyAndRun()">
          <BaseIcon v-if="sortEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body" v-if="sortEnabled">
        <div class="sp-row" v-for="f in sortFields" :key="f.id">
          <span class="pill sp-field">{{ f.field }}</span>
          <button
            class="dir-toggle"
            :class="f.dir === 1 ? 'asc' : 'desc'"
            @click="f.dir = f.dir === 1 ? -1 : 1; applyAndRun()"
          >{{ f.dir === 1 ? '↑ ASC' : '↓ DESC' }}</button>
          <button class="icon-btn sm" @click="removeSortField(f.id)">
            <BaseIcon name="trash" :size="13" />
          </button>
        </div>
        <div class="add-field-row">
          <input
            class="add-field-input"
            v-model="sortInput"
            placeholder="field name"
            @keydown.enter.prevent="addSortField"
            spellcheck="false"
          />
          <button class="add-field-btn" @click="addSortField">Add</button>
        </div>
      </div>
      <div class="vqb-body" v-else>
        <div class="dropzone" @click="sortEnabled = true; applyAndRun()">
          <BaseIcon name="plus" :size="14" />
          Drag a cell here or click to enable sort
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.vqb {
  width: 360px;
  flex: none;
  background: var(--bg-panel);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
}
.vqb-section { border-bottom: 1px solid var(--border); position: relative; }
.vqb-section.drop-target { background: rgba(59, 130, 246, .08); }
/* Draw the highlight border as a positioned overlay so it paints on top of the
   section header bar (an in-flow child), whose opaque background would otherwise
   cover an outline / inset shadow along the top edge. */
.vqb-section.drop-target::after {
  content: '';
  position: absolute;
  inset: 0;
  border: 2px solid var(--accent);
  pointer-events: none;
  z-index: 5;
}

.vqb-head {
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  padding: 9px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  background: var(--bg-panel-2);
  user-select: none;
}
.vqb-head .cb { position: absolute; right: 10px; }

.vqb-body { padding: 10px; }

.vqb-row1 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 9px;
}

.vqb-select {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 6px 9px;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
  user-select: none;
}
.vqb-select:hover { border-color: var(--accent); }

.vqb-clear {
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-toolbar);
  color: var(--text);
  font-size: 12.5px;
  cursor: pointer;
}
.vqb-clear:hover { background: var(--bg-hover); }

/* checkbox */
.cb {
  width: 17px; height: 17px;
  border-radius: 4px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  display: grid;
  place-items: center;
  flex: none;
  cursor: pointer;
}
.cb.on { background: var(--accent); border-color: var(--accent); color: #fff; }
.cb.sm { width: 15px; height: 15px; border-radius: 3px; }

/* dropzone */
.dropzone {
  border: 1px dashed var(--border-soft);
  border-radius: 6px;
  padding: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-faint);
  cursor: pointer;
  user-select: none;
}
.dropzone:hover  { border-color: var(--accent); color: var(--accent); }

/* condition rows */
.cond {
  margin-bottom: 8px;
  background: var(--bg-panel-2);
  border-radius: 6px;
  padding: 6px 8px;
}
.cond-line {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-bottom: 4px;
}
.cond-line:last-child { margin-bottom: 0; }

/* pills */
.pill {
  border: 1px solid var(--border-soft);
  border-radius: 5px;
  background: var(--bg-input);
  color: var(--text);
  font-size: 12px;
  padding: 4px 7px;
  outline: none;
  min-width: 0;
}
.pill:focus { border-color: var(--accent); }
.pill.grow  { flex: 1; }
.cond-field { width: 90px; flex: none; }
.type-pill  {
  flex: none;
  font-size: 11px;
  color: var(--text-faint);
  background: var(--bg-panel);
  white-space: nowrap;
  cursor: default;
}
.cond-val   { font-family: var(--mono); }

/* operator dropdown — strip native chrome, overlay a token-colored caret */
.op-select { position: relative; display: flex; }
.op-select.grow { flex: 1; }
.op-select select.pill {
  width: 100%;
  cursor: pointer;
  appearance: none;
  -webkit-appearance: none;
  padding-right: 24px;
}
.op-caret {
  position: absolute;
  right: 7px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-faint);
  pointer-events: none;
}

/* icon buttons */
.icon-btn.sm {
  background: none; border: none;
  color: var(--text-faint); padding: 3px;
  border-radius: 4px; cursor: pointer;
  display: flex; align-items: center; flex: none;
}
.icon-btn.sm:hover { background: var(--bg-hover); color: var(--text); }

/* sort / projection rows */
.sp-row {
  display: flex; align-items: center; gap: 5px; margin-bottom: 6px;
}
.sp-field {
  flex: 1; font-family: var(--mono); font-size: 11.5px; cursor: default;
}
.dir-toggle {
  font-size: 11px; padding: 4px 8px; border-radius: 5px;
  cursor: pointer; border: 1px solid var(--border-soft);
  background: var(--bg-input); color: var(--text-dim); white-space: nowrap; flex: none;
}
.dir-toggle.asc  { color: var(--accent); border-color: var(--accent); }
.dir-toggle.desc { color: var(--link);   border-color: var(--link);   }
.dir-toggle.inc  { color: var(--green);  border-color: var(--green);  }
.dir-toggle.exc  { color: var(--prod);   border-color: var(--prod);   }

/* add-field row */
.add-field-row { display: flex; gap: 6px; margin-top: 4px; }
.add-field-input {
  flex: 1; background: var(--bg-input); border: 1px solid var(--border-soft);
  border-radius: 5px; color: var(--text); font-size: 12px;
  padding: 5px 8px; outline: none; min-width: 0;
}
.add-field-input:focus { border-color: var(--accent); }
.add-field-btn {
  padding: 5px 12px; border-radius: 5px;
  background: var(--accent); border: none;
  color: #fff; font-size: 12px; cursor: pointer; white-space: nowrap;
}
.add-field-btn:hover { background: var(--accent-soft); }
</style>
