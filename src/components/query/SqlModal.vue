<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errMessage } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import StateMessage from '../base/StateMessage.vue'

// Top-bar "SQL" tool. Translates a simple SQL SELECT into the equivalent MongoDB
// find query (filter / projection / sort / limit / skip) and the shell command,
// the way Studio-3T's SQL Query surface does. Pure translation — no connection.
defineEmits(['close'])

const EXAMPLE = "SELECT name, age FROM users\nWHERE age >= 18 AND status IN ('active', 'trial')\nORDER BY age DESC\nLIMIT 20"

const sql = ref(EXAMPLE)
const result = ref(null)
const error = ref(null)
const copied = ref(false)

async function translate() {
  error.value = null
  copied.value = false
  try {
    result.value = await invoke('translate_sql', { sql: sql.value })
  } catch (e) {
    error.value = errMessage(e)
    result.value = null
  }
}

// Compact a pretty JSON string onto one line for the shell command; falls back to
// the original text if it somehow does not parse.
function compact(json) {
  try {
    return JSON.stringify(JSON.parse(json))
  } catch (_) {
    return json
  }
}

// Assemble db.<collection>.find(...).sort(...).skip(...).limit(...), including
// only the clauses the query actually uses.
const shellCommand = computed(() => {
  const r = result.value
  if (!r) return ''
  const filter = compact(r.filter)
  const projection = compact(r.projection)
  let cmd = `db.${r.collection}.find(${filter}`
  if (projection !== '{}') cmd += `, ${projection}`
  cmd += ')'
  if (compact(r.sort) !== '{}') cmd += `.sort(${compact(r.sort)})`
  if (r.skip != null) cmd += `.skip(${r.skip})`
  if (r.limit != null) cmd += `.limit(${r.limit})`
  return cmd
})

async function copyShell() {
  try {
    await navigator.clipboard.writeText(shellCommand.value)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1500)
  } catch (_) {}
}

function onKeydown(e) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
    e.preventDefault()
    translate()
  }
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">SQL → MongoDB</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="sq-body">
        <label class="sq-lbl">SQL query</label>
        <textarea
          v-model="sql"
          class="sq-input"
          spellcheck="false"
          rows="5"
          placeholder="SELECT * FROM collection WHERE field = value ORDER BY field LIMIT n"
          @keydown="onKeydown"
        ></textarea>

        <div class="sq-actions">
          <button class="sq-run" @click="translate">Translate</button>
          <span class="sq-hint">⌘/Ctrl + Enter</span>
        </div>

        <StateMessage v-if="error" mode="error" :message="error" />

        <template v-if="result && !error">
          <div class="sq-out-head">
            <span class="sq-lbl">MongoDB query</span>
            <button class="sq-copy" @click="copyShell">
              <BaseIcon :name="copied ? 'check' : 'copy'" :size="12" />
              {{ copied ? 'Copied' : 'Copy' }}
            </button>
          </div>
          <pre class="sq-shell">{{ shellCommand }}</pre>

          <div class="sq-parts">
            <div class="sq-part">
              <span class="sq-part-lbl">Collection</span>
              <code class="sq-part-val">{{ result.collection }}</code>
            </div>
            <div class="sq-part">
              <span class="sq-part-lbl">Filter</span>
              <pre class="sq-json">{{ result.filter }}</pre>
            </div>
            <div class="sq-part" v-if="result.projection !== '{}'">
              <span class="sq-part-lbl">Projection</span>
              <pre class="sq-json">{{ result.projection }}</pre>
            </div>
            <div class="sq-part" v-if="result.sort !== '{}'">
              <span class="sq-part-lbl">Sort</span>
              <pre class="sq-json">{{ result.sort }}</pre>
            </div>
            <div class="sq-part" v-if="result.limit != null || result.skip != null">
              <span class="sq-part-lbl">Paging</span>
              <code class="sq-part-val">
                <template v-if="result.skip != null">skip {{ result.skip }}</template>
                <template v-if="result.limit != null"> limit {{ result.limit }}</template>
              </code>
            </div>
          </div>
        </template>
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
  width: 620px;
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

.sq-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 74vh;
  overflow-y: auto;
}
.sq-lbl {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.sq-input {
  width: 100%;
  box-sizing: border-box;
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.5;
  resize: vertical;
}
.sq-input:focus { outline: none; border-color: var(--accent); }
.sq-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 2px 0 4px;
}
.sq-run {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 5px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.sq-run:hover { background: var(--accent-soft); }
.sq-hint { font-size: 11.5px; color: var(--text-faint); }

.sq-out-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 6px;
}
.sq-copy {
  display: flex;
  align-items: center;
  gap: 5px;
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 5px;
  padding: 3px 8px;
  font-size: 11.5px;
  cursor: pointer;
}
.sq-copy:hover { background: var(--bg-hover); color: var(--text); }
.sq-shell {
  margin: 0;
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  color: var(--cell-op);
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text;
}
.sq-parts {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 4px;
}
.sq-part { display: flex; flex-direction: column; gap: 3px; }
.sq-part-lbl {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.sq-part-val {
  font-family: var(--mono);
  font-size: 12.5px;
  color: var(--text);
  user-select: text;
}
.sq-json {
  margin: 0;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-dim);
  white-space: pre;
  overflow-x: auto;
  user-select: text;
}
</style>
