<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as tauriEmit } from '@tauri-apps/api/event'
import { errMessage } from '../utils/errors'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  editConn: { type: Object, default: null },
})
const emit = defineEmits(['close', 'saved', 'updated'])

const isEditMode = !!props.editConn

// ── step: 'intro' | 'form'  (edit mode always starts on form)
const step     = ref(isEditMode ? 'form' : 'intro')
const mode     = ref('uri')
const pastedUri = ref('')
const uriError  = ref('')

// ── form state — pre-filled from editConn in edit mode
const connName  = ref(isEditMode ? props.editConn.name : 'New Connection')
const activeTab = ref('server')

// server tab
// Seed list — always at least one { host, port } row. In edit mode it comes
// from the stored config (already a `hosts` array after backend migration).
const hosts = ref(
  isEditMode && Array.isArray(props.editConn.hosts) && props.editConn.hosts.length
    ? props.editConn.hosts.map(h => ({ host: h.host, port: h.port }))
    : [{ host: 'localhost', port: 27017 }]
)
const connType       = ref(isEditMode ? props.editConn.connection_type : 'standalone')
const replicaSetName = ref(isEditMode ? (props.editConn.replica_set_name ?? '') : '')

function addHost() { hosts.value.push({ host: '', port: 27017 }) }
function removeHost(index) { if (hosts.value.length > 1) hosts.value.splice(index, 1) }

// auth tab
const authMode  = ref(isEditMode ? (props.editConn.auth_mechanism ?? 'SCRAM-SHA-256') : 'SCRAM-SHA-256')
const username  = ref(isEditMode ? (props.editConn.username ?? '') : '')
const password  = ref('')   // never pre-filled — empty means "keep existing"
const authDb    = ref(isEditMode ? (props.editConn.auth_db ?? 'admin') : 'admin')

const AUTH_MODES = [
  { value: 'none',          label: 'None',                            available: true  },
  { value: 'SCRAM-SHA-256', label: 'Basic (SCRAM-SHA-256)',           available: true  },
  { value: 'SCRAM-SHA-1',   label: 'Legacy (SCRAM-SHA-1)',            available: true  },
  { value: 'PLAIN',         label: 'LDAP (PLAIN)',                    available: true  },
  { value: 'X509',          label: 'X.509',                           available: false },
  { value: 'GSSAPI',        label: 'Kerberos (GSSAPI)',               available: false },
  { value: 'AWS',           label: 'AWS Identity and Access Management (IAM)', available: false },
]

const authModeOpen = ref(false)

const authModeLabel = computed(() =>
  AUTH_MODES.find(m => m.value === authMode.value)?.label ?? authMode.value
)

// ssl tab
const useTls              = ref(isEditMode ? !!props.editConn.tls : false)
const tlsCaFile           = ref(isEditMode ? (props.editConn.tls_ca_file ?? '') : '')
const tlsCertKeyFile      = ref(isEditMode ? (props.editConn.tls_cert_key_file ?? '') : '')
const tlsAllowInvalidCerts = ref(isEditMode ? !!props.editConn.tls_allow_invalid_certificates : false)

async function pickTlsFile(target) {
  try {
    const picked = await openDialog({
      multiple: false,
      filters: [{ name: 'Certificate', extensions: ['pem', 'crt', 'cert', 'cer', 'key'] }],
    })
    if (typeof picked === 'string') {
      if (target === 'ca') tlsCaFile.value = picked
      else tlsCertKeyFile.value = picked
    }
  } catch (_) {}
}

// ssh tab
const useSsh          = ref(isEditMode ? !!props.editConn.ssh_enabled : false)
const sshHost         = ref(isEditMode ? (props.editConn.ssh_host ?? '') : '')
const sshPort         = ref(isEditMode ? (props.editConn.ssh_port ?? 22) : 22)
const sshUser         = ref(isEditMode ? (props.editConn.ssh_user ?? '') : '')
const sshAuth         = ref(isEditMode ? (props.editConn.ssh_auth ?? 'password') : 'password')
const sshPassword     = ref('')   // never pre-filled — empty means "keep existing"
const sshKeyFile      = ref(isEditMode ? (props.editConn.ssh_key_file ?? '') : '')
const sshKeyPassphrase = ref('')  // never pre-filled

async function pickSshKey() {
  try {
    const picked = await openDialog({ multiple: false })
    if (typeof picked === 'string') sshKeyFile.value = picked
  } catch (_) {}
}

// advanced tab
const selectedTag = ref(isEditMode ? (props.editConn.tag ?? 'none') : 'none')

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

// Builds a temporary URI from form fields — used only for Test Connection.
function buildUriForTest() {
  const isSrv = connType.value === 'srv'
  const scheme = isSrv ? 'mongodb+srv' : 'mongodb'
  let uri = `${scheme}://`
  if (username.value && password.value) {
    uri += `${encodeURIComponent(username.value)}:${encodeURIComponent(password.value)}@`
  } else if (username.value) {
    uri += `${encodeURIComponent(username.value)}@`
  }
  uri += isSrv
    ? hosts.value[0].host
    : hosts.value.map(h => `${h.host}:${h.port}`).join(',')
  uri += `/${authDb.value || 'admin'}`
  const params = []
  if (useTls.value) {
    params.push('tls=true')
    if (tlsCaFile.value) params.push(`tlsCAFile=${encodeURIComponent(tlsCaFile.value)}`)
    if (tlsCertKeyFile.value) params.push(`tlsCertificateKeyFile=${encodeURIComponent(tlsCertKeyFile.value)}`)
    if (tlsAllowInvalidCerts.value) params.push('tlsAllowInvalidCertificates=true')
  }
  if (params.length) uri += `?${params.join('&')}`
  return uri
}

// Parses a pasted MongoDB URI into the form fields so the user can review and
// adjust before saving. Hand-rolled rather than relying on the browser's URL
// parser, which throws on a multi-host seed list (`host1,host2,…`) — the
// standard replica-set / cluster format. Returns true if `raw` looked like a
// MongoDB connection string.
function parseUri(raw) {
  const scheme = raw.match(/^mongodb(\+srv)?:\/\//)
  if (!scheme) return false
  const isSrv = !!scheme[1]
  let rest = raw.slice(scheme[0].length)

  // Peel off the query string (everything after the first '?').
  let queryStr = ''
  const qIdx = rest.indexOf('?')
  if (qIdx !== -1) {
    queryStr = rest.slice(qIdx + 1)
    rest = rest.slice(0, qIdx)
  }

  // Peel off the optional /database path (first '/').
  let dbPath = ''
  const slashIdx = rest.indexOf('/')
  if (slashIdx !== -1) {
    dbPath = rest.slice(slashIdx + 1)
    rest = rest.slice(0, slashIdx)
  }

  // Split userinfo from hosts at the LAST '@' so an unescaped '@' inside a
  // password is tolerated — the host portion never contains '@'.
  let userInfo = ''
  let hostsPart = rest
  const atIdx = rest.lastIndexOf('@')
  if (atIdx !== -1) {
    userInfo = rest.slice(0, atIdx)
    hostsPart = rest.slice(atIdx + 1)
  }

  const decode = (s) => { try { return decodeURIComponent(s) } catch (_) { return s } }

  if (userInfo) {
    const cIdx = userInfo.indexOf(':')
    if (cIdx === -1) {
      username.value = decode(userInfo)
      password.value = ''
    } else {
      username.value = decode(userInfo.slice(0, cIdx))
      password.value = decode(userInfo.slice(cIdx + 1))
    }
  }

  // Parse the comma-separated seed list. host:port splits at the last ':' to
  // leave IPv6 brackets alone. SRV uses a single hostname with no port.
  const list = hostsPart.split(',').filter(Boolean)
  if (isSrv) {
    hosts.value = [{ host: list[0] || 'localhost', port: 27017 }]
  } else if (list.length) {
    hosts.value = list.map((hp) => {
      const colonIdx = hp.lastIndexOf(':')
      if (colonIdx === -1 || hp.includes(']')) {
        return { host: hp || 'localhost', port: 27017 }
      }
      return {
        host: hp.slice(0, colonIdx) || 'localhost',
        port: parseInt(hp.slice(colonIdx + 1)) || 27017,
      }
    })
  } else {
    hosts.value = [{ host: 'localhost', port: 27017 }]
  }

  connType.value = isSrv ? 'srv' : 'standalone'
  authDb.value = decode(dbPath) || 'admin'

  const params = new URLSearchParams(queryStr)
  const rs = params.get('replicaSet')
  if (rs) {
    connType.value = 'replica'
    replicaSetName.value = rs
  }
  const authSource = params.get('authSource')
  if (authSource) authDb.value = authSource

  const mech = params.get('authMechanism')
  if (mech) {
    const mechMap = { 'MONGODB-X509': 'X509', 'MONGODB-AWS': 'AWS' }
    authMode.value = mechMap[mech] || mech
  } else if (!username.value) {
    authMode.value = 'none'
  }

  if (params.get('tls') === 'true' || params.get('ssl') === 'true') {
    useTls.value = true
    if (params.get('tlsAllowInvalidCertificates') === 'true') tlsAllowInvalidCerts.value = true
  }

  return true
}

function goNext() {
  if (mode.value === 'uri') {
    const raw = pastedUri.value.trim()
    if (!raw) {
      uriError.value = 'Paste a connection string, or choose "Manually configure" below.'
      return
    }
    if (!parseUri(raw)) {
      uriError.value = 'That doesn’t look like a MongoDB connection string (expected mongodb:// or mongodb+srv://).'
      return
    }
    connName.value = 'Imported from URI'
  }
  uriError.value = ''
  step.value = 'form'
  activeTab.value = 'server'
}

async function testConnection() {
  status.value = null
  isTesting.value = true
  try {
    if (useSsh.value) {
      await invoke('test_ssh_connection', {
        sshHost:       sshHost.value,
        sshPort:       Number(sshPort.value) || 22,
        sshUser:       sshUser.value,
        sshAuth:       sshAuth.value,
        sshPassword:   sshPassword.value || null,
        sshKeyFile:    sshKeyFile.value || null,
        sshPassphrase: sshKeyPassphrase.value || null,
        mongoHost:     hosts.value[0].host,
        mongoPort:     Number(hosts.value[0].port) || 27017,
        username:      authMode.value !== 'none' ? (username.value || null) : null,
        password:      authMode.value !== 'none' ? (password.value || null) : null,
        authDb:        authMode.value !== 'none' ? (authDb.value || null) : null,
        authMechanism: authMode.value,
      })
    } else {
      await invoke('test_connection', { uri: buildUriForTest() })
    }
    status.value = { type: 'success', message: 'Connected successfully.' }
  } catch (e) {
    status.value = { type: 'error', message: errMessage(e) }
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
    const fields = {
      name:            connName.value.trim(),
      hosts:           hosts.value.map(h => ({ host: h.host, port: Number(h.port) || 27017 })),
      connectionType:  connType.value,
      replicaSetName:  replicaSetName.value || null,
      username:        authMode.value !== 'none' ? (username.value || null) : null,
      password:        authMode.value !== 'none' ? (password.value || null) : null,
      authDb:          authMode.value !== 'none' ? (authDb.value || null) : null,
      authMechanism:   authMode.value,
      tls:                          useTls.value,
      tlsCaFile:                    useTls.value ? (tlsCaFile.value || null) : null,
      tlsCertKeyFile:               useTls.value ? (tlsCertKeyFile.value || null) : null,
      tlsAllowInvalidCertificates:  useTls.value ? tlsAllowInvalidCerts.value : false,
      sshEnabled:    useSsh.value,
      sshHost:       useSsh.value ? (sshHost.value || null) : null,
      sshPort:       Number(sshPort.value) || 22,
      sshUser:       useSsh.value ? (sshUser.value || null) : null,
      sshAuth:       useSsh.value ? sshAuth.value : null,
      sshKeyFile:    (useSsh.value && sshAuth.value === 'key') ? (sshKeyFile.value || null) : null,
      sshPassword:   (useSsh.value && sshAuth.value === 'password') ? (sshPassword.value || null) : null,
      sshPassphrase: (useSsh.value && sshAuth.value === 'key') ? (sshKeyPassphrase.value || null) : null,
      tag:             selectedTag.value !== 'none' ? selectedTag.value : null,
    }

    if (isEditMode) {
      await invoke('update_connection', { id: props.editConn.id, ...fields })
      const updated = {
        ...props.editConn,
        name:            fields.name,
        hosts:           fields.hosts,
        connection_type: fields.connectionType,
        replica_set_name: fields.replicaSetName,
        username:        fields.username,
        auth_db:         fields.authDb,
        auth_mechanism:  fields.authMechanism,
        tls:                            fields.tls,
        tls_ca_file:                    fields.tlsCaFile,
        tls_cert_key_file:              fields.tlsCertKeyFile,
        tls_allow_invalid_certificates: fields.tlsAllowInvalidCertificates,
        ssh_enabled:  fields.sshEnabled,
        ssh_host:     fields.sshHost,
        ssh_port:     fields.sshPort,
        ssh_user:     fields.sshUser,
        ssh_auth:     fields.sshAuth,
        ssh_key_file: fields.sshKeyFile,
        tag:             fields.tag,
      }
      emit('updated', updated)
    } else {
      const id = await invoke('save_connection', fields)
      const conn = {
        id:              id,
        name:            fields.name,
        hosts:           fields.hosts,
        connection_type: fields.connectionType,
        tag:             fields.tag,
        last_accessed:   null,
      }
      emit('saved', conn)
      await tauriEmit('connection-saved', conn)
    }
  } catch (e) {
    status.value = { type: 'error', message: errMessage(e) }
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

        <p v-if="uriError" class="nci-error">{{ uriError }}</p>

        <label class="nci-radio" @click="mode = 'manual'; uriError = ''">
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
        <div class="t">{{ isEditMode ? 'Edit Connection' : 'New Connection' }}</div>
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
              <button v-for="[v, l] in [['standalone','Standalone'],['replica','Replica Set'],['sharded','Sharded'],['srv','DNS Seedlist (SRV)']]"
                :key="v" class="seg-b" :class="{ on: connType === v }" @click="connType = v">{{ l }}</button>
            </div>
          </div>
          <div class="nc-field">
            <label>{{ connType === 'srv' ? 'Server (SRV hostname)' : 'Server(s)' }}</label>
            <input v-if="connType === 'srv'" class="nc-input" v-model="hosts[0].host" placeholder="cluster.example.com" />
            <template v-else>
              <div v-for="(h, i) in hosts" :key="i" class="nc-inline nc-host-row">
                <input class="nc-input" v-model="h.host" style="flex:3" placeholder="localhost" />
                <span class="nc-colon">:</span>
                <input class="nc-input" v-model.number="h.port" type="number" style="flex:1" />
                <button v-if="hosts.length > 1" class="nc-host-rm" title="Remove host" @click="removeHost(i)">
                  <BaseIcon name="close" :size="12" />
                </button>
              </div>
              <button class="nc-host-add" @click="addHost">
                <BaseIcon name="plus" :size="12" /> Add host
              </button>
            </template>
          </div>
          <div v-if="connType === 'replica'" class="nc-field">
            <label>Replica set name</label>
            <input class="nc-input" v-model="replicaSetName" placeholder="myReplicaSet" />
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
            <div
              class="nc-dropdown-wrap"
              tabindex="0"
              @blur.capture="authModeOpen = false"
            >
              <div class="nc-select" @mousedown.prevent="authModeOpen = !authModeOpen">
                <span>{{ authModeLabel }}</span>
                <BaseIcon name="caretDown" :size="13" />
              </div>
              <div v-if="authModeOpen" class="nc-dropdown-list">
                <div
                  v-for="m in AUTH_MODES"
                  :key="m.value"
                  class="nc-dropdown-item"
                  :class="{ active: m.value === authMode, dimmed: !m.available }"
                  @mousedown.prevent="m.available && (authMode = m.value, authModeOpen = false)"
                >
                  <span>{{ m.label }}</span>
                  <span v-if="!m.available" class="nc-soon">soon</span>
                </div>
              </div>
            </div>
          </div>

          <template v-if="authMode !== 'none'">
            <div class="nc-field">
              <label>User name</label>
              <input class="nc-input" v-model="username" />
            </div>
            <div class="nc-field">
              <label>Password</label>
              <input
                class="nc-input"
                type="password"
                v-model="password"
                :placeholder="isEditMode ? 'Leave blank to keep existing password' : ''"
              />
            </div>
            <div class="nc-field">
              <label>Authentication DB</label>
              <input class="nc-input" v-model="authDb" :placeholder="authMode === 'PLAIN' ? '$external' : 'admin'" />
            </div>
            <div v-if="authMode === 'PLAIN'" class="nc-hint">
              LDAP (PLAIN) requires SSL/TLS. Enable SSL in the SSL tab.
            </div>
          </template>
        </div>

        <!-- SSH Tunnel -->
        <div v-else-if="activeTab === 'ssh'" class="nc-form">
          <label class="chk-line big" @click="useSsh = !useSsh">
            <span class="cb" :class="{ on: useSsh }"><BaseIcon v-if="useSsh" name="check" :size="12" /></span>
            Use SSH tunnel
          </label>

          <template v-if="useSsh">
            <div class="nc-inline2">
              <div class="nc-field" style="flex:1">
                <label>SSH host</label>
                <input class="nc-input" v-model="sshHost" placeholder="bastion.example.com" spellcheck="false" />
              </div>
              <div class="nc-field" style="width:92px">
                <label>Port</label>
                <input class="nc-input" type="number" v-model.number="sshPort" />
              </div>
            </div>
            <div class="nc-field">
              <label>SSH user</label>
              <input class="nc-input" v-model="sshUser" spellcheck="false" />
            </div>
            <div class="nc-field">
              <label>Authentication</label>
              <div class="seg">
                <button type="button" class="seg-b" :class="{ on: sshAuth === 'password' }" @click="sshAuth = 'password'">Password</button>
                <button type="button" class="seg-b" :class="{ on: sshAuth === 'key' }" @click="sshAuth = 'key'">Private key</button>
              </div>
            </div>

            <div v-if="sshAuth === 'password'" class="nc-field">
              <label>SSH password</label>
              <input class="nc-input" type="password" v-model="sshPassword" :placeholder="isEditMode ? 'Leave blank to keep existing' : ''" />
            </div>
            <template v-else>
              <div class="nc-field">
                <label>Private key file</label>
                <div class="nc-file-row">
                  <input class="nc-input" v-model="sshKeyFile" placeholder="~/.ssh/id_ed25519" spellcheck="false" />
                  <button type="button" class="nc-browse" @click="pickSshKey">Browse…</button>
                </div>
              </div>
              <div class="nc-field">
                <label>Key passphrase (optional)</label>
                <input class="nc-input" type="password" v-model="sshKeyPassphrase" :placeholder="isEditMode ? 'Leave blank to keep existing' : ''" />
              </div>
            </template>

            <div class="nc-hint">The MongoDB host/port (Server tab) are resolved from the SSH host. Standalone connections only — replica set / SRV over SSH aren't supported yet.</div>
          </template>
        </div>

        <!-- SSL -->
        <div v-else-if="activeTab === 'ssl'" class="nc-form">
          <label class="chk-line big" @click="useTls = !useTls">
            <span class="cb" :class="{ on: useTls }"><BaseIcon v-if="useTls" name="check" :size="12" /></span>
            Use SSL/TLS protocol to connect
          </label>

          <template v-if="useTls">
            <div class="nc-field">
              <label>Certificate Authority (.pem)</label>
              <div class="nc-file-row">
                <input class="nc-input" v-model="tlsCaFile" placeholder="Path to CA certificate" spellcheck="false" />
                <button type="button" class="nc-browse" @click="pickTlsFile('ca')">Browse…</button>
              </div>
            </div>

            <div class="nc-field">
              <label>Client Certificate + Key (.pem)</label>
              <div class="nc-file-row">
                <input class="nc-input" v-model="tlsCertKeyFile" placeholder="Path to client certificate (optional)" spellcheck="false" />
                <button type="button" class="nc-browse" @click="pickTlsFile('cert')">Browse…</button>
              </div>
            </div>

            <label class="chk-line" @click="tlsAllowInvalidCerts = !tlsAllowInvalidCerts">
              <span class="cb" :class="{ on: tlsAllowInvalidCerts }"><BaseIcon v-if="tlsAllowInvalidCerts" name="check" :size="12" /></span>
              Allow invalid certificates (accept self-signed / expired)
            </label>
            <div class="nc-hint">A Certificate Authority file verifies the server securely; “allow invalid certificates” skips that check.</div>
          </template>
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
          {{ isSaving ? 'Saving…' : (isEditMode ? 'Save Changes' : 'Save') }}
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
.nci-error { margin: 12px 0 0; font-size: 12.5px; line-height: 1.5; color: #e07070; }

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
.nc-file-row { display: flex; gap: 8px; align-items: center; }
.nc-browse {
  flex: none; padding: 8px 12px; border-radius: 6px;
  border: 1px solid var(--border-soft); background: var(--bg-toolbar);
  color: var(--text); font-size: 12.5px; cursor: pointer; white-space: nowrap;
}
.nc-browse:hover { background: var(--bg-hover); }
.nc-inline  { display: flex; align-items: center; gap: 8px; }
.nc-inline2 { display: flex; gap: 14px; }
.nc-inline2 .nc-field { flex: 1; }
.nc-colon { color: var(--text-faint); }
.nc-host-row { margin-bottom: 8px; }
.nc-host-rm {
  display: flex; align-items: center; justify-content: center;
  width: 28px; height: 28px; flex: none;
  background: none; border: 1px solid var(--border-soft); border-radius: 6px;
  color: var(--text-dim);
}
.nc-host-rm:hover { background: var(--bg-hover); color: var(--text); }
.nc-host-add {
  display: inline-flex; align-items: center; gap: 5px;
  background: none; border: none; padding: 2px 0;
  color: var(--accent); font-size: 12.5px; cursor: pointer;
}
.nc-host-add:hover { text-decoration: underline; }
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

/* custom dropdown */
.nc-dropdown-wrap {
  position: relative;
  outline: none;
}
.nc-dropdown-wrap:focus-within .nc-select {
  border-color: var(--accent);
}
.nc-dropdown-list {
  position: absolute;
  top: calc(100% + 4px);
  left: 0; right: 0;
  background: var(--bg-panel-2);
  border: 1px solid var(--border-soft);
  border-radius: 7px;
  box-shadow: 0 8px 24px rgba(0,0,0,.4);
  z-index: 100;
  overflow: hidden;
}
.nc-dropdown-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
  user-select: none;
}
.nc-dropdown-item:hover:not(.dimmed) { background: var(--bg-hover); }
.nc-dropdown-item.active { color: var(--accent); }
.nc-dropdown-item.dimmed { color: var(--text-faint); cursor: default; }
.nc-soon {
  font-size: 10.5px;
  color: var(--text-faint);
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 4px;
  padding: 1px 6px;
}
</style>
