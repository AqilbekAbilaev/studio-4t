<script setup>
import { ref, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import BaseIcon from './BaseIcon.vue'

// IntelliShell console. Bound to a shell tab (connection + database). Each tab
// carries its own backend session (tab.sessionId), so variables persist across
// submissions. Phase 1: plain JS — print() output, the completion value, and JS
// errors. The db.* bridge arrives in a later phase.
const props = defineProps({
  activeTab: { type: Object, required: true },
})

const input       = ref('')
const transcript  = ref(null)   // scrollback element, for autoscroll
const histIndex   = ref(-1)     // position while cycling history with ↑/↓

function scrollToEnd() {
  nextTick(() => {
    const el = transcript.value
    if (el) el.scrollTop = el.scrollHeight
  })
}

// Pretty-print the completion value. Phase 1 uses plain JSON; a shared
// mongosh-style formatter (ObjectId(...) etc.) is wired in a later phase.
function formatValue(value) {
  if (value === undefined || value === null) return ''
  try {
    return JSON.stringify(value, null, 2)
  } catch (_) {
    return String(value)
  }
}

async function run() {
  const code = input.value
  if (!code.trim() || props.activeTab.isRunning) return

  const entry = { command: code, logs: [], value: undefined, error: null }
  props.activeTab.entries.push(entry)
  props.activeTab.history.push(code)
  histIndex.value = -1
  input.value = ''
  props.activeTab.isRunning = true
  scrollToEnd()

  try {
    const res = await invoke('run_shell_command', {
      sessionId: props.activeTab.sessionId,
      code:      code,
    })
    entry.logs  = res.logs || []
    entry.value = res.value
    entry.error = res.error
  } catch (e) {
    entry.error = String(e)
  } finally {
    props.activeTab.isRunning = false
    scrollToEnd()
  }
}

// ↑/↓ cycle through previously submitted commands (most recent first).
function onKeydown(e) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    run()
    return
  }
  const hist = props.activeTab.history
  if (e.key === 'ArrowUp' && !e.shiftKey) {
    if (!hist.length) return
    e.preventDefault()
    histIndex.value = histIndex.value < 0 ? hist.length - 1 : Math.max(0, histIndex.value - 1)
    input.value = hist[histIndex.value]
  } else if (e.key === 'ArrowDown' && !e.shiftKey) {
    if (histIndex.value < 0) return
    e.preventDefault()
    if (histIndex.value >= hist.length - 1) {
      histIndex.value = -1
      input.value = ''
    } else {
      histIndex.value += 1
      input.value = hist[histIndex.value]
    }
  }
}
</script>

<template>
  <div class="shell">
    <!-- Breadcrumb -->
    <div class="crumbs">
      <BaseIcon name="connect" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.connectionName }}</span>
      <BaseIcon name="caret" :size="11" class="sep" />
      <BaseIcon name="dbSmall" :size="15" class="c-ic" />
      <span class="crumb">{{ activeTab.dbName }}</span>
      <span class="shell-tag"><BaseIcon name="shell" :size="13" /> IntelliShell</span>
    </div>

    <!-- Scrollback transcript -->
    <div class="transcript" ref="transcript">
      <div v-if="!activeTab.entries.length" class="hint">
        Type JavaScript and press Enter to run. Shift+Enter for a new line. ↑/↓ for history.
      </div>
      <div v-for="(entry, i) in activeTab.entries" :key="i" class="entry">
        <pre class="cmd"><span class="prompt">&gt;</span> {{ entry.command }}</pre>
        <pre v-for="(line, j) in entry.logs" :key="j" class="log">{{ line }}</pre>
        <pre v-if="entry.error" class="err">{{ entry.error }}</pre>
        <pre v-else-if="entry.value !== undefined && entry.value !== null" class="val">{{ formatValue(entry.value) }}</pre>
      </div>
    </div>

    <!-- Input -->
    <div class="input-row">
      <span class="in-prompt">&gt;</span>
      <textarea
        class="in"
        v-model="input"
        :disabled="activeTab.isRunning"
        @keydown="onKeydown"
        placeholder="db is coming soon — try: 1 + 1, print('hi'), var x = 5"
        spellcheck="false"
        autocorrect="off"
        autocapitalize="off"
        rows="2"
      ></textarea>
    </div>
  </div>
</template>

<style scoped>
.shell { flex: 1; display: flex; flex-direction: column; min-height: 0; background: var(--bg-window); }

.crumbs {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 14px; border-bottom: 1px solid var(--border);
  flex: none; font-size: 13px; color: var(--text);
}
.crumbs .c-ic { color: var(--text-dim); }
.crumbs .sep  { color: var(--text-faint); }
.crumbs .crumb { color: var(--text); }
.shell-tag {
  margin-left: auto; display: flex; align-items: center; gap: 5px;
  font-size: 11px; color: var(--text-dim);
}

.transcript {
  flex: 1; overflow: auto; padding: 12px 16px;
  font-family: var(--mono); font-size: 12.5px; line-height: 1.4;
  min-height: 0;
}
.hint { color: var(--text-faint); font-size: 12px; }
.entry { margin-bottom: 10px; }
.cmd { color: var(--text); white-space: pre-wrap; word-break: break-word; margin: 0; }
.prompt { color: var(--accent); margin-right: 6px; }
.log { color: var(--text-dim); white-space: pre-wrap; word-break: break-word; margin: 2px 0 0; }
.val { color: var(--cell-num); white-space: pre-wrap; word-break: break-word; margin: 2px 0 0; }
.err { color: #e0625b; white-space: pre-wrap; word-break: break-word; margin: 2px 0 0; }

.input-row {
  flex: none; display: flex; align-items: flex-start; gap: 8px;
  padding: 10px 16px; border-top: 1px solid var(--border); background: var(--bg-panel);
}
.in-prompt { color: var(--accent); font-family: var(--mono); font-size: 13px; padding-top: 7px; }
.in {
  flex: 1; resize: vertical; min-height: 38px;
  background: var(--bg-input); border: 1px solid var(--border-soft); border-radius: 6px;
  color: var(--text); font-family: var(--mono); font-size: 12.5px;
  padding: 8px 10px; outline: none;
}
.in:focus { border-color: var(--accent); }
.in:disabled { opacity: .6; }
</style>
