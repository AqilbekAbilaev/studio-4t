<script setup>
import { ref } from 'vue'
import { getImageUrl } from '../../utils'

import Modal from "./menubar-options-modal.vue"

const props = defineProps(['isPopupClosed'])

const openModalId = ref(0)

const menuItems = [
    {
        id: 1,
        name: "File",
        options: [
            {
                id: 1,
                name: "Connect"
            },
            {
                id: 2,
                name: "Add Database"
            },
            {
                id: 3,
                name: "Open IntelliShell",
                icon: "terminal.svg"
            },
            {
                id: 4,
                name: "Open SQL"
            },
            {
                id: 5,
                name: "Open Tasks"
            },
            {
                id: 6,
                name: "Search in..."
            },
            {
                id: 7,
                name: "Manage SQL Connections"
            },
            {
                id: 8,
                name: "Load"
            },
            {
                id: 9,
                name: "Save"
            },
            {
                id: 10,
                name: "Server Status"
            },
            {
                id: 11,
                name: "Exit"
            }
        ]
    },
    {
        id: 2,
        name: "Edit",
        options: [
            {
                id: 1,
                name: "Copy Value As Text"
            },
            {
                id: 2,
                name: "Copy Value"
            },
            {
                id: 3,
                name: "Copy Field"
            },
            {
                id: 4,
                name: "Copy Field Path"
            },
            {
                id: 5,
                name: "Copy Document"
            },
            {
                id: 6,
                name: "Paste Documents(s)"
            },
            {
                id: 7,
                name: "Preferences.. (Ctrl+P)"
            }
        ]
    },
    {
        id: 3,
        name: "Database",
        options: [
            {
                name: "hi"
            }
        ]
    },
    {
        id: 4,
        name: "Collection",
        options: [
            {
                name: "hi"
            }
        ]
    },
    {
        id: 5,
        name: "Index",
        options: [
            {
                name: "hi"
            }
        ]
    },
    {
        id: 6,
        name: "GridFS",
        options: [
            {
                name: "hi"
            }
        ]
    },
    {
        id: 7,
        name: "View",
        options: [
            {
                name: "hi"
            }
        ]
    },
    {
        id: 8,
        name: "Help",
        options: [
            {
                name: "hi"
            }
        ]
    },


]


</script>
<template>
    <div class="menubar">
        <div class="nav-container">
            <div v-for="item in menuItems" class="menubar-item">
                <a class="pointer" @click.prevent="openModalId = item.id; $emit('close-modal')"
                    @mouseover="openModalId = item.id">
                    {{ item.name }}
                </a>
                <Modal class="option" v-if="openModalId == item.id && isPopupClosed" :options="item.options" />
            </div>
        </div>


        <div class="disable-menubar pointer" @click.prevent="$emit('disable-menubar')">
            <img :src="getImageUrl('close.svg')" width="16px" height="16px" />
        </div>
    </div>
</template>



<style scoped>
.menubar {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: #4D4D4D;
}

.nav-container {
    display: flex;
    padding: 4px;

    .menubar-item {
        position: relative;
    }

    .menubar-item:not(:first-child) {
        margin-left: 10px;
    }
}

.option {
    position: absolute;
    top: 24px;
    border: 1px solid black;
}

a {
    list-style: none;
    font-size: 16px;
    font-style: normal;
    font-weight: 100;
}

.disable-menubar {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-right: 4px;
}
</style>