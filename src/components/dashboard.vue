<script setup>
import Tab from "./tab/tab.vue"

const props = defineProps(['options'])


</script>
<template>
    <div class="container">
        <div class="dashboard">
            <Tab />

        </div>
    </div>
</template>



<style scoped>

.container {
    width: 25%;
    background-color: #3c3c3c;
    padding: 4px;
    height: 100%;
    
}

.dashboard {
    /* margin-left: 8px; */
    height: 100%;
    border: 1px solid #575757;
}
</style>