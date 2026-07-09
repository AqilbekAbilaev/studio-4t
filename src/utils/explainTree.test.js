import { describe, it, expect } from 'vitest'
import { buildExplainTree, labelForStage, annotateSeverity } from './explainTree'

// Realistic (trimmed) find-explain documents. Shapes match what the Rust
// `explain_query` command returns with executionStats verbosity.

// Simple full-collection scan.
const collscanDoc = {
  queryPlanner: {
    winningPlan: { stage: 'COLLSCAN', direction: 'forward' },
  },
  executionStats: {
    nReturned: 5,
    executionTimeMillis: 3,
    totalKeysExamined: 0,
    totalDocsExamined: 200,
    executionStages: {
      stage: 'COLLSCAN',
      nReturned: 5,
      executionTimeMillisEstimate: 2,
      works: 202,
      docsExamined: 200,
    },
  },
}

// IXSCAN feeding a FETCH — the classic indexed lookup.
const ixscanFetchDoc = {
  queryPlanner: {
    winningPlan: {
      stage: 'FETCH',
      inputStage: {
        stage: 'IXSCAN',
        indexName: 'name_1',
        keyPattern: { name: 1 },
      },
    },
  },
  executionStats: {
    nReturned: 2,
    executionTimeMillis: 1,
    totalKeysExamined: 2,
    totalDocsExamined: 2,
    executionStages: {
      stage: 'FETCH',
      nReturned: 2,
      executionTimeMillisEstimate: 1,
      works: 3,
      docsExamined: 2,
      inputStage: {
        stage: 'IXSCAN',
        nReturned: 2,
        executionTimeMillisEstimate: 0,
        works: 3,
        keysExamined: 2,
        indexName: 'name_1',
      },
    },
  },
}

// Branching plan: FETCH ← OR ← two IXSCANs.
const orBranchDoc = {
  executionStats: {
    nReturned: 4,
    executionTimeMillis: 2,
    totalKeysExamined: 5,
    totalDocsExamined: 4,
    executionStages: {
      stage: 'FETCH',
      nReturned: 4,
      executionTimeMillisEstimate: 2,
      docsExamined: 4,
      inputStage: {
        stage: 'OR',
        nReturned: 4,
        executionTimeMillisEstimate: 1,
        inputStages: [
          { stage: 'IXSCAN', nReturned: 2, keysExamined: 2, indexName: 'a_1' },
          { stage: 'IXSCAN', nReturned: 2, keysExamined: 3, indexName: 'b_1' },
        ],
      },
    },
  },
}

// Structure only — no executionStats (lower verbosity fallback).
const plannerOnlyDoc = {
  queryPlanner: {
    winningPlan: {
      stage: 'SORT',
      inputStage: {
        stage: 'COLLSCAN',
        direction: 'forward',
      },
    },
  },
}

// Aggregate explain: stages array (execution order) with a $cursor sub-plan, a
// $group (memory via maxUsedMemBytes), and a $sort (memory via sorted-bytes estimate).
const aggregateDoc = {
  stages: [
    {
      $cursor: {
        queryPlanner: { winningPlan: { stage: 'COLLSCAN' } },
        executionStats: {
          nReturned: 100,
          executionTimeMillis: 4,
          totalDocsExamined: 100,
          totalKeysExamined: 0,
          executionStages: {
            stage: 'COLLSCAN',
            nReturned: 100,
            executionTimeMillisEstimate: 3,
            docsExamined: 100,
          },
        },
      },
      nReturned: 100,
      executionTimeMillisEstimate: 3,
    },
    {
      $group: { _id: '$cat', total: { $sum: 1 } },
      nReturned: 5,
      executionTimeMillisEstimate: 2,
      maxUsedMemBytes: 2200000,
    },
    {
      $sort: { total: -1 },
      nReturned: 5,
      executionTimeMillisEstimate: 1,
      totalDataSizeSortedBytesEstimate: 1048576,
    },
  ],
}

// Find explain whose SORT stage reports memory usage.
const sortMemDoc = {
  executionStats: {
    nReturned: 10,
    executionTimeMillis: 5,
    totalKeysExamined: 0,
    totalDocsExamined: 50,
    executionStages: {
      stage: 'SORT',
      nReturned: 10,
      executionTimeMillisEstimate: 4,
      memUsage: 3145728,
      totalDataSizeSortedBytesEstimate: 3000000,
      inputStage: { stage: 'COLLSCAN', nReturned: 50, docsExamined: 50 },
    },
  },
}

// COLLSCAN with good selectivity and a trivial timing → warn (no index) but not hot.
const goodCollscanDoc = {
  executionStats: {
    nReturned: 40,
    executionTimeMillis: 2,
    totalKeysExamined: 0,
    totalDocsExamined: 50,
    executionStages: {
      stage: 'COLLSCAN',
      nReturned: 40,
      executionTimeMillisEstimate: 1,
      docsExamined: 50,
    },
  },
}

// Indexed plan whose FETCH time dominates the total (>= 50%) → that node is hot.
const slowFetchDoc = {
  executionStats: {
    nReturned: 5,
    executionTimeMillis: 10,
    totalKeysExamined: 5,
    totalDocsExamined: 5,
    executionStages: {
      stage: 'FETCH',
      nReturned: 5,
      executionTimeMillisEstimate: 8,
      docsExamined: 5,
      inputStage: {
        stage: 'IXSCAN',
        nReturned: 5,
        executionTimeMillisEstimate: 1,
        keysExamined: 5,
        indexName: 'x_1',
      },
    },
  },
}

// Sharded find explain: per-shard sub-structures we don't lay out.
const shardedDoc = {
  queryPlanner: { winningPlan: { stage: 'SHARD_MERGE' } },
  shards: { shard0000: { executionStages: { stage: 'COLLSCAN' } } },
}

describe('labelForStage', () => {
  it('maps known stages to friendly names', () => {
    expect(labelForStage('COLLSCAN')).toBe('Collection scan')
    expect(labelForStage('IXSCAN')).toBe('Index scan')
    expect(labelForStage('PROJECTION_COVERED')).toBe('Projection')
    expect(labelForStage('SORT_MERGE')).toBe('Sort merge')
  })
  it('maps aggregation stage keys to friendly names', () => {
    expect(labelForStage('$group')).toBe('Group')
    expect(labelForStage('$lookup')).toBe('Lookup')
    expect(labelForStage('$unwind')).toBe('Unwind')
  })
  it('falls back to the raw stage string when unmapped', () => {
    expect(labelForStage('MYSTERY_STAGE')).toBe('MYSTERY_STAGE')
    expect(labelForStage('$mysteryStage')).toBe('$mysteryStage')
  })
})

describe('buildExplainTree', () => {
  it('returns null for a non-plan document', () => {
    expect(buildExplainTree(null)).toBeNull()
    expect(buildExplainTree({})).toBeNull()
    expect(buildExplainTree('nope')).toBeNull()
  })

  it('builds a Result root carrying top-level totals', () => {
    const root = buildExplainTree(collscanDoc)
    expect(root.isResult).toBe(true)
    expect(root.label).toBe('Result')
    expect(root.nReturned).toBe(5)
    expect(root.timeMs).toBe(3)
    expect(root.docsExamined).toBe(200)
    expect(root.keysExamined).toBe(0)
    expect(root.children).toHaveLength(1)
  })

  it('parses a COLLSCAN plan with runtime counts from executionStages', () => {
    const root = buildExplainTree(collscanDoc)
    const scan = root.children[0]
    expect(scan.stage).toBe('COLLSCAN')
    expect(scan.label).toBe('Collection scan')
    expect(scan.nReturned).toBe(5)
    expect(scan.timeMs).toBe(2)
    expect(scan.docsExamined).toBe(200)
    expect(scan.children).toHaveLength(0)
  })

  it('parses IXSCAN→FETCH with index name and nesting', () => {
    const root = buildExplainTree(ixscanFetchDoc)
    const fetch = root.children[0]
    expect(fetch.stage).toBe('FETCH')
    expect(fetch.children).toHaveLength(1)
    const ixscan = fetch.children[0]
    expect(ixscan.stage).toBe('IXSCAN')
    expect(ixscan.label).toBe('Index scan')
    expect(ixscan.indexName).toBe('name_1')
    expect(ixscan.keysExamined).toBe(2)
    expect(ixscan.children).toHaveLength(0)
  })

  it('walks inputStages arrays for branching plans', () => {
    const root = buildExplainTree(orBranchDoc)
    const fetch = root.children[0]
    const or = fetch.children[0]
    expect(or.stage).toBe('OR')
    expect(or.children).toHaveLength(2)
    expect(or.children.map((c) => c.indexName)).toEqual(['a_1', 'b_1'])
    expect(or.children.map((c) => c.stage)).toEqual(['IXSCAN', 'IXSCAN'])
  })

  it('falls back to winningPlan structure with null counts when no executionStats', () => {
    const root = buildExplainTree(plannerOnlyDoc)
    expect(root.isResult).toBe(true)
    expect(root.nReturned).toBeNull()
    expect(root.timeMs).toBeNull()
    const sort = root.children[0]
    expect(sort.stage).toBe('SORT')
    expect(sort.timeMs).toBeNull()
    expect(sort.nReturned).toBeNull()
    const scan = sort.children[0]
    expect(scan.stage).toBe('COLLSCAN')
    expect(scan.children).toHaveLength(0)
  })
})

describe('buildExplainTree — aggregate', () => {
  it('chains pipeline stages in reverse execution order under Result', () => {
    const root = buildExplainTree(aggregateDoc)
    expect(root.isResult).toBe(true)
    // Last stage ($sort) is nearest Result; $cursor/scan is deepest.
    const sort = root.children[0]
    expect(sort.stage).toBe('$sort')
    const group = sort.children[0]
    expect(group.stage).toBe('$group')
    const cursor = group.children[0]
    expect(cursor.stage).toBe('$cursor')
    const scan = cursor.children[0]
    expect(scan.stage).toBe('COLLSCAN')
    expect(scan.children).toHaveLength(0)
  })

  it('carries the final stage output as the Result totals', () => {
    const root = buildExplainTree(aggregateDoc)
    expect(root.nReturned).toBe(5)
  })

  it('parses stage memory from maxUsedMemBytes and sorted-bytes estimate', () => {
    const root = buildExplainTree(aggregateDoc)
    const sort = root.children[0]
    const group = sort.children[0]
    expect(sort.memBytes).toBe(1048576)
    expect(group.memBytes).toBe(2200000)
  })
})

describe('buildExplainTree — memory on a find SORT stage', () => {
  it('reads memUsage as the SORT stage memBytes', () => {
    const root = buildExplainTree(sortMemDoc)
    const sort = root.children[0]
    expect(sort.stage).toBe('SORT')
    expect(sort.memBytes).toBe(3145728)
  })
})

describe('annotateSeverity', () => {
  it('flags a COLLSCAN with good selectivity as warn, not hot', () => {
    const root = buildExplainTree(goodCollscanDoc)
    const scan = root.children[0]
    expect(scan.stage).toBe('COLLSCAN')
    expect(scan.severity).toBe('warn')
  })

  it('flags a dominating-time stage as hot', () => {
    const root = buildExplainTree(slowFetchDoc)
    const fetch = root.children[0]
    expect(fetch.stage).toBe('FETCH')
    expect(fetch.severity).toBe('hot')
    // The fast IXSCAN below it is not flagged.
    expect(fetch.children[0].severity).toBeNull()
  })

  it('leaves a fast, well-indexed tiny plan unflagged', () => {
    const root = buildExplainTree(ixscanFetchDoc)
    const fetch = root.children[0]
    expect(fetch.severity).toBeNull()
    expect(fetch.children[0].severity).toBeNull()
  })

  it('never flags hot when timings are null (planner-only verbosity)', () => {
    const root = buildExplainTree(plannerOnlyDoc)
    const severities = []
    const walk = (n) => { severities.push(n.severity); (n.children || []).forEach(walk) }
    walk(root)
    expect(severities).not.toContain('hot')
  })

  it('is a no-op on a null tree', () => {
    expect(annotateSeverity(null)).toBeNull()
  })
})

describe('buildExplainTree — sharded', () => {
  it('returns a graceful notice instead of throwing', () => {
    expect(() => buildExplainTree(shardedDoc)).not.toThrow()
    const root = buildExplainTree(shardedDoc)
    expect(root.isResult).toBe(true)
    const notice = root.children[0]
    expect(notice.stage).toBe('SHARDED')
    expect(notice.note).toBeTruthy()
    expect(notice.children).toHaveLength(0)
  })

  it('treats a split aggregation pipeline as sharded too', () => {
    const root = buildExplainTree({ splitPipeline: { shardsPart: [], mergerPart: [] } })
    expect(root.children[0].stage).toBe('SHARDED')
  })
})
