<script setup>
import { getImageUrl } from '../../utils'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

const emit = defineEmits(['close-modal'])
const props = defineProps(['options'])

async function open_connection() {
    const webview = new WebviewWindow('connect-window', {
        url: 'src/pages/connect.html',
        title: 'New Connection',
        width: 480,
        height: 460,
        resizable: false,
        center: true,
    })

    webview.once('tauri://created', () => {
        emit('close-modal')
    })

    webview.once('tauri://error', (e) => {
        console.error('Failed to open connection dialog:', e)
    })
}

function handleOptionClick(item) {
    if (item.id === 1) {
        open_connection()
    }
}
</script>
<template>
    <div class="modal">
        <div class="option-container pointer" v-for="item in options" @click="handleOptionClick(item)">
            <img v-if="item.icon" :src="getImageUrl(item.icon)" alt="" width="16px" />
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
    gap: 8px;
    padding: 4px 10px;
    font-weight: 300;
    color: white;
}

.option-container:hover {
    background-color: #3489eb;
}
</style>
