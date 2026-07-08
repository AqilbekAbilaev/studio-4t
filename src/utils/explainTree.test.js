import { describe, it, expect } from 'vitest'
import { buildExplainTree, labelForStage } from './explainTree'

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

describe('labelForStage', () => {
  it('maps known stages to friendly names', () => {
    expect(labelForStage('COLLSCAN')).toBe('Collection scan')
    expect(labelForStage('IXSCAN')).toBe('Index scan')
    expect(labelForStage('PROJECTION_COVERED')).toBe('Projection')
    expect(labelForStage('SORT_MERGE')).toBe('Sort merge')
  })
  it('falls back to the raw stage string when unmapped', () => {
    expect(labelForStage('MYSTERY_STAGE')).toBe('MYSTERY_STAGE')
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
