<script setup>
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BaseButton from "../base/BaseButton.vue";
import TabStrip from "../base/TabStrip.vue";

const activeTab = ref("server");

const name = ref("");
const host = ref("localhost");
const port = ref(27017);
const username = ref("");
const password = ref("");
const authDatabase = ref("");

const manualUri = ref("");

const status = ref(null); // { type: 'success' | 'error', message: string }
const isTesting = ref(false);
const isSaving = ref(false);

const assembledUri = computed(() => {
  let uri = "mongodb://";
  if (username.value && password.value) {
    uri += `${encodeURIComponent(username.value)}:${encodeURIComponent(password.value)}@`;
  }
  uri += `${host.value}:${port.value}`;
  if (authDatabase.value) {
    uri += `/${authDatabase.value}`;
  }
  return uri;
});

function onSwitchToUri() {
  activeTab.value = "uri";
  manualUri.value = assembledUri.value;
}

const activeUri = computed(() =>
  activeTab.value === "server" ? assembledUri.value : manualUri.value
);

async function testConnection() {
  if (!name.value.trim()) {
    status.value = { type: "error", message: "Connection name is required." };
    return;
  }
  status.value = null;
  isTesting.value = true;
  try {
    await invoke("test_connection", { uri: activeUri.value });
    status.value = { type: "success", message: "Connected successfully." };
  } catch (e) {
    status.value = { type: "error", message: String(e) };
  } finally {
    isTesting.value = false;
  }
}

async function saveAndConnect() {
  if (!name.value.trim()) {
    status.value = { type: "error", message: "Connection name is required." };
    return;
  }
  status.value = null;
  isSaving.value = true;
  try {
    const id = await invoke("save_connection", {
      name: name.value.trim(),
      uri: activeUri.value,
    });
    await emit("connection-saved", {
      id,
      name: name.value.trim(),
      uri: activeUri.value,
    });
    await getCurrentWindow().close();
  } catch (e) {
    status.value = { type: "error", message: String(e) };
    isSaving.value = false;
  }
}

async function cancel() {
  await getCurrentWindow().close();
}
</script>

<template>
  <div class="dialog">
    <div class="tabs">
      <TabStrip
        :model-value="activeTab"
        :options="[{ value: 'server', label: 'Server' }, { value: 'uri', label: 'URI' }]"
        @update:model-value="v => v === 'uri' ? onSwitchToUri() : (activeTab = 'server')"
      />
    </div>

    <div class="tab-content">
      <div v-if="activeTab === 'server'" class="fields">
        <div class="field">
          <label>Connection Name</label>
          <input v-model="name" type="text" placeholder="My Connection" />
        </div>
        <div class="field-row">
          <div class="field flex-3">
            <label>Host</label>
            <input v-model="host" type="text" placeholder="localhost" />
          </div>
          <div class="field flex-1">
            <label>Port</label>
            <input v-model.number="port" type="number" placeholder="27017" />
          </div>
        </div>
        <div class="section-label">Authentication (optional)</div>
        <div class="field">
          <label>Username</label>
          <input v-model="username" type="text" placeholder="" />
        </div>
        <div class="field">
          <label>Password</label>
          <input v-model="password" type="password" placeholder="" />
        </div>
        <div class="field">
          <label>Auth Database</label>
          <input v-model="authDatabase" type="text" placeholder="admin" />
        </div>
      </div>

      <div v-if="activeTab === 'uri'" class="fields">
        <div class="field">
          <label>Connection Name</label>
          <input v-model="name" type="text" placeholder="My Connection" />
        </div>
        <div class="field">
          <label>URI</label>
          <input
            v-model="manualUri"
            type="text"
            placeholder="mongodb://localhost:27017"
            class="uri-input"
          />
        </div>
      </div>

      <div v-if="status" class="status" :class="status.type">
        {{ status.message }}
      </div>
    </div>

    <div class="actions">
      <BaseButton bordered @click="cancel">Cancel</BaseButton>
      <div class="actions-right">
        <BaseButton bordered :disabled="isTesting" @click="testConnection">
          {{ isTesting ? "Testing..." : "Test Connection" }}
        </BaseButton>
        <BaseButton variant="primary" :disabled="isSaving" @click="saveAndConnect">
          {{ isSaving ? "Saving..." : "Save & Connect" }}
        </BaseButton>
      </div>
    </div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    sans-serif;
  font-size: 13px;
}

body,
html {
  background-color: var(--bg-titlebar);
  color: var(--text);
  height: 100%;
}

#connect-app {
  height: 100%;
}
</style>

<style scoped>
.dialog {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background-color: var(--bg-titlebar);
  color: var(--text);
}

.tabs {
  display: flex;
  background-color: var(--bg-active);
  border-bottom: 1px solid var(--border-soft);
}



.tab-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.fields {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-row {
  display: flex;
  gap: 10px;
}

.flex-3 {
  flex: 3;
}

.flex-1 {
  flex: 1;
}

.section-label {
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-top: 4px;
}

label {
  font-size: 12px;
  color: var(--text-dim);
}

input {
  background-color: var(--bg-input);
  border: 1px solid var(--border-soft);
  color: var(--text);
  padding: 5px 8px;
  outline: none;
  width: 100%;
}

input:focus {
  border-color: var(--accent);
}

.uri-input {
  font-family: monospace;
}

.status {
  margin-top: 12px;
  padding: 6px 10px;
  font-size: 12px;
  border-radius: 2px;
}

.status.success {
  background-color: var(--success-bg);
  color: var(--success-text);
  border: 1px solid var(--success-border);
}

.status.error {
  background-color: var(--danger-bg);
  color: var(--danger-text);
  border: 1px solid var(--danger);
}

.actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  border-top: 1px solid var(--border-soft);
  background-color: var(--bg-toolbar);
}

.actions-right {
  display: flex;
  gap: 8px;
}

</style>
