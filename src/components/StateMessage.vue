<script setup>
import { computed } from 'vue'
import BaseIcon from './BaseIcon.vue'

// A shared loading / empty / error placeholder for result and tree surfaces.
// For errors, an optional `code` (from errCode) drives an actionable hint + icon.
const props = defineProps({
  mode:      { type: String,  required: true },  // 'loading' | 'empty' | 'error'
  message:   { type: String,  default: '' },     // error text
  code:      { type: String,  default: null },    // error category from errCode()
  label:     { type: String,  default: '' },      // override loading/empty title
  retryable: { type: Boolean, default: false },
})
const emit = defineEmits(['retry'])

const HINTS = {
  auth:        'Check the username and password.',
  network:     'Check the host and port, or that the server is running.',
  unreachable: 'Check the host and port, or that the server is running.',
  tls:         'Check the TLS / SSL settings for this connection.',
  ssh:         'Check the SSH tunnel settings.',
  keychain:    'Could not reach the system keychain.',
}

const hint = computed(() => (props.code ? (HINTS[props.code] || '') : ''))

const title = computed(() => {
  if (props.mode === 'loading') return props.label || 'Loading…'
  if (props.mode === 'empty') return props.label || 'No documents found'
  return props.message || 'Something went wrong'
})

const icon = computed(() => {
  if (props.mode === 'empty') return 'collection'
  if (props.code === 'auth' || props.code === 'keychain') return 'lock'
  if (props.code === 'network' || props.code === 'unreachable') return 'connect'
  if (props.code === 'ssh') return 'anchor'
  return 'close'
})
</script>

<template>
  <div class="state" :class="mode">
    <div v-if="mode === 'loading'" class="spinner"></div>
    <BaseIcon v-else :name="icon" :size="30" class="state-ic" />
    <div class="state-title">{{ title }}</div>
    <div v-if="hint" class="state-hint">{{ hint }}</div>
    <button v-if="mode === 'error' && retryable" class="state-retry" @click="emit('retry')">
      <BaseIcon name="refresh" :size="13" /> Retry
    </button>
  </div>
</template>

<style scoped>
.state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 40px 24px;
  text-align: center;
}
.state-ic { color: var(--text-faint); }
.state.error .state-ic { color: #e05555; }
.state-title {
  font-size: 13px;
  color: var(--text-dim);
  max-width: 520px;
  word-break: break-word;
}
.state.error .state-title { color: #e8857d; }
.state-hint { font-size: 12px; color: var(--text-faint); max-width: 520px; }
.state-retry {
  margin-top: 4px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 14px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--bg-toolbar);
  color: var(--text);
  font-size: 12.5px;
  cursor: pointer;
}
.state-retry:hover { background: var(--bg-hover); }

.spinner {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 2.5px solid var(--border);
  border-top-color: var(--accent);
  animation: state-spin 0.7s linear infinite;
}
@keyframes state-spin {
  to { transform: rotate(360deg); }
}
</style>
