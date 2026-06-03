<script setup>
import Collection from './collection.vue';
const props = defineProps(['collections'])


</script>
<template>
    <div class="collections-container">
        <Collection v-for="name in collections" :key="name" :collection="{ name }" />
    </div>
</template>



<style scoped>
.collections-container {
    padding: 4px;
    margin-left: 24px;
}
</style>