import { describe, it, expect } from 'vitest'
import { predefinedQuery, selectedRowIndices, hasSelectedDocs } from './predefinedQuery'

describe('selectedRowIndices', () => {
  it('prefers the multi-select array', () => {
    expect(selectedRowIndices({ selectedRows: [1, 3], selectedRow: 0 })).toEqual([1, 3])
  })
  it('falls back to the single active row', () => {
    expect(selectedRowIndices({ selectedRows: [], selectedRow: 2 })).toEqual([2])
  })
  it('is empty when nothing is selected', () => {
    expect(selectedRowIndices({ selectedRows: [], selectedRow: -1 })).toEqual([])
    expect(selectedRowIndices(null)).toEqual([])
  })
})

describe('hasSelectedDocs', () => {
  it('reflects whether any row is selected', () => {
    expect(hasSelectedDocs({ selectedRows: [0] })).toBe(true)
    expect(hasSelectedDocs({ selectedRows: [], selectedRow: -1 })).toBe(false)
  })
})

describe('predefinedQuery', () => {
  it('all → empty document', () => {
    expect(predefinedQuery('all', {})).toBe('{}')
  })

  it('current → the tab filter, or {} when blank', () => {
    expect(predefinedQuery('current', { filter: '{ a: 1 }' })).toBe('{ a: 1 }')
    expect(predefinedQuery('current', { filter: '   ' })).toBe('{}')
    expect(predefinedQuery('current', {})).toBe('{}')
  })

  it('selected → an $in over the selected rows\' _ids as shell types', () => {
    const tab = {
      selectedRows: [0, 1],
      results: [
        { _id: { $oid: 'aaaaaaaaaaaaaaaaaaaaaaaa' } },
        { _id: 42 },
      ],
    }
    const out = predefinedQuery('selected', tab)
    expect(out).toContain('"_id"')
    expect(out).toContain('"$in"')
    expect(out).toContain('ObjectId("aaaaaaaaaaaaaaaaaaaaaaaa")')
    expect(out).toContain('42')
  })

  it('selected → empty $in when nothing is selected', () => {
    expect(predefinedQuery('selected', { selectedRows: [], selectedRow: -1, results: [] }))
      .toBe('{\n  "_id" : {\n    "$in" : []\n  }\n}')
  })
})
