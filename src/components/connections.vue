<script setup>
import { ref } from "vue";
import Connection from "./database/connection.vue";
const props = defineProps(['options'])

const databaseConnections = [
    {
        id: 1,
        name: "Local",
        databases: [
            {
                name: "admin",
                collections: [
                    {
                        name: "users"
                    }
                ]
            },
            {
                name: "roles",
                collections: [
                    {
                        name: "users"
                    }
                ]
            },
            {
                name: "shits",
                collections: [
                    {
                        name: "users"
                    }
                ]
            }
        ],
    },
    {
        id: 2,
        name: "Prod",
        collections: [
            {
                name: "admin",
                documents: [
                    {
                        name: "users",
                    },
                    {
                        name: "roles"
                    }
                ]
            }
        ],
    }
]
const selectedConnectionId = ref(0);

</script>
<template>
    <div class="container">
        <div class="connections">
            <Connection v-for="databaseConnection in databaseConnections" :connection="databaseConnection"
                :selectedConnectionId="selectedConnectionId" @select-connection="e => selectedConnectionId = e" />
        </div>
    </div>
</template>



<style scoped>
.container {
    width: 30%;
    background-color: #3c3c3c;
    height: 100%;
    padding: 4px;
    /* border: 1px solid #4d4d4d; */
}

.connections {
    height: 100%;
    /* background-color: white; */
    border: 1px solid #575757;
    padding: 6px;
}
</style>