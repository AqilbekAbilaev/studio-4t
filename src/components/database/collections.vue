<script setup>
import Collection from './collection.vue';
const props = defineProps(['collections'])


</script>
<template>
    <div class="collections-container">
        <Collection v-for="collection in collections" :collection="collection" />
    </div>
</template>



<style scoped>
.collections-container {
    padding: 4px;
    margin-left: 24px;
}
</style>