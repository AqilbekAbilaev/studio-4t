<script setup>
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getImageUrl } from '../../utils';
import Database from './database.vue';

const props = defineProps(['connection', 'selectedConnectionId'])
const emit = defineEmits(['select-connection'])

const isCollapsed = ref(true)
const databases = ref([])
const isLoading = ref(false)
const hasLoaded = ref(false)
const loadError = ref(null)

async function toggleExpand() {
    isCollapsed.value = !isCollapsed.value

    if (!isCollapsed.value && !hasLoaded.value) {
        isLoading.value = true
        loadError.value = null
        try {
            databases.value = await invoke('list_databases', { uri: props.connection.uri })
            hasLoaded.value = true
        } catch (e) {
            loadError.value = String(e)
            isCollapsed.value = true
        } finally {
            isLoading.value = false
        }
    }
}
</script>
<template>
    <div class="connection-container pointer">
        <div class="connection"
            :style="{ backgroundColor: selectedConnectionId === connection.id ? '#1268da' : 'transparent' }"
            @dblclick="toggleExpand"
            @click="$emit('select-connection', connection.id)">
            <img :src="getImageUrl('collapse.svg')" alt="collapse" width="12px"
                :class="[isCollapsed ? 'collapsed' : 'not-collapsed']"
                @click.stop="toggleExpand" />
            {{ connection.name }}
        </div>

        <div v-if="isLoading" class="loading-state">Loading...</div>
        <div v-if="loadError" class="error-state">{{ loadError }}</div>

        <div v-if="!isCollapsed && !isLoading">
            <Database v-for="database in databases" :key="database.name" :database="database" />
        </div>
    </div>
</template>



<style scoped>
.connection {
    padding: 4px;
    display: flex;
    align-items: center;
    gap: 4px;
}

.collapsed {
    transform: rotate(90deg);
}

.not-collapsed {
    transform: rotate(180deg);
}

.loading-state {
    font-size: 11px;
    color: #888;
    padding: 4px 20px;
}

.error-state {
    font-size: 11px;
    color: #e07070;
    padding: 4px 20px;
    word-break: break-word;
}
</style>
