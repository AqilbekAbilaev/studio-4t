<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  activeCollectionKey: String, // "connId/dbName/collName"
})
const emit = defineEmits(['select-collection'])

const connections = ref([])
const expandedConns = ref({})      // connId → boolean
const loadingConns = ref({})       // connId → boolean
const connDatabases = ref({})      // connId → DatabaseInfo[]
const connErrors = ref({})         // connId → string
const expandedDbs = ref({})        // "connId/dbName" → boolean
const searchText = ref('')

onMounted(async () => {
  connections.value = await invoke('list_connections')
  await listen('connection-saved', (e) => {
    connections.value.push(e.payload)
  })
})

async function toggleConnection(conn) {
  const id = conn.id
  const wasOpen = expandedConns.value[id]
  expandedConns.value[id] = !wasOpen

  if (!wasOpen && !connDatabases.value[id]) {
    loadingConns.value[id] = true
    connErrors.value[id] = null
    try {
      connDatabases.value[id] = await invoke('list_databases', { id, uri: conn.uri })
    } catch (e) {
      connErrors.value[id] = String(e)
      expandedConns.value[id] = false
    } finally {
      loadingConns.value[id] = false
    }
  }
}

function toggleDatabase(connId, dbName) {
  const key = `${connId}/${dbName}`
  expandedDbs.value[key] = !expandedDbs.value[key]
}

function selectCollection(conn, db, collName) {
  emit('select-collection', {
    connectionId: conn.id,
    connectionName: conn.name,
    uri: conn.uri,
    dbName: db.name,
    collectionName: collName,
  })
}

function collectionKey(connId, dbName, collName) {
  return `${connId}/${dbName}/${collName}`
}

const filteredConnections = ref([])
import { computed } from 'vue'
const filtered = computed(() => {
  if (!searchText.value) return connections.value
  const q = searchText.value.toLowerCase()
  return connections.value.filter(c => c.name.toLowerCase().includes(q))
})
</script>

<template>
  <div class="sidebar">
    <!-- Search row -->
    <div class="side-search">
      <div class="search-box">
        <BaseIcon name="search" :size="14" style="color:var(--text-faint);flex:none" />
        <input v-model="searchText" placeholder="Search open connections (⌘F)" />
      </div>
      <button class="icon-btn sm" title="Font size">
        <BaseIcon name="textType" :size="15" />
      </button>
    </div>

    <!-- Tree -->
    <div class="tree">
      <div v-if="filtered.length === 0" class="tree-empty">
        No connections. Use File → Connect.
      </div>

      <template v-for="conn in filtered" :key="conn.id">
        <!-- Connection root -->
        <div
          class="tnode"
          :class="{ sel: activeCollectionKey?.startsWith(conn.id) }"
          style="padding-left: 6px"
          @click="toggleConnection(conn)"
        >
          <span class="tw">
            <BaseIcon :name="expandedConns[conn.id] ? 'caretDown' : 'caret'" :size="12" />
          </span>
          <span class="ti"><BaseIcon name="connect" :size="15" /></span>
          <span class="tt">{{ conn.name }}</span>
        </div>

        <!-- Loading indicator -->
        <div v-if="loadingConns[conn.id]" class="tnode" style="padding-left: 36px">
          <span class="tt" style="color:var(--text-faint);font-size:11.5px">Loading…</span>
        </div>

        <!-- Error -->
        <div v-if="connErrors[conn.id]" class="tnode" style="padding-left: 36px">
          <span class="tt" style="color:#e07070;font-size:11.5px">{{ connErrors[conn.id] }}</span>
        </div>

        <!-- Databases -->
        <template v-if="expandedConns[conn.id] && connDatabases[conn.id]">
          <template v-for="db in connDatabases[conn.id]" :key="db.name">
            <!-- Database row -->
            <div
              class="tnode"
              style="padding-left: 21px"
              @click="toggleDatabase(conn.id, db.name)"
            >
              <span class="tw">
                <BaseIcon :name="expandedDbs[`${conn.id}/${db.name}`] ? 'caretDown' : 'caret'" :size="12" />
              </span>
              <span class="ti"><BaseIcon name="dbSmall" :size="15" /></span>
              <span class="tt">{{ db.name }}</span>
            </div>

            <!-- Collections -->
            <template v-if="expandedDbs[`${conn.id}/${db.name}`]">
              <div
                v-for="coll in db.collections"
                :key="coll"
                class="tnode"
                :class="{ sel: activeCollectionKey === collectionKey(conn.id, db.name, coll) }"
                style="padding-left: 51px"
                @click="selectCollection(conn, db, coll)"
              >
                <span class="tw empty"><BaseIcon name="caret" :size="12" /></span>
                <span class="ti"><BaseIcon name="collSmall" :size="15" /></span>
                <span class="tt">{{ coll }}</span>
              </div>
            </template>
          </template>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 320px;
  flex: none;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.side-search {
  padding: 8px;
  display: flex;
  gap: 6px;
  align-items: center;
  flex-shrink: 0;
}

.search-box {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 6px 9px;
}

.search-box input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--text);
  font-size: 12.5px;
}

.search-box input::placeholder { color: var(--text-faint); }

.icon-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-dim);
  padding: 5px;
  display: grid;
  place-items: center;
}
.icon-btn:hover { background: var(--bg-hover); color: var(--text); }
.icon-btn.sm { padding: 4px; }

.tree {
  flex: 1;
  overflow-y: auto;
  padding: 2px 0 12px;
}

.tree-empty {
  padding: 16px 12px;
  font-size: 12px;
  color: var(--text-faint);
  text-align: center;
}

.tnode {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px 3px 0;
  font-size: 12.5px;
  cursor: default;
  white-space: nowrap;
  user-select: none;
}

.tnode:hover { background: var(--bg-hover); }
.tnode.sel  { background: var(--bg-active); }

.tw {
  width: 16px;
  display: grid;
  place-items: center;
  color: var(--text-faint);
  flex: none;
}
.tw.empty { visibility: hidden; }

.ti { flex: none; color: var(--text-dim); }
.tt { overflow: hidden; text-overflow: ellipsis; }
</style>
