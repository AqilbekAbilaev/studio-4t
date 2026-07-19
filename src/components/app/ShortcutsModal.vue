<script setup>
import { computed, reactive, ref } from 'vue'
import BaseIcon from '../base/BaseIcon.vue'
import { SHORTCUT_COMMANDS, defaultAccel, accelToTokens, accelFromEvent } from '../../utils/keybindings'
import BaseModal from '../base/BaseModal.vue'
import BaseButton from '../base/BaseButton.vue'
import KeybindButton from '../base/KeybindButton.vue'

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
  <BaseModal title="Keyboard Shortcuts" width="560px" max-width="92vw" @close="$emit('close')">

      <div class="sc-body">
        <!-- Customizable menu shortcuts -->
        <section class="sc-group">
          <div class="sc-group-head">
            <h3 class="sc-group-title">Menu shortcuts</h3>
            <BaseButton variant="ghost" size="sm" @click="resetAll">Reset all</BaseButton>
          </div>

          <div v-for="cmd in SHORTCUT_COMMANDS" :key="cmd.id" class="sc-edit-row">
            <span class="sc-desc">{{ cmd.label }}</span>

            <KeybindButton
              v-if="capturingId !== cmd.id"
              :keys="tokens(working[cmd.id])"
              @click="startCapture(cmd.id)"
            />

            <span
              v-else
              class="sc-binding capturing"
              tabindex="0"
              :ref="(el) => el && el.focus()"
              @keydown="onCaptureKeydown(cmd.id, $event)"
              @blur="cancelCapture"
            >Press a shortcut… <span class="sc-esc">Esc to cancel</span></span>

            <BaseButton
              variant="ghost"
              size="sm"
              :disabled="working[cmd.id] === cmd.default"
              title="Reset to default"
              @click="resetOne(cmd.id)"
            >Reset</BaseButton>
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
        <BaseButton bordered @click="$emit('close')">Close</BaseButton>
        <BaseButton variant="primary" :disabled="!dirty" @click="save">Save changes</BaseButton>
      </div>
    </BaseModal>
</template>

<style scoped>

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
