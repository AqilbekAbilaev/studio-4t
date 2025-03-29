<script setup>
import { ref } from 'vue';

import { getImageUrl } from '../../utils';
import Collections from './collections.vue';


const props = defineProps(['database'])

const isCollapsed = ref(true)

</script>
<template>
    <div class="database-container pointer">
        <div class="database" @click.prevent="isCollapsed = !isCollapsed">
            <img :src="getImageUrl('collapse.svg')" alt="collapse" width="12px"
                :class="[isCollapsed ? 'collapsed' : 'not-collapsed']" />
            {{ database.name }}
        </div>
        <div>
            <Collections v-if="!isCollapsed" :collections="database.collections" />
        </div>
    </div>
</template>



<style scoped>
.database-container {
    /* border: 1px solid red; */
    padding: 4px;
    margin-left: 20px;
}

.collapsed {
    transform: rotate(90deg);
}

.not-collapsed {
    transform: rotate(180deg);
}
</style>