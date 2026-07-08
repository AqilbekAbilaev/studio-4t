// Pure module (no Vue) that normalizes a MongoDB `find` explain document into a
// render-ready tree for the Visual Explain graph (see ExplainGraph.vue).
//
// Orientation mirrors Studio 3T: the synthetic **Result** node is the OUTPUT and
// sits at the root (leftmost when drawn); execution stages hang off it as children,
// with the data source (COLLSCAN / IXSCAN) as the deepest descendant.
//
// We prefer `executionStats.executionStages` because it carries runtime numbers
// (nReturned, time, docs/keys examined). When only `queryPlanner.winningPlan` is
// present (verbosity below executionStats), we fall back to it for structure and
// leave the counts/timings null.

// Friendly label per stage. Unmapped stages fall back to the raw stage string.
const STAGE_LABELS = {
  COLLSCAN:           'Collection scan',
  IXSCAN:             'Index scan',
  FETCH:              'Fetch',
  SORT:               'Sort',
  SORT_KEY_GENERATOR: 'Sort key generator',
  SORT_MERGE:         'Sort merge',
  LIMIT:              'Limit',
  SKIP:               'Skip',
  PROJECTION_SIMPLE:  'Projection',
  PROJECTION_COVERED: 'Projection',
  PROJECTION_DEFAULT: 'Projection',
  OR:                 'Or',
  AND_SORTED:         'And (sorted)',
  AND_HASH:           'And (hash)',
  SUBPLAN:            'Subplan',
  COUNT:              'Count',
  COUNT_SCAN:         'Count scan',
  DISTINCT_SCAN:      'Distinct scan',
  TEXT:               'Text',
  GEO_NEAR_2D:        'Geo near (2d)',
  GEO_NEAR_2DSPHERE:  'Geo near (2dsphere)',
  SHARD_MERGE:        'Shard merge',
  GROUP:              'Group',
}

export function labelForStage(stage) {
  if (!stage) return 'Stage'
  return STAGE_LABELS[stage] || stage
}

// A number or null — explain nodes sometimes omit a field entirely.
function numOrNull(value) {
  return value === undefined || value === null ? null : value
}

// Recursively normalize one plan/executionStages node and its input stage(s).
// Both `inputStage` (single object) and `inputStages` (array — OR / SORT_MERGE /
// AND_*) are walked so branching plans keep all their arms.
function normalizeStage(node) {
  if (!node || typeof node !== 'object') return null

  const children = []
  if (node.inputStage) {
    const child = normalizeStage(node.inputStage)
    if (child) children.push(child)
  }
  if (Array.isArray(node.inputStages)) {
    for (const input of node.inputStages) {
      const child = normalizeStage(input)
      if (child) children.push(child)
    }
  }

  return {
    stage:        node.stage || null,
    label:        labelForStage(node.stage),
    timeMs:       numOrNull(node.executionTimeMillisEstimate),
    nReturned:    numOrNull(node.nReturned),
    docsExamined: numOrNull(node.docsExamined),
    keysExamined: numOrNull(node.keysExamined),
    works:        numOrNull(node.works),
    indexName:    node.indexName || null,
    children:     children,
  }
}

// Build the render-ready tree from a raw explain document. Returns the synthetic
// Result root (with the plan's top stage as its single child), or null when the
// document has no recognizable plan.
export function buildExplainTree(explainDoc) {
  if (!explainDoc || typeof explainDoc !== 'object') return null

  const stats = explainDoc.executionStats || null
  const planner = explainDoc.queryPlanner || null

  // Prefer the annotated executionStages tree; fall back to the winning plan.
  const planRoot =
    (stats && stats.executionStages) ||
    (planner && planner.winningPlan) ||
    null

  const topStage = normalizeStage(planRoot)
  if (!topStage) return null

  return {
    stage:     'RESULT',
    label:     'Result',
    isResult:  true,
    // Top-level totals live on the Result node (folds in the old summary chips).
    timeMs:       stats ? numOrNull(stats.executionTimeMillis) : null,
    nReturned:    stats ? numOrNull(stats.nReturned) : null,
    docsExamined: stats ? numOrNull(stats.totalDocsExamined) : null,
    keysExamined: stats ? numOrNull(stats.totalKeysExamined) : null,
    works:        null,
    indexName:    null,
    children:  [topStage],
  }
}
