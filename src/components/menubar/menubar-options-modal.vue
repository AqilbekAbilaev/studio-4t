<script setup>
import { getImageUrl } from '../../utils'
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow, WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Webview } from '@tauri-apps/api/webview'
import { getCurrentWindow, Window } from '@tauri-apps/api/window'

const emit = defineEmits(['close-modal'])



async function open_connection() {
    console.log("hi")
    const position = await getCurrentWindow().innerPosition()
    const width = await getCurrentWindow().innerSize()
    console.log(width)
    

    const webview = new WebviewWindow('my-label', {
        url: 'https://kun.uz',
        // center: true,
        alwaysOnTop: true,
        // visible: false,
        // center: true,
        parent: getCurrentWebviewWindow(),
        // maxHeight: 250,
        // maxWidth: 450,
        x: position.x + width.width / 2 - 225,
        y: position.y + width.height / 2 - 125,
    });


    // since the webview window is created asynchronously,
    // Tauri emits the `tauri://created` and `tauri://error` to notify you of the creation response
    webview.once('tauri://created', async function () {
        // webview window successfully created
        // const position = await appWindow.innerPosition()
        // console.log(position)
        emit('close-modal')
        console.log()

    })
    webview.once('tauri://error', function (e) {
        console.log(e)
        // an error occurred during webview window creation
    })
}

const props = defineProps(['options'])

</script>
<template>
    <div class="modal">
        <div class="option-container pointer" v-for="item in options" @click="open_connection">
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