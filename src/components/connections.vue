<script setup>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Connection from "./database/connection.vue";

const databaseConnections = ref([]);
const selectedConnectionId = ref(null);

onMounted(async () => {
    try {
        databaseConnections.value = await invoke("list_connections");
    } catch (e) {
        console.error("Failed to load connections:", e);
    }

    await listen("connection-saved", (event) => {
        databaseConnections.value.push(event.payload);
    });
});
</script>
<template>
    <div class="container">
        <div class="connections">
            <Connection
                v-for="connection in databaseConnections"
                :key="connection.id"
                :connection="connection"
                :selectedConnectionId="selectedConnectionId"
                @select-connection="e => selectedConnectionId = e"
            />
            <div v-if="databaseConnections.length === 0" class="empty-state">
                No connections. Use File → Connect to add one.
            </div>
        </div>
    </div>
</template>



<style scoped>
.container {
    width: 30%;
    background-color: #3c3c3c;
    height: 100%;
    padding: 4px;
}

.connections {
    height: 100%;
    border: 1px solid #575757;
    padding: 6px;
}

.empty-state {
    color: #777;
    font-size: 12px;
    padding: 12px 4px;
    text-align: center;
}
</style>
