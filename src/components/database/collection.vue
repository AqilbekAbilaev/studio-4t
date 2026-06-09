<script setup>
import { inject, computed } from 'vue'

const props = defineProps(['collection', 'connectionId', 'connectionUri', 'dbName'])

const activeCollection = inject('activeCollection')

const isActive = computed(() =>
    activeCollection.value?.connectionId === props.connectionId &&
    activeCollection.value?.dbName === props.dbName &&
    activeCollection.value?.collectionName === props.collection.name
)

function select() {
    activeCollection.value = {
        connectionId: props.connectionId,
        uri: props.connectionUri,
        dbName: props.dbName,
        collectionName: props.collection.name,
    }
}
</script>
<template>
    <div
        class="collection pointer"
        :class="{ active: isActive }"
        @click="select"
    >
        {{ collection.name }}
    </div>
</template>

<style scoped>
.collection {
    padding: 3px 4px;
    border-radius: 3px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.collection:hover {
    background-color: #4a4a4a;
}

.collection.active {
    background-color: #1268da;
}
</style>
