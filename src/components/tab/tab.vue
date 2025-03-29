<script setup>
import { ref } from "vue";
import { getImageUrl } from "../../utils"

const props = defineProps(['options'])

const query = ref('')
</script>
<template>
    <div class="tab">
        <div class="operations">
            <button class="run-btn pointer">
                <img :src="getImageUrl('triangle.svg')" width="16px" />
                <p class="btn-text">Run</p>
            </button>
            <button class="run-btn load-query-btn pointer">
                <img :src="getImageUrl('folder.svg')" width="16px" />
                <p class="btn-text">Load query</p>
            </button>

            <button class="run-btn save-query-btn pointer">
                <img :src="getImageUrl('save.svg')" width="16px" />
                <p class="btn-text">Save query</p>
            </button>


            <button class="run-btn visual-query-builder pointer">
                <img :src="getImageUrl('visual-query-builder.svg')" width="16px" />
                <p class="btn-text">Visual Query Builder</p>
            </button>
        </div>
        <div class="operations">
            <div class="query-container">
                <label for="john">Query</label>
                <input v-model="query" class="query" id="john" />
                <button class="run-btn pointer clear-btn">
                    <img :src="getImageUrl('broom.png')" width="18px" />
                </button>
                <button class="run-btn pointer clear-btn">
                    <img :src="getImageUrl('fullscreen.svg')" width="16px" />
                </button>
            </div>
            <!-- <div class="query-container">
                <label for="john">Query</label>
                <input v-model="query" class="query" id="john" />
            </div> -->
        </div>
    </div>
</template>



<style scoped>
.operations {
    border-bottom: 0.4px solid rgba(148, 148, 148, 0.253);
    padding: 10px;
    display: flex;
    column-gap: 10px;
}

.run-btn {
    background-color: transparent;
    border: 0;
    display: flex;
    align-items: center;
    column-gap: 10px;
    border: 1px solid rgb(197, 197, 197);
    color: white;
    padding: 4px 6px;
}

.run-btn:active {
    transform: scale(0.975);
    background: rgb(41, 41, 41);
}

.visual-query-builder {
    margin-left: auto;
}

input.query {
    flex-grow: 1;
}

.query {
    flex-grow: 2;
    background: none;
    padding: 4px;
    border: 1px solid #272727;
    color: white;
    position: relative;
}

.query-container {
    flex: 1;
    display: flex;
    align-items: center;
    column-gap: 10px;
}

.clear-btn {
    border: none;
}
</style>