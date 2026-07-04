import { describe, it, expect } from 'vitest'
import { isProtectedIndex, indexKeyLabel, indexSpecJson, isIndexHidden } from './indexSpec'

describe('isProtectedIndex', () => {
  it('protects only the _id_ index', () => {
    expect(isProtectedIndex('_id_')).toBe(true)
    expect(isProtectedIndex('email_1')).toBe(false)
    expect(isProtectedIndex('_id_2')).toBe(false)
    expect(isProtectedIndex('user_id_1')).toBe(false)
    expect(isProtectedIndex('')).toBe(false)
  })
})

describe('indexKeyLabel', () => {
  it('renders a single-field key', () => {
    expect(indexKeyLabel({ key: { email: 1 } })).toBe('email: 1')
  })

  it('renders a compound key in order with directions', () => {
    expect(indexKeyLabel({ key: { name: 1, age: -1 } })).toBe('name: 1, age: -1')
  })

  it('returns an empty string for a missing or malformed key', () => {
    expect(indexKeyLabel(null)).toBe('')
    expect(indexKeyLabel({})).toBe('')
    expect(indexKeyLabel({ key: 'nope' })).toBe('')
  })
})

describe('indexSpecJson', () => {
  it('serializes the full index definition as pretty JSON', () => {
    const spec = { v: 2, key: { email: 1 }, name: 'email_1', unique: true }
    const out = indexSpecJson(spec)
    expect(JSON.parse(out)).toEqual(spec)
    expect(out).toContain('\n') // pretty-printed
  })

  it('handles a nullish index', () => {
    expect(indexSpecJson(null)).toBe('{}')
  })
})

describe('isIndexHidden', () => {
  it('reflects the hidden flag', () => {
    expect(isIndexHidden({ name: 'a', hidden: true })).toBe(true)
    expect(isIndexHidden({ name: 'a', hidden: false })).toBe(false)
    expect(isIndexHidden({ name: 'a' })).toBe(false)
    expect(isIndexHidden(null)).toBe(false)
  })
})
