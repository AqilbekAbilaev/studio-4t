<script setup>
import { ref } from 'vue';

import { getImageUrl } from '../../utils';
import Database from './database.vue';


const props = defineProps(['connection', 'selectedConnectionId'])

const isCollapsed = ref(true)

</script>
<template>
    <div class="connection-container pointer">
        <div class="connection"
            :style="{ backgroundColor: selectedConnectionId === connection.id ? '#1268da' : 'transparent' }"
            @dblclick="isCollapsed = !isCollapsed" @click="$emit('select-connection', connection.id)">
            <img :src="getImageUrl('collapse.svg')" alt="collapse" width="12px"
                :class="[isCollapsed ? 'collapsed' : 'not-collapsed']" @click.prevent="isCollapsed = !isCollapsed" />
            {{ connection.name }}
        </div>
        <div>
            <Database v-for="database in connection.databases" v-if="!isCollapsed" :database="database" />
        </div>
    </div>
</template>



<style scoped>
.connection {
    padding: 4px;
}

.database {
    display: flex;
    align-items: center;
}


.collapsed {
    transform: rotate(90deg);
}

.not-collapsed {
    transform: rotate(180deg);
}
</style>