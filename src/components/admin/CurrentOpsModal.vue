<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { errText, errCode } from '../../utils/errors'
import BaseIcon from '../base/BaseIcon.vue'
import RawToggle from '../base/RawToggle.vue'
import StateMessage from '../base/StateMessage.vue'
import BaseModalBody from '../base/BaseModalBody.vue'
import BaseModal from '../base/BaseModal.vue'

// Opened from App.vue for a connection node. Fetches admin `currentOp` once and lists
// the operations currently in progress; the full document is available raw below.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName }
})
defineEmits(['close'])

const loading = ref(true)
const error = ref(null)
const errorCode = ref(null)
const result = ref(null)
const showRaw = ref(false)

onMounted(async () => {
  try {
    result.value = await invoke('current_ops', { id: props.target.connId })
  } catch (e) {
    error.value = errText(e)
    errorCode.value = errCode(e)
  } finally {
    loading.value = false
  }
})

// The in-progress operations, normalized to the columns we show.
const ops = computed(() => {
  const inprog = result.value && Array.isArray(result.value.inprog) ? result.value.inprog : []
  return inprog.map(op => ({
    opid: op.opid ?? '—',
    op: op.op || '—',
    ns: op.ns || '—',
    secs: op.secs_running != null ? `${op.secs_running}s` : '—',
    desc: op.desc || op.client || '—',
  }))
})

const rawJson = computed(() =>
  result.value ? JSON.stringify(result.value, null, 2) : ''
)
</script>

<template>
  <BaseModal :title="`Current Operations — ${target.connName}`" width="720px" max-width="92vw" @close="$emit('close')">

      <BaseModalBody>
        <StateMessage v-if="loading" mode="loading" label="Fetching current operations…" />
        <StateMessage
          v-else-if="error"
          mode="error"
          :message="error"
          :code="errorCode"
        />
        <template v-else>
          <StateMessage
            v-if="!ops.length"
            mode="empty"
            label="No operations currently in progress"
          />
          <table v-else class="ops-table">
            <thead>
              <tr>
                <th>Op ID</th>
                <th>Type</th>
                <th>Namespace</th>
                <th>Running</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(op, i) in ops" :key="i">
                <td>{{ op.opid }}</td>
                <td>{{ op.op }}</td>
                <td>{{ op.ns }}</td>
                <td>{{ op.secs }}</td>
                <td>{{ op.desc }}</td>
              </tr>
            </tbody>
          </table>

          <RawToggle v-model="showRaw" label="Raw currentOp" />
          <pre v-if="showRaw" class="ss-raw">{{ rawJson }}</pre>
        </template>
      </BaseModalBody>
    </BaseModal>
</template>

<style scoped>



.ops-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}
.ops-table th {
  text-align: left;
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
}
.ops-table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text);
  user-select: text;
  word-break: break-word;
}

.ss-raw {
  margin: 0;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  color: var(--text-dim);
  white-space: pre;
  overflow-x: auto;
  user-select: text;
}
</style>
