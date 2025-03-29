<script setup>
import { getImageUrl } from '../../utils'
import { invoke } from "@tauri-apps/api/core"


const props = defineProps(['options'])

function open() {
    invoke("open_connection")
}

</script>
<template>
    <div class="modal">
        <div class="option-container pointer" v-for="item in options" @click.prevent="open">
            <img v-if="item.icon" :src="getImageUrl(item.icon)" alt="shit" width="24px" />
            {{ item.name }}
        </div>
    </div>
</template>



<style scoped>
.modal {
    max-width: 300px;
    min-width: 200px;
    background-color: #3c3c3c;
    z-index: 1;
}

.option-container {
    display: flex;
    align-items: center;
    /* justify-content: space-between; */
    padding: 4px 10px;
    font-weight: 300;
    color: white;
    position: relative;
}

.option-container:hover {
    background-color: #3489eb;
}

.option-text {}

.option-text:hover {
    background-color: #3489eb;
}
</style>