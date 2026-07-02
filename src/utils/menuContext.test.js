import { describe, it, expect } from 'vitest'
import { deriveMenuContext, resolveMenuTarget } from './menuContext'

// These lock the fix that made the native menu usable: context is the union of the
// active tab and the sidebar/tree selection, so selecting a node in the tree
// enables the matching items even while the context-less Quickstart tab is active.

const quickstart = { id: 't0', kind: 'quickstart', title: 'Quickstart' }
const collectionTab = {
  id: 't1', kind: 'collection',
  connectionId: 'c1', connectionName: 'Local', dbName: 'shop', collectionName: 'orders',
}

describe('deriveMenuContext', () => {
  it('is all-false with no tab, no selection, no connections', () => {
    expect(deriveMenuContext(null, null, 0)).toEqual({
      hasConnection: false, hasDatabase: false, hasCollection: false, anyConnection: false,
    })
  })

  it('Quickstart active with no selection gates everything off', () => {
    const ctx = deriveMenuContext(quickstart, null, 0)
    expect(ctx.hasConnection).toBe(false)
    expect(ctx.hasDatabase).toBe(false)
    expect(ctx.hasCollection).toBe(false)
  })

  it('a collection selected in the sidebar enables all three, even on Quickstart', () => {
    const sel = { connectionId: 'c1', connectionName: 'Local', dbName: 'shop', collectionName: 'orders', kind: 'collection' }
    const ctx = deriveMenuContext(quickstart, sel, 1)
    expect(ctx.hasConnection).toBe(true)
    expect(ctx.hasDatabase).toBe(true)
    expect(ctx.hasCollection).toBe(true)
  })

  it('a database selected in the sidebar enables connection + database only', () => {
    const sel = { connectionId: 'c1', connectionName: 'Local', dbName: 'shop', collectionName: null, kind: 'database' }
    const ctx = deriveMenuContext(quickstart, sel, 1)
    expect(ctx.hasConnection).toBe(true)
    expect(ctx.hasDatabase).toBe(true)
    expect(ctx.hasCollection).toBe(false)
  })

  it('a connection selected in the sidebar enables connection only', () => {
    const sel = { connectionId: 'c1', connectionName: 'Local', dbName: null, collectionName: null, kind: 'connection' }
    const ctx = deriveMenuContext(quickstart, sel, 1)
    expect(ctx.hasConnection).toBe(true)
    expect(ctx.hasDatabase).toBe(false)
    expect(ctx.hasCollection).toBe(false)
  })

  it('an active collection tab still satisfies all three (no regression)', () => {
    const ctx = deriveMenuContext(collectionTab, null, 1)
    expect(ctx.hasConnection).toBe(true)
    expect(ctx.hasDatabase).toBe(true)
    expect(ctx.hasCollection).toBe(true)
  })

  it('anyConnection tracks open connections, for Refresh (no active-tab connection needed)', () => {
    expect(deriveMenuContext(quickstart, null, 0).anyConnection).toBe(false)
    expect(deriveMenuContext(quickstart, null, 2).anyConnection).toBe(true)
    // Also true purely from the active tab's connection.
    expect(deriveMenuContext(collectionTab, null, 0).anyConnection).toBe(true)
  })
})

describe('resolveMenuTarget', () => {
  it('prefers the sidebar selection over the active tab', () => {
    const sel = { connectionId: 'c2', connectionName: 'Prod', dbName: 'analytics', collectionName: 'events', kind: 'collection' }
    expect(resolveMenuTarget(collectionTab, sel)).toEqual({
      connectionId: 'c2', connectionName: 'Prod', dbName: 'analytics', collectionName: 'events', kind: 'collection',
    })
  })

  it('derives kind from the deepest field present', () => {
    const dbSel = { connectionId: 'c1', connectionName: 'Local', dbName: 'shop', collectionName: null }
    expect(resolveMenuTarget(null, dbSel).kind).toBe('database')
    const connSel = { connectionId: 'c1', connectionName: 'Local', dbName: null, collectionName: null }
    expect(resolveMenuTarget(null, connSel).kind).toBe('connection')
  })

  it('falls back to the active tab when nothing is selected', () => {
    expect(resolveMenuTarget(collectionTab, null)).toBe(collectionTab)
  })

  it('returns null when neither a selection nor a tab is available', () => {
    expect(resolveMenuTarget(null, null)).toBe(null)
  })
})
