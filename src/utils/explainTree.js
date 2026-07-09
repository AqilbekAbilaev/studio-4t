// Pure module (no Vue) that normalizes a MongoDB explain document into a
// render-ready tree for the Visual Explain graph (see ExplainGraph.vue). Handles the
// three shapes we can lay out — a `find` plan, an `aggregate` pipeline, and a graceful
// notice for sharded plans — plus a bottleneck-severity pass.
//
// Orientation mirrors Studio 3T: the synthetic **Result** node is the OUTPUT and
// sits at the root (leftmost when drawn); execution stages hang off it as children,
// with the data source (COLLSCAN / IXSCAN / $cursor scan) as the deepest descendant.
//
// We prefer `executionStats.executionStages` because it carries runtime numbers
// (nReturned, time, docs/keys examined). When only `queryPlanner.winningPlan` is
// present (verbosity below executionStats), we fall back to it for structure and
// leave the counts/timings null.

// Friendly label per stage. Unmapped stages fall back to the raw stage string.
// Aggregation stages are keyed with their leading `$` (e.g. `$group`).
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
  SHARDED:            'Sharded plan',
  // Aggregation pipeline stages.
  $cursor:            'Cursor',
  $match:             'Match',
  $group:             'Group',
  $sort:              'Sort',
  $project:           'Project',
  $unwind:            'Unwind',
  $limit:             'Limit',
  $skip:              'Skip',
  $lookup:            'Lookup',
  $facet:             'Facet',
  $addFields:         'Add fields',
  $set:               'Set',
  $sample:            'Sample',
  $graphLookup:       'Graph lookup',
  $bucket:            'Bucket',
  $bucketAuto:        'Bucket (auto)',
  $unionWith:         'Union with',
  $count:             'Count',
  $replaceRoot:       'Replace root',
  $replaceWith:       'Replace with',
  $sortByCount:       'Sort by count',
}

export function labelForStage(stage) {
  if (!stage) return 'Stage'
  return STAGE_LABELS[stage] || stage
}

// A number or null — explain nodes sometimes omit a field entirely.
function numOrNull(value) {
  return value === undefined || value === null ? null : value
}

// Best-effort memory usage in bytes for a stage, from whichever field the server
// happened to report it under (SORT stages use `memUsage` /
// `totalDataSizeSortedBytesEstimate`, GROUP uses `maxUsedMemBytes`). Null when none
// is present or the value isn't a number.
function memBytesFor(node) {
  const candidates = [
    node.memUsage,
    node.totalDataSizeSortedBytesEstimate,
    node.maxUsedMemBytes,
  ]
  for (const candidate of candidates) {
    if (typeof candidate === 'number') return candidate
  }
  return null
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
    memBytes:     memBytesFor(node),
    children:     children,
  }
}

// The one operator key (`$group`, …) of a pipeline-stage document, skipping the
// sibling runtime fields (nReturned, executionTimeMillisEstimate). Null if none.
function stageKeyOf(stageDoc) {
  for (const key of Object.keys(stageDoc)) {
    if (key.startsWith('$')) return key
  }
  return null
}

// Normalize one aggregation pipeline stage element. The `$cursor` stage embeds a
// `find` sub-plan (the initial scan/fetch), which we recurse through the existing
// normalizeStage so the scan becomes the deepest chained node beneath it.
function normalizeAggStage(stageDoc) {
  const key = stageKeyOf(stageDoc)
  const node = {
    stage:        key,
    label:        labelForStage(key),
    timeMs:       numOrNull(stageDoc.executionTimeMillisEstimate),
    nReturned:    numOrNull(stageDoc.nReturned),
    docsExamined: null,
    keysExamined: null,
    works:        null,
    indexName:    null,
    memBytes:     memBytesFor(stageDoc),
    children:     [],
  }
  if (key === '$cursor') {
    const cursor = stageDoc.$cursor || {}
    const stats = cursor.executionStats || null
    const planner = cursor.queryPlanner || null
    const planRoot =
      (stats && stats.executionStages) ||
      (planner && planner.winningPlan) ||
      null
    const sub = normalizeStage(planRoot)
    if (sub) node.children.push(sub)
    // Fall back to the cursor's own runtime numbers when the stage element omitted them.
    if (node.nReturned === null && stats) node.nReturned = numOrNull(stats.nReturned)
    if (node.timeMs === null && stats) node.timeMs = numOrNull(stats.executionTimeMillis)
    if (node.docsExamined === null && stats) node.docsExamined = numOrNull(stats.totalDocsExamined)
    if (node.keysExamined === null && stats) node.keysExamined = numOrNull(stats.totalKeysExamined)
  }
  return node
}

// Build the tree for an aggregation explain (`explainDoc.stages` is an array in
// execution order). We chain the stages in REVERSE execution order: the last stage's
// output is nearest Result; the `$cursor`/scan is the deepest node.
function buildAggregateTree(explainDoc) {
  const stages = explainDoc.stages
  if (!Array.isArray(stages) || stages.length === 0) return null

  const nodes = stages.map((stage) => normalizeAggStage(stage))
  // Hang each stage as the child of the one that runs after it, so execution order
  // reversed becomes Result → last stage → … → $cursor → scan.
  for (let i = 1; i < nodes.length; i++) {
    nodes[i].children.push(nodes[i - 1])
  }
  const topNode = nodes[nodes.length - 1]

  // Pipeline totals, when the server reported them, otherwise the final stage's output.
  const stats = explainDoc.executionStats || null
  return {
    stage:        'RESULT',
    label:        'Result',
    isResult:     true,
    timeMs:       stats ? numOrNull(stats.executionTimeMillis) : numOrNull(topNode.timeMs),
    nReturned:    stats ? numOrNull(stats.nReturned) : numOrNull(topNode.nReturned),
    docsExamined: stats ? numOrNull(stats.totalDocsExamined) : null,
    keysExamined: stats ? numOrNull(stats.totalKeysExamined) : null,
    works:        null,
    indexName:    null,
    memBytes:     null,
    children:     [topNode],
  }
}

// Build the tree for a `find` explain (executionStages preferred, winningPlan fallback).
function buildFindTree(explainDoc) {
  const stats = explainDoc.executionStats || null
  const planner = explainDoc.queryPlanner || null

  const planRoot =
    (stats && stats.executionStages) ||
    (planner && planner.winningPlan) ||
    null

  const topStage = normalizeStage(planRoot)
  if (!topStage) return null

  return {
    stage:        'RESULT',
    label:        'Result',
    isResult:     true,
    // Top-level totals live on the Result node (folds in the old summary chips).
    timeMs:       stats ? numOrNull(stats.executionTimeMillis) : null,
    nReturned:    stats ? numOrNull(stats.nReturned) : null,
    docsExamined: stats ? numOrNull(stats.totalDocsExamined) : null,
    keysExamined: stats ? numOrNull(stats.totalKeysExamined) : null,
    works:        null,
    indexName:    null,
    memBytes:     null,
    children:     [topStage],
  }
}

// A minimal notice tree for sharded plans, whose per-shard structure we don't lay out.
// The graph shows the single SHARDED node with its note; raw JSON stays available.
function buildShardedNotice() {
  return {
    stage:        'RESULT',
    label:        'Result',
    isResult:     true,
    timeMs:       null,
    nReturned:    null,
    docsExamined: null,
    keysExamined: null,
    works:        null,
    indexName:    null,
    memBytes:     null,
    children: [{
      stage:        'SHARDED',
      label:        'Sharded plan',
      note:         'View JSON for per-shard detail',
      timeMs:       null,
      nReturned:    null,
      docsExamined: null,
      keysExamined: null,
      works:        null,
      indexName:    null,
      memBytes:     null,
      children:     [],
    }],
  }
}

// Bottleneck heuristic. Walks the tree once and tags each node with a `severity`:
//   'warn' — a collection scan (COLLSCAN): no index used at that point.
//   'hot'  — the single slowest node when its time dominates the plan (>= 50% of the
//            Result total, and the total is above a small floor so near-zero plans
//            aren't flagged); OR a scan/FETCH node with poor selectivity (examines
//            >= 100 docs and >= 10x what it returns).
//   null   — otherwise.
// 'hot' takes precedence over 'warn'. Pure aside from tagging the passed-in nodes.
export function annotateSeverity(root) {
  if (!root) return root

  const all = []
  const collect = (node) => {
    all.push(node)
    for (const child of node.children || []) collect(child)
  }
  collect(root)

  for (const node of all) node.severity = null

  const totalTime = root.timeMs
  // Slowest non-Result node with a known timing (for the dominating-time rule).
  let maxTimeNode = null
  for (const node of all) {
    if (node.isResult) continue
    if (node.timeMs === null || node.timeMs === undefined) continue
    if (maxTimeNode === null || node.timeMs > maxTimeNode.timeMs) maxTimeNode = node
  }

  const HOT_MIN_TOTAL_MS = 5 // floor so tiny/near-zero plans aren't flagged as hot

  for (const node of all) {
    if (node.isResult) continue
    if (node.stage === 'COLLSCAN') node.severity = 'warn'
    // Poor selectivity on a scan / fetch → hot (overrides the COLLSCAN warn above).
    const scanLike =
      node.stage === 'COLLSCAN' || node.stage === 'IXSCAN' || node.stage === 'FETCH'
    if (scanLike && node.docsExamined !== null && node.docsExamined !== undefined) {
      const returned = node.nReturned || 1
      if (node.docsExamined >= 100 && node.docsExamined >= 10 * returned) {
        node.severity = 'hot'
      }
    }
  }

  // Dominating-time node → hot. Only meaningful with real (non-null, non-trivial) timings.
  if (maxTimeNode && typeof totalTime === 'number' && totalTime >= HOT_MIN_TOTAL_MS) {
    if (maxTimeNode.timeMs >= 0.5 * totalTime) maxTimeNode.severity = 'hot'
  }

  return root
}

// Build the render-ready tree from a raw explain document. Detects the shape
// (sharded / aggregate / find), builds the matching tree, then runs the severity
// pass. Returns null when the document has no recognizable plan.
export function buildExplainTree(explainDoc) {
  if (!explainDoc || typeof explainDoc !== 'object') return null

  let root = null
  if (explainDoc.shards || explainDoc.splitPipeline) {
    root = buildShardedNotice()
  } else if (Array.isArray(explainDoc.stages)) {
    root = buildAggregateTree(explainDoc)
  } else {
    root = buildFindTree(explainDoc)
  }
  if (!root) return null

  return annotateSeverity(root)
}
