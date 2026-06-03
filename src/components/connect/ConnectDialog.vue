<script setup>
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

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
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'server' }"
        @click="activeTab = 'server'"
      >
        Server
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'uri' }"
        @click="onSwitchToUri"
      >
        URI
      </button>
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
      <button class="btn btn-secondary" @click="cancel">Cancel</button>
      <div class="actions-right">
        <button class="btn btn-secondary" :disabled="isTesting" @click="testConnection">
          {{ isTesting ? "Testing..." : "Test Connection" }}
        </button>
        <button class="btn btn-primary" :disabled="isSaving" @click="saveAndConnect">
          {{ isSaving ? "Saving..." : "Save & Connect" }}
        </button>
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
  background-color: #2c2c2c;
  color: #e0e0e0;
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
  background-color: #2c2c2c;
  color: #e0e0e0;
}

.tabs {
  display: flex;
  background-color: #3c3c3c;
  border-bottom: 1px solid #555;
}

.tab-btn {
  background: transparent;
  border: none;
  color: #aaa;
  padding: 8px 20px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  font-size: 13px;
}

.tab-btn.active {
  color: #fff;
  border-bottom-color: #3489eb;
}

.tab-btn:hover:not(.active) {
  color: #ccc;
  background-color: #454545;
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
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-top: 4px;
}

label {
  font-size: 12px;
  color: #aaa;
}

input {
  background-color: #1e1e1e;
  border: 1px solid #555;
  color: #e0e0e0;
  padding: 5px 8px;
  outline: none;
  width: 100%;
}

input:focus {
  border-color: #3489eb;
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
  background-color: #1a3a1a;
  color: #6dbf6d;
  border: 1px solid #3a6e3a;
}

.status.error {
  background-color: #3a1a1a;
  color: #e07070;
  border: 1px solid #6e3a3a;
}

.actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  border-top: 1px solid #444;
  background-color: #333;
}

.actions-right {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 5px 14px;
  border: 1px solid #555;
  cursor: pointer;
  font-size: 12px;
  background-color: transparent;
  color: #e0e0e0;
}

.btn:hover:not(:disabled) {
  background-color: #454545;
}

.btn:active:not(:disabled) {
  transform: scale(0.97);
}

.btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.btn-primary {
  background-color: #1a5db5;
  border-color: #3489eb;
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background-color: #2068c9;
}
</style>
