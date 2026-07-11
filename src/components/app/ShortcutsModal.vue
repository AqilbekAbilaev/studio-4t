<script setup>
import { computed, reactive, ref } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'
import { SHORTCUT_COMMANDS, defaultAccel, accelToTokens, accelFromEvent } from '../../utils/keybindings'

// Keyboard shortcuts: the top section is customizable (the menu actions the app
// can rebind); the reference groups below list the fixed shortcuts the editors
// and grid handle. Rebinds are saved via the parent (persisted + applied live on
// Linux, and on the native menu bar at next launch).
const props = defineProps({
  bindings: { type: Object, default: () => ({}) },
})
const emit = defineEmits(['close', 'save'])

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || '')
const mod = isMac ? '⌘' : 'Ctrl'

// A local working copy so edits can be reviewed and saved (or discarded) as a
// batch rather than mutating the live bindings on every keystroke.
const working = reactive({})
for (const cmd of SHORTCUT_COMMANDS) {
  working[cmd.id] = props.bindings[cmd.id] || cmd.default
}

const capturingId = ref(null)   // command id whose row is listening for a key
const conflict = ref(null)      // { label } the last capture collided with

const dirty = computed(() =>
  SHORTCUT_COMMANDS.some((cmd) => working[cmd.id] !== (props.bindings[cmd.id] || cmd.default))
)

function tokens(accel) {
  return accelToTokens(accel, isMac)
}

function startCapture(id) {
  conflict.value = null
  capturingId.value = id
}

function cancelCapture() {
  capturingId.value = null
  conflict.value = null
}

// While a row is capturing, turn the keypress into an accelerator. Reject a combo
// already bound to another command (reassigning would leave that one unbound and
// silently fall back to its default — a confusing duplicate), keeping the row in
// capture so the user can try again or press Esc.
function onCaptureKeydown(id, e) {
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Escape') {
    cancelCapture()
    return
  }
  const accel = accelFromEvent(e)
  if (!accel) return
  const clash = SHORTCUT_COMMANDS.find((cmd) => cmd.id !== id && working[cmd.id] === accel)
  if (clash) {
    conflict.value = { label: clash.label }
    return
  }
  working[id] = accel
  cancelCapture()
}

function resetOne(id) {
  working[id] = defaultAccel(id)
  if (capturingId.value === id) cancelCapture()
}

function resetAll() {
  for (const cmd of SHORTCUT_COMMANDS) working[cmd.id] = cmd.default
  cancelCapture()
}

function save() {
  const payload = {}
  for (const cmd of SHORTCUT_COMMANDS) payload[cmd.id] = working[cmd.id]
  emit('save', payload)
  emit('close')
}

// Fixed reference shortcuts handled directly by the editors and results grid.
const REFERENCE = computed(() => [
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
        <!-- Customizable menu shortcuts -->
        <section class="sc-group">
          <div class="sc-group-head">
            <h3 class="sc-group-title">Menu shortcuts</h3>
            <button class="sc-reset-all" @click="resetAll">Reset all</button>
          </div>

          <div v-for="cmd in SHORTCUT_COMMANDS" :key="cmd.id" class="sc-edit-row">
            <span class="sc-desc">{{ cmd.label }}</span>

            <button
              v-if="capturingId !== cmd.id"
              class="sc-binding"
              title="Click, then press the new shortcut"
              @click="startCapture(cmd.id)"
            >
              <span class="sc-keys">
                <template v-for="(k, i) in tokens(working[cmd.id])" :key="i">
                  <kbd>{{ k }}</kbd><span v-if="i < tokens(working[cmd.id]).length - 1" class="sc-plus">+</span>
                </template>
              </span>
            </button>

            <span
              v-else
              class="sc-binding capturing"
              tabindex="0"
              :ref="(el) => el && el.focus()"
              @keydown="onCaptureKeydown(cmd.id, $event)"
              @blur="cancelCapture"
            >Press a shortcut… <span class="sc-esc">Esc to cancel</span></span>

            <button
              class="sc-row-reset"
              :disabled="working[cmd.id] === cmd.default"
              title="Reset to default"
              @click="resetOne(cmd.id)"
            >Reset</button>
          </div>

          <p v-if="conflict" class="sc-conflict">
            That shortcut is already used by “{{ conflict.label }}”. Pick another.
          </p>
        </section>

        <!-- Fixed reference -->
        <section v-for="group in REFERENCE" :key="group.title" class="sc-group">
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

      <div class="sc-footer">
        <button class="btn ghost" @click="$emit('close')">Close</button>
        <button class="btn primary" :disabled="!dirty" @click="save">Save changes</button>
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
  width: 560px;
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
  max-height: 66vh;
  overflow-y: auto;
}
.sc-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.sc-group-title {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: .05em;
  color: var(--text-faint);
}
.sc-group-head .sc-group-title { margin: 0; }
.sc-reset-all {
  background: none;
  border: none;
  color: var(--link);
  cursor: pointer;
  font-size: 12px;
  padding: 2px 4px;
  border-radius: 4px;
}
.sc-reset-all:hover { text-decoration: underline; }

.sc-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 4px 0;
}
.sc-edit-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 0;
}
.sc-edit-row .sc-desc { flex: 1; }

.sc-keys {
  flex: none;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}
.sc-row .sc-keys { width: 170px; }
.sc-plus { color: var(--text-faint); font-size: 11px; }
.sc-desc { font-size: 13px; color: var(--text); }

.sc-binding {
  min-width: 150px;
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 5px 8px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 12px;
}
.sc-binding:hover { border-color: var(--border-soft); }
.sc-binding.capturing {
  border-color: var(--accent);
  color: var(--text-dim);
  cursor: default;
  outline: none;
}
.sc-esc { color: var(--text-faint); margin-left: 6px; font-size: 11px; }

.sc-row-reset {
  flex: none;
  background: none;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 12px;
  padding: 4px 6px;
  border-radius: 4px;
}
.sc-row-reset:hover:not(:disabled) { color: var(--text); background: var(--bg-hover); }
.sc-row-reset:disabled { opacity: .35; cursor: default; }

.sc-conflict {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--danger-text);
}

.sc-footer {
  flex: none;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px 14px;
  border-top: 1px solid var(--border);
  background: var(--bg-panel);
}
.btn {
  font-size: 13px;
  padding: 6px 14px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border);
}
.btn.ghost { background: var(--bg-input); color: var(--text); }
.btn.ghost:hover { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent-soft); color: #fff; }
.btn.primary:hover:not(:disabled) { background: var(--accent-soft); }
.btn.primary:disabled { opacity: .45; cursor: default; }

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
.sc-binding kbd { background: var(--bg-panel-2); }
</style>
