import { describe, it, expect } from 'vitest'
import { scheduleSummary } from './taskSchedule'

describe('scheduleSummary', () => {
  it('is Manual when there is no schedule', () => {
    expect(scheduleSummary(null)).toBe('Manual')
    expect(scheduleSummary(undefined)).toBe('Manual')
    expect(scheduleSummary({})).toBe('Manual')
  })

  it('summarizes interval schedules in natural units', () => {
    expect(scheduleSummary({ kind: 'interval', every_minutes: 1 })).toBe('Every 1 minute')
    expect(scheduleSummary({ kind: 'interval', every_minutes: 5 })).toBe('Every 5 minutes')
    expect(scheduleSummary({ kind: 'interval', every_minutes: 60 })).toBe('Every 1 hour')
    expect(scheduleSummary({ kind: 'interval', every_minutes: 120 })).toBe('Every 2 hours')
    expect(scheduleSummary({ kind: 'interval', every_minutes: 90 })).toBe('Every 1h 30m')
  })

  it('flags an interval with no minutes set', () => {
    expect(scheduleSummary({ kind: 'interval' })).toBe('Interval (not set)')
    expect(scheduleSummary({ kind: 'interval', every_minutes: 0 })).toBe('Interval (not set)')
  })

  it('summarizes daily schedules', () => {
    expect(scheduleSummary({ kind: 'daily', at_hhmm: '09:00' })).toBe('Daily at 09:00')
    expect(scheduleSummary({ kind: 'daily' })).toBe('Daily (time not set)')
  })

  it('summarizes weekly schedules with the weekday name', () => {
    expect(scheduleSummary({ kind: 'weekly', weekday: 0, at_hhmm: '08:30' }))
      .toBe('Weekly on Sunday at 08:30')
    expect(scheduleSummary({ kind: 'weekly', weekday: 5, at_hhmm: '18:00' }))
      .toBe('Weekly on Friday at 18:00')
  })

  it('flags a weekly schedule missing pieces', () => {
    expect(scheduleSummary({ kind: 'weekly', weekday: 2 })).toBe('Weekly (not set)')
  })
})
