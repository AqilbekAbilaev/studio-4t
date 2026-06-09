<script setup>
import { inject, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { getImageUrl } from "../../utils"

const activeCollection = inject('activeCollection')
const query = ref('')
const results = ref([])
const hasRun = ref(false)
const isRunning = ref(false)
const runError = ref(null)

async function runQuery() {
    if (!activeCollection.value) return
    isRunning.value = true
    runError.value = null
    try {
        results.value = await invoke('find_documents', {
            id: activeCollection.value.connectionId,
            uri: activeCollection.value.uri,
            database: activeCollection.value.dbName,
            collection: activeCollection.value.collectionName,
            filter: query.value.trim() || '{}',
        })
        hasRun.value = true
    } catch (e) {
        runError.value = String(e)
        results.value = []
    } finally {
        isRunning.value = false
    }
}

function clearQuery() {
    query.value = ''
}
</script>
<template>
    <div class="tab">
        <div class="operations">
            <button class="run-btn pointer" :disabled="!activeCollection || isRunning" @click="runQuery">
                <img :src="getImageUrl('triangle.svg')" width="16px" />
                <p class="btn-text">{{ isRunning ? 'Running...' : 'Run' }}</p>
            </button>
            <button class="run-btn load-query-btn pointer" disabled>
                <img :src="getImageUrl('folder.svg')" width="16px" />
                <p class="btn-text">Load query</p>
            </button>
            <button class="run-btn save-query-btn pointer" disabled>
                <img :src="getImageUrl('save.svg')" width="16px" />
                <p class="btn-text">Save query</p>
            </button>
            <button class="run-btn visual-query-builder pointer" disabled>
                <img :src="getImageUrl('visual-query-builder.svg')" width="16px" />
                <p class="btn-text">Visual Query Builder</p>
            </button>
        </div>

        <div class="operations">
            <div class="query-container">
                <label for="query-input">Query</label>
                <input
                    v-model="query"
                    class="query"
                    id="query-input"
                    placeholder="{}"
                    @keydown.enter.ctrl="runQuery"
                    @keydown.enter.meta="runQuery"
                />
                <button class="run-btn pointer clear-btn" @click="clearQuery" title="Clear">
                    <img :src="getImageUrl('broom.png')" width="18px" />
                </button>
                <button class="run-btn pointer clear-btn" disabled title="Fullscreen">
                    <img :src="getImageUrl('fullscreen.svg')" width="16px" />
                </button>
            </div>
        </div>

        <div class="collection-label" v-if="activeCollection">
            {{ activeCollection.dbName }} / {{ activeCollection.collectionName }}
        </div>

        <div class="results-area">
            <div v-if="!activeCollection" class="placeholder">
                Select a collection from the sidebar to run a query
            </div>
            <div v-else-if="runError" class="run-error">{{ runError }}</div>
            <div v-else-if="isRunning" class="placeholder">Running query...</div>
            <div v-else-if="hasRun && results.length === 0" class="placeholder">No documents found</div>
            <div v-else-if="hasRun">
                <div class="results-header">{{ results.length }} document{{ results.length === 1 ? '' : 's' }}</div>
                <div class="documents">
                    <div class="document" v-for="(doc, i) in results" :key="i">
                        <pre>{{ JSON.stringify(doc, null, 2) }}</pre>
                    </div>
                </div>
            </div>
            <div v-else class="placeholder">Press Run or Ctrl+Enter to execute the query</div>
        </div>
    </div>
</template>

<style scoped>
.tab {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.operations {
    border-bottom: 0.4px solid rgba(148, 148, 148, 0.253);
    padding: 10px;
    display: flex;
    column-gap: 10px;
    flex-shrink: 0;
}

.run-btn {
    background-color: transparent;
    border: 0;
    display: flex;
    align-items: center;
    column-gap: 10px;
    border: 1px solid rgb(197, 197, 197);
    color: white;
    padding: 4px 6px;
}

.run-btn:active:not(:disabled) {
    transform: scale(0.975);
    background: rgb(41, 41, 41);
}

.run-btn:disabled {
    opacity: 0.4;
    cursor: default;
}

.visual-query-builder {
    margin-left: auto;
}

input.query {
    flex-grow: 1;
}

.query {
    flex-grow: 2;
    background: none;
    padding: 4px;
    border: 1px solid #272727;
    color: white;
    position: relative;
}

.query-container {
    flex: 1;
    display: flex;
    align-items: center;
    column-gap: 10px;
}

.clear-btn {
    border: none;
}

.collection-label {
    padding: 4px 10px;
    font-size: 11px;
    color: #aaa;
    background-color: #2a2a2a;
    border-bottom: 0.4px solid rgba(148, 148, 148, 0.253);
    flex-shrink: 0;
}

.results-area {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
}

.placeholder {
    color: #666;
    font-size: 12px;
    text-align: center;
    padding: 24px 8px;
}

.run-error {
    color: #e07070;
    font-size: 12px;
    padding: 8px;
    word-break: break-word;
}

.results-header {
    font-size: 11px;
    color: #888;
    padding: 4px 0 8px;
}

.documents {
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.document {
    background-color: #2a2a2a;
    border: 1px solid #404040;
    border-radius: 4px;
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
