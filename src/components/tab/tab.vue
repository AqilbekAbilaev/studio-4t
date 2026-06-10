<script setup>
import { inject, ref, computed } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { getImageUrl } from "../../utils"

const activeCollection = inject('activeCollection')

// Query inputs
const filter = ref('')
const projection = ref('')
const sort = ref('')
const skip = ref(0)
const limit = ref(50)

// Results state
const results = ref([])
const hasRun = ref(false)
const isRunning = ref(false)
const runError = ref(null)
const viewMode = ref('table') // 'table' | 'json'
const selectedRow = ref(null)

// Convert MongoDB-style query (unquoted keys) to strict JSON.
// Handles the common case: { name: "x", age: 25 } → { "name": "x", "age": 25 }
function toStrictJson(raw) {
    const s = raw.trim()
    if (!s || s === '{}') return '{}'
    // Quote any bare identifier key that isn't already quoted.
    // Matches: optional leading { or , then whitespace, then identifier, then :
    return s.replace(/([{,]\s*)([a-zA-Z_$][a-zA-Z0-9_$.]*)\s*:/g, '$1"$2":')
}

async function runQuery() {
    if (!activeCollection.value) return
    isRunning.value = true
    runError.value = null
    selectedRow.value = null
    try {
        results.value = await invoke('find_documents', {
            id: activeCollection.value.connectionId,
            uri: activeCollection.value.uri,
            database: activeCollection.value.dbName,
            collection: activeCollection.value.collectionName,
            filter: toStrictJson(filter.value),
            projection: toStrictJson(projection.value),
            sort: toStrictJson(sort.value),
            skip: skip.value || 0,
            limit: limit.value || 50,
        })
        hasRun.value = true
    } catch (e) {
        runError.value = String(e)
        results.value = []
    } finally {
        isRunning.value = false
    }
}

// Auto-detect columns from result set; _id always first
const columns = computed(() => {
    if (!results.value.length) return []
    const seen = new Set()
    for (const doc of results.value) {
        for (const key of Object.keys(doc)) seen.add(key)
    }
    const rest = [...seen].filter(k => k !== '_id').sort()
    return seen.has('_id') ? ['_id', ...rest] : rest
})

function formatCellValue(value) {
    if (value === null || value === undefined) return ''
    if (typeof value === 'string') return value
    if (typeof value === 'number' || typeof value === 'boolean') return String(value)
    if (Array.isArray(value)) return `Array(${value.length})`
    if (typeof value === 'object') {
        if ('$oid' in value) return value.$oid
        if ('$date' in value) {
            const d = value.$date
            if (typeof d === 'string') return d
            if (typeof d === 'object' && '$numberLong' in d)
                return new Date(parseInt(d.$numberLong)).toISOString()
            return String(d)
        }
        if ('$numberLong' in value) return value.$numberLong
        if ('$numberDecimal' in value) return value.$numberDecimal
        if ('$binary' in value) return `Binary(...)`
        return '{...}'
    }
    return JSON.stringify(value)
}

function cellClass(value) {
    if (value === null || value === undefined) return 'val-null'
    if (typeof value === 'number') return 'val-number'
    if (typeof value === 'boolean') return 'val-bool'
    if (typeof value === 'object') {
        if ('$oid' in value) return 'val-oid'
        if ('$date' in value) return 'val-date'
        if (Array.isArray(value)) return 'val-array'
        return 'val-object'
    }
    return 'val-string'
}
</script>

<template>
    <div class="tab">
        <!-- Toolbar -->
        <div class="toolbar">
            <button class="tool-btn pointer" :disabled="!activeCollection || isRunning" @click="runQuery">
                <img :src="getImageUrl('triangle.svg')" width="14px" />
                <span>{{ isRunning ? 'Running…' : 'Run' }}</span>
            </button>
            <button class="tool-btn pointer" disabled>
                <img :src="getImageUrl('folder.svg')" width="14px" />
                <span>Load query</span>
            </button>
            <button class="tool-btn pointer" disabled>
                <img :src="getImageUrl('save.svg')" width="14px" />
                <span>Save query</span>
            </button>
            <button class="tool-btn pointer visual-qb" disabled>
                <img :src="getImageUrl('visual-query-builder.svg')" width="14px" />
                <span>Visual Query Builder</span>
            </button>
        </div>

        <!-- Query fields -->
        <div class="query-row">
            <span class="field-label">Query</span>
            <input
                v-model="filter"
                class="query-input"
                placeholder="{}"
                @keydown.enter.ctrl="runQuery"
                @keydown.enter.meta="runQuery"
            />
            <button class="icon-btn pointer" @click="filter = ''" title="Clear query">
                <img :src="getImageUrl('broom.png')" width="16px" />
            </button>
        </div>
        <div class="query-row two-col">
            <span class="field-label">Projection</span>
            <input v-model="projection" class="query-input" placeholder="{}" />
            <span class="field-label field-label-right">Sort</span>
            <input v-model="sort" class="query-input" placeholder="{}" />
        </div>
        <div class="query-row two-col">
            <span class="field-label">Skip</span>
            <input v-model.number="skip" class="query-input query-input-short" type="number" min="0" placeholder="0" />
            <span class="field-label field-label-right">Limit</span>
            <input v-model.number="limit" class="query-input query-input-short" type="number" min="1" max="1000" placeholder="50" />
        </div>

        <!-- Results header bar -->
        <div class="results-bar" v-if="activeCollection">
            <span class="collection-path">
                {{ activeCollection.dbName }} › {{ activeCollection.collectionName }}
            </span>
            <span class="doc-count" v-if="hasRun && !runError">
                {{ results.length }} document{{ results.length === 1 ? '' : 's' }}
            </span>
            <div class="view-toggle" v-if="hasRun && results.length">
                <button :class="['toggle-btn', { active: viewMode === 'table' }]" @click="viewMode = 'table'">Table</button>
                <button :class="['toggle-btn', { active: viewMode === 'json' }]" @click="viewMode = 'json'">JSON</button>
            </div>
        </div>

        <!-- Results area -->
        <div class="results-area">
            <div v-if="!activeCollection" class="placeholder">
                Select a collection from the sidebar to run a query
            </div>
            <div v-else-if="runError" class="run-error">{{ runError }}</div>
            <div v-else-if="isRunning" class="placeholder">Running query…</div>
            <div v-else-if="hasRun && results.length === 0" class="placeholder">No documents found</div>

            <!-- Table view -->
            <div v-else-if="hasRun && viewMode === 'table'" class="table-wrapper">
                <table class="result-table">
                    <thead>
                        <tr>
                            <th class="row-num">#</th>
                            <th v-for="col in columns" :key="col">{{ col }}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="(doc, i) in results"
                            :key="i"
                            :class="{ selected: selectedRow === i }"
                            @click="selectedRow = i"
                        >
                            <td class="row-num">{{ i + 1 + skip }}</td>
                            <td v-for="col in columns" :key="col">
                                <span :class="cellClass(doc[col])">{{ formatCellValue(doc[col]) }}</span>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <!-- JSON view -->
            <div v-else-if="hasRun && viewMode === 'json'" class="json-view">
                <div class="document" v-for="(doc, i) in results" :key="i">
                    <pre>{{ JSON.stringify(doc, null, 2) }}</pre>
                </div>
            </div>

            <div v-else-if="activeCollection" class="placeholder">
                Press Run or Ctrl+Enter to execute the query
            </div>
        </div>
    </div>
</template>

<style scoped>
.tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
}

/* ── Toolbar ── */
.toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid #404040;
    flex-shrink: 0;
}

.tool-btn {
    background: transparent;
    border: 1px solid #555;
    color: #ddd;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    font-size: 12px;
    white-space: nowrap;
}

.tool-btn:hover:not(:disabled) {
    background: #3a3a3a;
}

.tool-btn:active:not(:disabled) {
    background: #2a2a2a;
}

.tool-btn:disabled {
    opacity: 0.4;
    cursor: default;
}

.visual-qb {
    margin-left: auto;
}

/* ── Query rows ── */
.query-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid #353535;
    flex-shrink: 0;
}

.two-col {
    gap: 6px;
}

.field-label {
    font-size: 11px;
    color: #888;
    min-width: 58px;
    flex-shrink: 0;
}

.field-label-right {
    margin-left: 8px;
}

.query-input {
    flex: 1;
    background: #1e1e1e;
    border: 1px solid #3a3a3a;
    color: #d4d4d4;
    padding: 3px 6px;
    font-family: 'Menlo', 'Monaco', monospace;
    font-size: 12px;
}

.query-input:focus {
    outline: none;
    border-color: #1268da;
}

.query-input-short {
    max-width: 80px;
    flex: none;
}

.icon-btn {
    background: transparent;
    border: none;
    padding: 2px 4px;
    opacity: 0.7;
    flex-shrink: 0;
}

.icon-btn:hover { opacity: 1; }

/* ── Results bar ── */
.results-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 3px 8px;
    background: #2a2a2a;
    border-bottom: 1px solid #404040;
    flex-shrink: 0;
}

.collection-path {
    font-size: 11px;
    color: #888;
    flex: 1;
}

.doc-count {
    font-size: 11px;
    color: #aaa;
}

.view-toggle {
    display: flex;
    gap: 2px;
}

.toggle-btn {
    background: transparent;
    border: 1px solid #555;
    color: #aaa;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
}

.toggle-btn.active {
    background: #1268da;
    border-color: #1268da;
    color: white;
}

/* ── Results area ── */
.results-area {
    flex: 1;
    overflow: auto;
    min-height: 0;
}

.placeholder {
    color: #555;
    font-size: 12px;
    text-align: center;
    padding: 32px 12px;
}

.run-error {
    color: #e07070;
    font-size: 12px;
    padding: 10px;
    word-break: break-word;
}

/* ── Table ── */
.table-wrapper {
    width: 100%;
    height: 100%;
    overflow: auto;
}

.result-table {
    border-collapse: collapse;
    width: 100%;
    table-layout: auto;
    font-size: 12px;
}

.result-table thead {
    position: sticky;
    top: 0;
    z-index: 1;
    background: #252525;
}

.result-table th {
    padding: 5px 10px;
    text-align: left;
    font-weight: 500;
    color: #bbb;
    border-bottom: 1px solid #404040;
    border-right: 1px solid #333;
    white-space: nowrap;
}

.result-table td {
    padding: 3px 10px;
    border-bottom: 1px solid #2c2c2c;
    border-right: 1px solid #2c2c2c;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.result-table tbody tr:hover {
    background: #2e2e2e;
}

.result-table tbody tr.selected {
    background: #1a3a6e;
}

.row-num {
    color: #555;
    font-size: 11px;
    text-align: right;
    min-width: 32px;
    user-select: none;
}

/* ── Cell value colours ── */
.val-oid   { color: #ce9178; font-family: monospace; }
.val-date  { color: #9cdcfe; }
.val-string{ color: #d4d4d4; }
.val-number{ color: #b5cea8; }
.val-bool  { color: #569cd6; }
.val-null  { color: #555; font-style: italic; }
.val-array { color: #888; }
.val-object{ color: #888; }

/* ── JSON view ── */
.json-view {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.document {
    background: #2a2a2a;
    border: 1px solid #404040;
    border-radius: 3px;
    padding: 8px;
}

.document pre {
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    color: #d4d4d4;
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
}
</style>
