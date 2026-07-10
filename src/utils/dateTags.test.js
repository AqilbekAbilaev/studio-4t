import { describe, it, expect } from 'vitest'
import { expandDateTags } from './dateTags'

// A fixed reference "now": Wed 2026-07-08 14:30 local time.
const NOW = new Date(2026, 6, 8, 14, 30, 0)

function iso(y, m, d) {
  return new Date(y, m, d).toISOString()
}

describe('expandDateTags', () => {
  it('leaves a string without tags unchanged', () => {
    const src = '{ status: "active", n: 5 }'
    expect(expandDateTags(src, NOW)).toBe(src)
  })

  it('expands #today to local midnight as an ISODate', () => {
    const out = expandDateTags('{ ts: { $gte: #today } }', NOW)
    expect(out).toBe(`{ ts: { $gte: ISODate("${iso(2026, 6, 8)}") } }`)
  })

  it('expands #yesterday and #tomorrow', () => {
    expect(expandDateTags('#yesterday', NOW)).toBe(`ISODate("${iso(2026, 6, 7)}")`)
    expect(expandDateTags('#tomorrow', NOW)).toBe(`ISODate("${iso(2026, 6, 9)}")`)
  })

  it('expands #startOfWeek to Monday (Wed reference → Mon the 6th)', () => {
    expect(expandDateTags('#startOfWeek', NOW)).toBe(`ISODate("${iso(2026, 6, 6)}")`)
  })

  it('expands #startOfMonth and #startOfYear', () => {
    expect(expandDateTags('#startOfMonth', NOW)).toBe(`ISODate("${iso(2026, 6, 1)}")`)
    expect(expandDateTags('#startOfYear', NOW)).toBe(`ISODate("${iso(2026, 0, 1)}")`)
  })

  it('expands relative windows from now', () => {
    const sevenAgo = new Date(NOW.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
    expect(expandDateTags('#last7days', NOW)).toBe(`ISODate("${sevenAgo}")`)
  })

  it('is case-insensitive', () => {
    expect(expandDateTags('#TODAY', NOW)).toBe(`ISODate("${iso(2026, 6, 8)}")`)
  })

  it('does NOT touch a tag inside a double-quoted string', () => {
    const src = '{ note: "meeting #today at noon" }'
    expect(expandDateTags(src, NOW)).toBe(src)
  })

  it('does NOT touch a tag inside a single-quoted string', () => {
    const src = "{ note: 'ping #last7days' }"
    expect(expandDateTags(src, NOW)).toBe(src)
  })

  it('expands outside a string but not inside on the same input', () => {
    const out = expandDateTags('{ tag: "#today", ts: { $lt: #today } }', NOW)
    expect(out).toBe(`{ tag: "#today", ts: { $lt: ISODate("${iso(2026, 6, 8)}") } }`)
  })

  it('requires a word boundary (does not match a prefix of a longer word)', () => {
    // "#todays" is not a known tag, and #today must not match the "#today" prefix.
    expect(expandDateTags('#todays', NOW)).toBe('#todays')
  })

  it('leaves an unknown tag untouched', () => {
    expect(expandDateTags('#nextquarter', NOW)).toBe('#nextquarter')
  })

  it('matches the longest token (#last12months, not a shorter prefix)', () => {
    const out = expandDateTags('#last12months', NOW)
    expect(out).toBe(`ISODate("${iso(2025, 6, 8)}")`)
  })

  it('respects escaped quotes inside a string', () => {
    const src = '{ a: "he said \\"#today\\"", b: #today }'
    const out = expandDateTags(src, NOW)
    expect(out).toBe(`{ a: "he said \\"#today\\"", b: ISODate("${iso(2026, 6, 8)}") }`)
  })
})
