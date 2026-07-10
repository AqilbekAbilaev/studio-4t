import { describe, it, expect } from 'vitest'
import { parseDocumentJson } from './docJson'

describe('parseDocumentJson', () => {
  it('returns the parsed object for a valid JSON object', () => {
    expect(parseDocumentJson('{"a": 1, "b": "x"}')).toEqual({ a: 1, b: 'x' })
  })

  it('throws a friendly message on invalid JSON', () => {
    expect(() => parseDocumentJson('{ not json')).toThrow(/^Invalid JSON:/)
  })

  it('rejects a top-level array', () => {
    expect(() => parseDocumentJson('[1, 2, 3]')).toThrow('Document must be a JSON object')
  })

  it('rejects a top-level null', () => {
    expect(() => parseDocumentJson('null')).toThrow('Document must be a JSON object')
  })

  it('rejects a top-level primitive', () => {
    expect(() => parseDocumentJson('42')).toThrow('Document must be a JSON object')
  })
})
