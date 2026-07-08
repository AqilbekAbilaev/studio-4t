<script setup>
// Visual Explain node-graph. Renders the parsed explain tree (see
// utils/explainTree.js) as an inline-SVG graph, Studio-3T style: the Result box on
// the left, execution stages flowing in from the right, the data source (COLLSCAN /
// IXSCAN) as the deepest node. All parsing lives in the util — this component only
// lays out and draws the already-normalized tree.
import { computed } from 'vue'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  // The render-ready tree from buildExplainTree(), or null.
  tree: { type: Object, default: null },
})

// ── layout constants ──────────────────────────────────
const NODE_W = 196
const NODE_H = 66
const COL_GAP = 74 // horizontal gap between depth columns
const ROW_GAP = 26 // vertical gap between stacked rows
const PAD = 22

// Stage → icon. Reuses existing glyphs where one fits; the ex* glyphs are dedicated.
const STAGE_ICONS = {
  RESULT:             'exResult',
  COLLSCAN:           'exScan',
  IXSCAN:             'exIndex',
  COUNT_SCAN:         'exIndex',
  DISTINCT_SCAN:      'exIndex',
  FETCH:              'exFetch',
  SORT:               'exSort',
  SORT_MERGE:         'exSort',
  SORT_KEY_GENERATOR: 'exSort',
  LIMIT:              'filter',
  SKIP:               'filter',
  PROJECTION_SIMPLE:  'collSmall',
  PROJECTION_COVERED: 'collSmall',
  PROJECTION_DEFAULT: 'collSmall',
  OR:                 'aggregate',
  SORT_MERGE_OR:      'aggregate',
}

function iconFor(node) {
  if (node.isResult) return 'exResult'
  return STAGE_ICONS[node.stage] || 'cog'
}

function nodeTitle(node) {
  if (node.stage === 'IXSCAN' && node.indexName) return `${node.label} · ${node.indexName}`
  return node.label
}

// ms → "0.2s" like the 3T screenshot; sub-100ms stays in ms so we don't round tiny
// timings to a misleading "0.0s". "—" when the timing is unknown (planner fallback).
function fmtTime(ms) {
  if (ms === null || ms === undefined) return '—'
  if (ms >= 100) return `${(ms / 1000).toFixed(1)}s`
  return `${ms} ms`
}

function fmtCount(n) {
  if (n === null || n === undefined) return null
  return n.toLocaleString()
}

// ── layout ────────────────────────────────────────────
// Tidy-tree pass: column = depth from Result (grows rightward); leaves each take the
// next free row, internal nodes center over their children. A linear chain therefore
// draws as one straight row; a branch (multiple inputStages) fans its arms onto
// separate rows and the parent sits centered between them.
const layout = computed(() => {
  const root = props.tree
  if (!root) return null

  const nodes = []
  const edges = []
  let nextRow = 0
  let maxCol = 0

  function place(node, col) {
    const children = node.children || []
    let row
    let placedChildren = []
    if (children.length === 0) {
      row = nextRow
      nextRow += 1
    } else {
      placedChildren = children.map((child) => place(child, col + 1))
      const first = placedChildren[0].row
      const last = placedChildren[placedChildren.length - 1].row
      row = (first + last) / 2
    }
    maxCol = Math.max(maxCol, col)
    const placed = {
      id: nodes.length,
      stage: node.stage,
      label: node.label,
      isResult: !!node.isResult,
      indexName: node.indexName,
      timeMs: node.timeMs,
      nReturned: node.nReturned,
      docsExamined: node.docsExamined,
      keysExamined: node.keysExamined,
      col: col,
      row: row,
      x: PAD + col * (NODE_W + COL_GAP),
      y: PAD + row * (NODE_H + ROW_GAP),
    }
    nodes.push(placed)
    // Edge from this node (output, left) to each child (source, right). Docs flow
    // from the child into the parent, so the edge is labelled with the child's
    // nReturned and its arrowhead points back toward the parent.
    for (const child of placedChildren) {
      edges.push({ from: placed, to: child, docs: child.nReturned })
    }
    return placed
  }

  place(root, 0)

  let width = 0
  let height = 0
  for (const node of nodes) {
    width = Math.max(width, node.x + NODE_W)
    height = Math.max(height, node.y + NODE_H)
  }

  return {
    nodes: nodes,
    edges: edges,
    width: width + PAD,
    height: height + PAD,
  }
})

// Cubic edge path from the child's left edge to the parent's right edge (drawn in
// flow direction so marker-end lands the arrow on the parent).
function edgePath(edge) {
  const x1 = edge.to.x
  const y1 = edge.to.y + NODE_H / 2
  const x2 = edge.from.x + NODE_W
  const y2 = edge.from.y + NODE_H / 2
  const mx = (x1 + x2) / 2
  return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`
}

function edgeLabelPos(edge) {
  const x1 = edge.to.x
  const y1 = edge.to.y + NODE_H / 2
  const x2 = edge.from.x + NODE_W
  const y2 = edge.from.y + NODE_H / 2
  return { x: (x1 + x2) / 2, y: (y1 + y2) / 2 - 7 }
}
</script>

<template>
  <div class="explain-graph">
    <div v-if="!layout" class="eg-empty">No plan to display.</div>
    <svg
      v-else
      class="eg-svg"
      :width="layout.width"
      :height="layout.height"
      :viewBox="`0 0 ${layout.width} ${layout.height}`"
    >
      <defs>
        <marker
          id="eg-arrow"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto"
        >
          <path d="M0 0 L10 5 L0 10 z" class="eg-arrow-head" />
        </marker>
      </defs>

      <!-- edges first so nodes paint over them -->
      <g class="eg-edges">
        <template v-for="edge in layout.edges" :key="`e${edge.from.id}-${edge.to.id}`">
          <path :d="edgePath(edge)" class="eg-edge" marker-end="url(#eg-arrow)" />
          <text
            v-if="fmtCount(edge.docs) !== null"
            class="eg-edge-label"
            :x="edgeLabelPos(edge).x"
            :y="edgeLabelPos(edge).y"
            text-anchor="middle"
          >{{ fmtCount(edge.docs) }} docs</text>
        </template>
      </g>

      <!-- nodes -->
      <g class="eg-nodes">
        <foreignObject
          v-for="node in layout.nodes"
          :key="`n${node.id}`"
          :x="node.x"
          :y="node.y"
          :width="NODE_W"
          :height="NODE_H"
        >
          <div
            xmlns="http://www.w3.org/1999/xhtml"
            class="eg-node"
            :class="{ 'is-result': node.isResult }"
          >
            <div class="eg-node-head">
              <span class="eg-node-icon"><BaseIcon :name="iconFor(node)" :size="15" /></span>
              <span class="eg-node-title" :title="nodeTitle(node)">{{ nodeTitle(node) }}</span>
            </div>
            <div class="eg-node-meta">
              <span class="eg-node-time"><BaseIcon name="clock" :size="11" /> {{ fmtTime(node.timeMs) }}</span>
              <span v-if="node.isResult && fmtCount(node.nReturned) !== null" class="eg-node-ret">
                {{ fmtCount(node.nReturned) }} returned
              </span>
              <span v-else-if="!node.isResult && fmtCount(node.docsExamined) !== null" class="eg-node-ret">
                {{ fmtCount(node.docsExamined) }} examined
              </span>
            </div>
          </div>
        </foreignObject>
      </g>
    </svg>
  </div>
</template>

<style scoped>
.explain-graph {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px;
}
.eg-empty {
  padding: 32px;
  color: var(--text-faint);
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.eg-svg { display: block; }

.eg-edge {
  fill: none;
  stroke: var(--border-soft);
  stroke-width: 1.6px;
}
.eg-arrow-head { fill: var(--text-faint); }
.eg-edge-label {
  font-family: var(--mono);
  font-size: 10.5px;
  fill: var(--text-dim);
  /* halo so the label stays legible where it crosses the edge */
  paint-order: stroke;
  stroke: var(--bg-panel);
  stroke-width: 3px;
  stroke-linejoin: round;
}

/* nodes are HTML inside <foreignObject> */
.eg-node {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  background: var(--bg-panel);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  font-family: system-ui, sans-serif;
}
.eg-node.is-result {
  border-color: var(--accent);
  background: var(--bg-panel-2);
}
.eg-node-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.eg-node-icon {
  display: flex;
  align-items: center;
  color: var(--accent);
  flex: 0 0 auto;
}
.is-result .eg-node-icon { color: var(--accent); }
.eg-node-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.eg-node-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11px;
  color: var(--text-dim);
}
.eg-node-time {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--green);
}
.eg-node-ret {
  color: var(--text-faint);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
