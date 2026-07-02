<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { errMessage, errCode } from '../utils/errors'
import BaseIcon from './BaseIcon.vue'
import StateMessage from './StateMessage.vue'

// Top-bar / tree GridFS browser for a database: list buckets, list files, and
// upload / download / delete files. Roadmap P2 item.
const props = defineProps({
  target: { type: Object, required: true },  // { connId, connName, dbName }
})
const emit = defineEmits(['close', 'toast'])

const buckets = ref([])
const selectedBucket = ref('fs')
const files = ref([])
const loading = ref(true)
const busy = ref(false)
const error = ref(null)
const errorCode = ref(null)
const pendingDelete = ref(null)  // file id armed for a confirming second click

// Always offer the default "fs" bucket even if it doesn't exist yet (an upload
// creates it), plus any discovered buckets.
const bucketOptions = computed(() => {
  const set = new Set(buckets.value)
  set.add('fs')
  return [...set].sort()
})

async function loadBuckets() {
  try {
    buckets.value = await invoke('list_gridfs_buckets', { id: props.target.connId, database: props.target.dbName })
    if (buckets.value.length && !buckets.value.includes(selectedBucket.value)) {
      selectedBucket.value = buckets.value[0]
    }
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  }
}

async function loadFiles() {
  loading.value = true
  error.value = null
  pendingDelete.value = null
  try {
    files.value = await invoke('list_gridfs_files', {
      id: props.target.connId,
      database: props.target.dbName,
      bucket: selectedBucket.value,
    })
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
    files.value = []
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await loadBuckets()
  await loadFiles()
})

async function onBucketChange() {
  await loadFiles()
}

async function upload() {
  let path
  try {
    path = await openDialog({ multiple: false })
  } catch (_) { return }
  if (!path) return
  busy.value = true
  try {
    await invoke('gridfs_upload', {
      id: props.target.connId,
      database: props.target.dbName,
      bucket: selectedBucket.value,
      path,
    })
    emit('toast', 'File uploaded')
    await loadBuckets()
    await loadFiles()
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    busy.value = false
  }
}

async function download(file) {
  let dest
  try {
    dest = await saveDialog({ defaultPath: file.filename })
  } catch (_) { return }
  if (!dest) return
  busy.value = true
  try {
    await invoke('gridfs_download', {
      id: props.target.connId,
      database: props.target.dbName,
      bucket: selectedBucket.value,
      fileId: file.id,
      dest,
    })
    emit('toast', `Downloaded ${file.filename}`)
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    busy.value = false
  }
}

async function confirmDelete(file) {
  if (pendingDelete.value !== file.id) {
    pendingDelete.value = file.id
    return
  }
  busy.value = true
  try {
    await invoke('gridfs_delete', {
      id: props.target.connId,
      database: props.target.dbName,
      bucket: selectedBucket.value,
      fileId: file.id,
    })
    emit('toast', `Deleted ${file.filename}`)
    await loadFiles()
  } catch (e) {
    error.value = errMessage(e)
    errorCode.value = errCode(e)
  } finally {
    busy.value = false
    pendingDelete.value = null
  }
}

function fmtBytes(bytes) {
  if (bytes == null) return '—'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let i = 0
  while (value >= 1024 && i < units.length - 1) { value /= 1024; i++ }
  return `${value.toFixed(value >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

function fmtDate(iso) {
  if (!iso) return '—'
  return iso.replace('T', ' ').replace(/\..*$/, '')
}
</script>

<template>
  <div class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog">
      <div class="dlg-title">
        <div class="t">GridFS — {{ target.dbName }}</div>
        <button class="close-btn" @click="$emit('close')">
          <BaseIcon name="close" :size="14" />
        </button>
      </div>

      <div class="gf-body">
        <div class="gf-controls">
          <label class="gf-f">
            Bucket
            <select v-model="selectedBucket" class="gf-select" :disabled="busy" @change="onBucketChange">
              <option v-for="b in bucketOptions" :key="b" :value="b">{{ b }}</option>
            </select>
          </label>
          <button class="gf-upload" :disabled="busy" @click="upload">
            <BaseIcon name="import" :size="13" /> Upload file
          </button>
        </div>

        <StateMessage v-if="loading" mode="loading" label="Loading files…" />
        <StateMessage v-else-if="error" mode="error" :message="error" :code="errorCode" />
        <StateMessage v-else-if="!files.length" mode="empty" label="No files in this bucket" />
        <template v-else>
          <div class="gf-head">
            <span>Filename</span>
            <span>Size</span>
            <span>Uploaded</span>
            <span></span>
          </div>
          <div class="gf-rows">
            <div v-for="f in files" :key="f.id" class="gf-row">
              <span class="gf-name" :title="f.filename">{{ f.filename }}</span>
              <span class="gf-size">{{ fmtBytes(f.length) }}</span>
              <span class="gf-date">{{ fmtDate(f.upload_date) }}</span>
              <span class="gf-actions">
                <button class="gf-act" :disabled="busy" @click="download(f)" title="Download">
                  <BaseIcon name="export" :size="13" />
                </button>
                <button
                  class="gf-act"
                  :class="{ danger: pendingDelete === f.id }"
                  :disabled="busy"
                  @click="confirmDelete(f)"
                  :title="pendingDelete === f.id ? 'Click again to confirm' : 'Delete'"
                >
                  <BaseIcon name="trash" :size="13" />
                </button>
              </span>
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
  width: 680px;
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

.gf-body {
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 220px;
  max-height: 74vh;
  overflow: hidden;
}
.gf-controls { display: flex; align-items: flex-end; gap: 14px; }
.gf-f { font-size: 12px; color: var(--text-dim); display: flex; flex-direction: column; gap: 4px; }
.gf-select {
  background: var(--bg-input);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 4px 8px;
  font-size: 12.5px;
  min-width: 160px;
}
.gf-upload {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12.5px;
  cursor: pointer;
}
.gf-upload:hover { background: var(--accent-soft); }
.gf-upload:disabled { opacity: .6; cursor: default; }

.gf-head, .gf-row {
  display: grid;
  grid-template-columns: 1fr 90px 150px 72px;
  gap: 10px;
  align-items: center;
}
.gf-head {
  padding: 0 6px 6px;
  border-bottom: 1px solid var(--border-soft);
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: .04em;
}
.gf-rows { overflow-y: auto; display: flex; flex-direction: column; }
.gf-row {
  padding: 5px 6px;
  border-bottom: 1px solid var(--grid-line);
  font-size: 12.5px;
}
.gf-row:hover { background: var(--bg-hover); }
.gf-name {
  font-family: var(--mono);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.gf-size, .gf-date { color: var(--text-dim); }
.gf-actions { display: flex; gap: 4px; justify-content: flex-end; }
.gf-act {
  background: none;
  border: 1px solid var(--border-soft);
  color: var(--text-dim);
  border-radius: 5px;
  padding: 3px 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
}
.gf-act:hover { background: var(--bg-hover); color: var(--text); }
.gf-act.danger { color: var(--danger-text); border-color: var(--danger-text); }
.gf-act:disabled { opacity: .5; cursor: default; }
</style>
