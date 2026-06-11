<script setup>
import { ref } from 'vue'
import BaseIcon from './BaseIcon.vue'

// Each section can be individually enabled
const queryEnabled      = ref(true)
const projectionEnabled = ref(false)
const sortEnabled       = ref(false)
</script>

<template>
  <div class="vqb">

    <!-- Query section -->
    <div class="vqb-section">
      <div class="vqb-head">
        Query
        <span class="cb" :class="{ on: queryEnabled }" @click="queryEnabled = !queryEnabled">
          <BaseIcon v-if="queryEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body">
        <div class="vqb-row1">
          <span class="vqb-select">
            Match all of ($and)
            <BaseIcon name="caretDown" :size="12" />
          </span>
          <button class="vqb-clear">Clear</button>
        </div>
        <div class="dropzone">
          <BaseIcon name="plus" :size="14" />
          Drag and drop field here or double-click
          <span class="add">Add</span>
        </div>
      </div>
    </div>

    <!-- Projection section -->
    <div class="vqb-section">
      <div class="vqb-head">
        Projection
        <span class="cb" :class="{ on: projectionEnabled }" @click="projectionEnabled = !projectionEnabled">
          <BaseIcon v-if="projectionEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body">
        <div class="dropzone">
          <BaseIcon name="plus" :size="14" />
          Drag and drop fields here or double-click
        </div>
      </div>
    </div>

    <!-- Sort section -->
    <div class="vqb-section">
      <div class="vqb-head">
        Sort
        <span class="cb" :class="{ on: sortEnabled }" @click="sortEnabled = !sortEnabled">
          <BaseIcon v-if="sortEnabled" name="check" :size="12" />
        </span>
      </div>
      <div class="vqb-body">
        <div class="dropzone">
          <BaseIcon name="plus" :size="14" />
          Drag and drop fields here or double-click
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.vqb {
  width: 360px;
  flex: none;
  background: var(--bg-panel);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
}

.vqb-section {
  border-bottom: 1px solid var(--border);
}

.vqb-head {
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  padding: 9px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  background: var(--bg-panel-2);
}
.vqb-head .cb {
  position: absolute;
  right: 10px;
}

.vqb-body {
  padding: 10px;
}

.vqb-row1 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 9px;
}

.vqb-select {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
  padding: 6px 9px;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
}

.vqb-clear {
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: var(--bg-toolbar);
  color: var(--text);
  font-size: 12.5px;
}
.vqb-clear:hover { background: var(--bg-hover); }

/* checkbox */
.cb {
  width: 17px;
  height: 17px;
  border-radius: 4px;
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  display: grid;
  place-items: center;
  flex: none;
  cursor: pointer;
}
.cb.on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

/* dropzone */
.dropzone {
  border: 1px dashed var(--border-soft);
  border-radius: 6px;
  padding: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-faint);
  flex-wrap: wrap;
}
.add {
  color: var(--link);
  cursor: pointer;
}
.add:hover { text-decoration: underline; }
</style>
