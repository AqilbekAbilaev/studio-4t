<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import BaseIcon from './BaseIcon.vue'

const emit = defineEmits(['close', 'saved'])

// ── step: 'intro' | 'form'
const step     = ref('intro')
const mode     = ref('uri')   // 'uri' | 'manual'
const pastedUri = ref('')

// ── form state
const connName  = ref('New Connection')
const activeTab = ref('server')

// server tab
const host      = ref('localhost')
const port      = ref(27017)
const connType  = ref('standalone')

// auth tab
const username  = ref('')
const password  = ref('')
const authDb    = ref('admin')

// advanced tab
const selectedTag = ref('none')

// status
const status    = ref(null)
const isTesting = ref(false)
const isSaving  = ref(false)

const TAG_COLORS = {
  none:   null,
  blue:   '#3b82f6',
  green:  '#4caf78',
  purple: '#b07ddb',
  red:    '#e07a6b',
}

const TABS = [
  ['server', 'Server'],
  ['auth', 'Authentication'],
  ['ssh', 'SSH Tunnel'],
  ['ssl', 'SSL'],
  ['advanced', 'Advanced'],
]

function buildUri() {
  if (mode.value === 'uri' && step.value === 'intro') return pastedUri.value.trim()
  let uri = 'mongodb://'
  if (username.value && password.value) {
    uri += `${encodeURIComponent(username.value)}:${encodeURIComponent(password.value)}@`
  }
  uri += `${host.value}:${port.value}`
  if (authDb.value) uri += `/${authDb.value}`
  return uri
}

function goNext() {
  if (mode.value === 'uri' && pastedUri.value.trim()) {
    connName.value = 'Imported from URI'
  }
  step.value = 'form'
  activeTab.value = 'server'
}

async function testConnection() {
  status.value = null
  isTesting.value = true
  try {
    await invoke('test_connection', { uri: buildUri() })
    status.value = { type: 'success', message: 'Connected successfully.' }
  } catch (e) {
    status.value = { type: 'error', message: String(e) }
  } finally {
    isTesting.value = false
  }
}

async function save() {
  if (!connName.value.trim()) {
    status.value = { type: 'error', message: 'Connection name is required.' }
    return
  }
  status.value = null
  isSaving.value = true
  try {
    const uri = buildUri()
    const id  = await invoke('save_connection', { name: connName.value.trim(), uri })
    if (selectedTag.value !== 'none') {
      await invoke('set_connection_tag', { id, tag: selectedTag.value })
    }
    emit('saved', { id, name: connName.value.trim(), uri, tag: selectedTag.value !== 'none' ? selectedTag.value : null, last_accessed: null })
  } catch (e) {
    status.value = { type: 'error', message: String(e) }
    isSaving.value = false
  }
}
</script>

<template>
  <!-- ── Intro step ─────────────────────────────────── -->
  <div v-if="step === 'intro'" class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog nc-intro">
      <div class="dlg-title">
        <div class="traffic">
          <span class="light r" @click="$emit('close')"></span>
          <span class="light y"></span>
          <span class="light g"></span>
        </div>
        <div class="t">New Connection</div>
      </div>

      <div class="nci-body">
        <p class="nci-lead">
          If you have a connection string (SRV or standard), e.g. for your MongoDB deployment,
          you can paste it here and Studio-4T will auto-configure your connection settings for you.
        </p>

        <label class="nci-radio" @click="mode = 'uri'">
          <span class="radio" :class="{ on: mode === 'uri' }"></span>
          <span class="nci-radio-lbl">Paste your connection string (SRV or standard) here:</span>
        </label>
        <div class="nci-uri-wrap">
          <span class="nci-uri-lbl">URI:</span>
          <textarea
            class="nci-uri"
            :disabled="mode !== 'uri'"
            v-model="pastedUri"
            placeholder="mongodb+srv://user:password@cluster.mongodb.net/"
          />
        </div>

        <label class="nci-radio" @click="mode = 'manual'">
          <span class="radio" :class="{ on: mode === 'manual' }"></span>
          <span class="nci-radio-lbl">Manually configure my connection settings</span>
        </label>
      </div>

      <div class="cm-footer">
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" @click="goNext">Next</button>
      </div>
    </div>
  </div>

  <!-- ── Form step ──────────────────────────────────── -->
  <div v-else class="overlay" @mousedown.self="$emit('close')">
    <div class="dialog nc">
      <div class="dlg-title">
        <div class="traffic">
          <span class="light r" @click="$emit('close')"></span>
          <span class="light y"></span>
          <span class="light g"></span>
        </div>
        <div class="t">New Connection</div>
      </div>

      <!-- Name row -->
      <div class="nc-top">
        <label class="nc-namelbl">Connection name</label>
        <input class="nc-name" v-model="connName" />
        <button class="nc-uri-btn" @click="step = 'intro'">
          <BaseIcon name="uri" :size="15" /> From URI
        </button>
      </div>

      <!-- Tabs -->
      <div class="nc-tabs">
        <button
          v-for="[k, l] in TABS"
          :key="k"
          class="nc-tab"
          :class="{ active: activeTab === k }"
          @click="activeTab = k"
        >{{ l }}</button>
      </div>

      <!-- Tab body -->
      <div class="nc-body">

        <!-- Server -->
        <div v-if="activeTab === 'server'" class="nc-form">
          <div class="nc-field">
            <label>Connection type</label>
            <div class="seg">
              <button v-for="[v, l] in [['standalone','Standalone'],['replica','Replica Set'],['sharded','Sharded'],['dns','DNS Seedlist (SRV)']]"
                :key="v" class="seg-b" :class="{ on: connType === v }" @click="connType = v">{{ l }}</button>
            </div>
          </div>
          <div class="nc-field">
            <label>Server</label>
            <div class="nc-inline">
              <input class="nc-input" v-model="host" style="flex:3" />
              <span class="nc-colon">:</span>
              <input class="nc-input" v-model.number="port" type="number" style="flex:1" />
            </div>
          </div>
          <div v-if="connType === 'replica'" class="nc-field">
            <label>Replica set name</label>
            <input class="nc-input" placeholder="myReplicaSet" />
          </div>
          <div class="nc-field">
            <label>Read preference</label>
            <div class="nc-select"><span>Primary</span><BaseIcon name="caretDown" :size="13" /></div>
          </div>
          <div class="nc-hint">
            Studio-4T currently targets MongoDB.
            PostgreSQL &amp; MySQL engines arrive in a future release.
          </div>
        </div>

        <!-- Authentication -->
        <div v-else-if="activeTab === 'auth'" class="nc-form">
          <div class="nc-field">
            <label>Authentication mode</label>
            <div class="nc-select"><span>SCRAM-SHA-256</span><BaseIcon name="caretDown" :size="13" /></div>
          </div>
          <div class="nc-field">
            <label>User name</label>
            <input class="nc-input" v-model="username" />
          </div>
          <div class="nc-field">
            <label>Password</label>
            <input class="nc-input" type="password" v-model="password" />
          </div>
          <div class="nc-field">
            <label>Authentication DB</label>
            <input class="nc-input" v-model="authDb" />
          </div>
          <label class="chk-line big">
            <span class="cb"></span>
            Store password (encrypted in OS keychain)
          </label>
        </div>

        <!-- SSH Tunnel -->
        <div v-else-if="activeTab === 'ssh'" class="nc-form">
          <label class="chk-line big"><span class="cb"></span> Use SSH tunnel</label>
          <div class="nc-hint" style="margin-top:12px">SSH tunnel configuration — coming soon.</div>
        </div>

        <!-- SSL -->
        <div v-else-if="activeTab === 'ssl'" class="nc-form">
          <label class="chk-line big"><span class="cb"></span> Use SSL/TLS protocol to connect</label>
          <div class="nc-hint" style="margin-top:12px">SSL/TLS configuration — coming soon.</div>
        </div>

        <!-- Advanced -->
        <div v-else-if="activeTab === 'advanced'" class="nc-form">
          <div class="nc-inline2">
            <div class="nc-field">
              <label>Connect timeout (ms)</label>
              <input class="nc-input" value="20000" />
            </div>
            <div class="nc-field">
              <label>Socket timeout (ms)</label>
              <input class="nc-input" value="0" />
            </div>
          </div>
          <div class="nc-field">
            <label>App name</label>
            <input class="nc-input" value="Studio-4T" />
          </div>
          <div class="nc-field">
            <label>Color tag</label>
            <div class="tag-row">
              <span
                v-for="t in ['none','blue','green','purple','red']"
                :key="t"
                class="tag-swatch"
                :class="{ on: selectedTag === t }"
                :style="TAG_COLORS[t]
                  ? { background: TAG_COLORS[t] }
                  : { background: 'transparent', border: '1px solid var(--border-soft)' }"
                @click="selectedTag = t"
              ></span>
            </div>
          </div>
        </div>

      </div>

      <!-- Status -->
      <div v-if="status" class="nc-status" :class="status.type">{{ status.message }}</div>

      <!-- Footer -->
      <div class="cm-footer">
        <button class="btn" :disabled="isTesting" @click="testConnection">
          <BaseIcon name="connect" :size="15" />
          {{ isTesting ? 'Testing…' : 'Test Connection' }}
        </button>
        <span class="spacer"></span>
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn primary" :disabled="isSaving" @click="save">
          {{ isSaving ? 'Saving…' : 'Save' }}
        </button>
      </div>

    </div>
  </div>
</template>

<style scoped>
/* shared overlay/dialog/title/traffic */
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.5);
  display: grid;
  place-items: center;
  z-index: 60;
}
.dialog {
  background: var(--bg-window);
  border-radius: 10px;
  box-shadow: 0 30px 80px rgba(0,0,0,.65), 0 0 0 1px #000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.nc-intro { width: 640px; }
.nc      { width: 720px; height: 600px; }

.dlg-title {
  height: 36px; flex: none;
  background: linear-gradient(#34363a, #2c2e31);
  border-bottom: 1px solid var(--border);
  display: flex; align-items: center;
  padding: 0 14px; position: relative;
}
.dlg-title .t {
  position: absolute; left: 0; right: 0; text-align: center;
  font-size: 13px; color: var(--text-dim); font-weight: 500;
  pointer-events: none;
}
.traffic { display: flex; gap: 8px; }
.light   { width: 12px; height: 12px; border-radius: 50%; cursor: pointer; }
.light.r { background: #ec6a5e; }
.light.y { background: #f4bf4f; }
.light.g { background: #61c554; }

/* ── Intro body ── */
.nci-body { padding: 22px 24px 8px; }
.nci-lead { font-size: 13.5px; line-height: 1.6; color: var(--text); margin: 0 0 22px; }
.nci-radio {
  display: flex; align-items: center; gap: 11px;
  cursor: pointer; margin: 0 0 4px;
}
.nci-radio-lbl { font-size: 14px; color: var(--text); font-weight: 600; }
.radio {
  width: 17px; height: 17px; border-radius: 50%;
  border: 1.5px solid var(--text-faint); flex: none;
  display: grid; place-items: center;
}
.radio.on { border-color: var(--accent); }
.radio.on::after {
  content: ""; width: 9px; height: 9px;
  border-radius: 50%; background: var(--accent);
}
.nci-uri-wrap {
  display: flex; gap: 12px; margin: 14px 0 22px; padding-left: 28px;
}
.nci-uri-lbl { font-size: 13px; color: var(--text-dim); padding-top: 8px; flex: none; }
.nci-uri {
  flex: 1; min-height: 90px; resize: vertical;
  background: var(--bg-input); border: 1px solid var(--border-soft);
  border-radius: 6px; padding: 9px 11px;
  color: var(--text); font-family: var(--mono); font-size: 13px;
  outline: none;
}
.nci-uri:focus   { border-color: var(--accent); }
.nci-uri:disabled { opacity: .5; }

/* ── Form top ── */
.nc-top {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 18px 10px; flex: none;
}
.nc-namelbl { font-size: 12.5px; color: var(--text-dim); flex: none; }
.nc-name {
  flex: 1; background: var(--bg-input);
  border: 1px solid var(--border-soft); border-radius: 6px;
  padding: 8px 11px; color: var(--text); font-size: 13px; outline: none;
}
.nc-name:focus { border-color: var(--accent); }
.nc-uri-btn {
  display: flex; align-items: center; gap: 6px;
  background: var(--bg-toolbar); border: 1px solid var(--border-soft);
  border-radius: 6px; padding: 8px 12px; color: var(--text); font-size: 12.5px;
  white-space: nowrap;
}
.nc-uri-btn:hover { background: var(--bg-hover); }

/* ── Tabs ── */
.nc-tabs {
  display: flex; gap: 2px; padding: 0 18px;
  border-bottom: 1px solid var(--border); flex: none;
}
.nc-tab {
  padding: 9px 14px; font-size: 12.5px;
  color: var(--text-dim); background: none;
  border: none; border-bottom: 2px solid transparent;
}
.nc-tab.active { color: var(--text); border-bottom-color: var(--accent); }
.nc-tab:hover  { color: var(--text); }

/* ── Tab body ── */
.nc-body { flex: 1; overflow-y: auto; padding: 18px; }
.nc-form { display: flex; flex-direction: column; gap: 15px; max-width: 560px; }
.nc-field { display: flex; flex-direction: column; gap: 6px; }
.nc-field > label { font-size: 12px; color: var(--text-dim); }
.nc-input {
  background: var(--bg-input); border: 1px solid var(--border-soft);
  border-radius: 6px; padding: 8px 11px;
  color: var(--text); font-size: 13px; outline: none; width: 100%;
}
.nc-input:focus { border-color: var(--accent); }
.nc-inline  { display: flex; align-items: center; gap: 8px; }
.nc-inline2 { display: flex; gap: 14px; }
.nc-inline2 .nc-field { flex: 1; }
.nc-colon { color: var(--text-faint); }
.nc-select {
  display: flex; align-items: center; justify-content: space-between;
  background: var(--bg-input); border: 1px solid var(--border-soft);
  border-radius: 6px; padding: 8px 11px; color: var(--text); font-size: 13px;
  cursor: pointer;
}
.nc-hint {
  font-size: 12px; color: var(--text-faint); line-height: 1.5;
  background: var(--bg-panel-2); border: 1px solid var(--border-soft);
  border-radius: 7px; padding: 11px 13px;
}

/* segmented control */
.seg {
  display: flex; background: var(--bg-input);
  border: 1px solid var(--border-soft); border-radius: 7px;
  padding: 2px; gap: 2px; width: fit-content;
}
.seg-b {
  padding: 6px 13px; border-radius: 5px; background: none;
  border: none; color: var(--text-dim); font-size: 12.5px;
}
.seg-b.on { background: var(--accent); color: #fff; }
.seg-b:hover:not(.on) { color: var(--text); }

/* checkbox */
.chk-line {
  display: flex; align-items: center; gap: 8px;
  font-size: 12.5px; color: var(--text-dim); cursor: pointer;
}
.chk-line.big { font-size: 13px; color: var(--text); }
.cb {
  width: 17px; height: 17px; border-radius: 4px;
  border: 1px solid var(--border-soft); background: var(--bg-input);
  display: grid; place-items: center; flex: none;
}
.cb.on { background: var(--accent); border-color: var(--accent); color: #fff; }

/* color tag swatches */
.tag-row  { display: flex; gap: 8px; }
.tag-swatch {
  width: 22px; height: 22px; border-radius: 5px; cursor: pointer;
}
.tag-swatch.on { outline: 2px solid var(--accent); outline-offset: 2px; }

/* status */
.nc-status {
  margin: 0 18px 4px;
  padding: 6px 10px; font-size: 12px; border-radius: 5px;
}
.nc-status.success { background: #1a3a1a; color: #6dbf6d; border: 1px solid #3a6e3a; }
.nc-status.error   { background: #3a1a1a; color: #e07070; border: 1px solid #6e3a3a; }

/* footer */
.cm-footer {
  display: flex; align-items: center; gap: 12px;
  padding: 12px 16px; border-top: 1px solid var(--border); flex: none;
}
.spacer { flex: 1; }
.btn {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 20px; border-radius: 7px; font-size: 13px;
  border: 1px solid var(--border-soft); background: var(--bg-toolbar); color: var(--text);
}
.btn:hover:not(:disabled) { background: var(--bg-hover); }
.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn.primary:hover:not(:disabled) { background: var(--accent-soft); }
.btn:disabled { opacity: .4; cursor: default; }
</style>
