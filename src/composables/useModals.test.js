import { describe, it, expect } from 'vitest'
import { useModals } from './useModals'

// The registry-driven modal API (openModal / closeModal / isModalOpen over a single
// `openModals` map) is what lets AppModals render every modal from one v-for and lets
// a new modal be added as one registry row. These specs pin that behaviour.
describe('useModals — registry-driven open-state', () => {
  it('starts with no registry modal open', () => {
    const modals = useModals()
    expect(modals.isModalOpen('stats')).toBe(false)
    expect(Object.keys(modals.openModals)).toHaveLength(0)
  })

  it('openModal records the payload under the modal id', () => {
    const modals = useModals()
    const payload = { connId: 'c1', connName: 'Local', dbName: 'app', collName: 'users' }
    modals.openModal('stats', payload)
    expect(modals.isModalOpen('stats')).toBe(true)
    expect(modals.openModals.stats).toEqual(payload)
  })

  it('openModal with no payload opens with an empty context', () => {
    const modals = useModals()
    modals.openModal('preferences')
    expect(modals.isModalOpen('preferences')).toBe(true)
    expect(modals.openModals.preferences).toEqual({})
  })

  it('closeModal removes the modal so it renders no more', () => {
    const modals = useModals()
    modals.openModal('stats', { connId: 'c1' })
    modals.closeModal('stats')
    expect(modals.isModalOpen('stats')).toBe(false)
    expect('stats' in modals.openModals).toBe(false)
  })

  it('tracks several modals independently', () => {
    const modals = useModals()
    modals.openModal('stats', { connId: 'a' })
    modals.openModal('schema', { connId: 'b' })
    modals.closeModal('stats')
    expect(modals.isModalOpen('stats')).toBe(false)
    expect(modals.isModalOpen('schema')).toBe(true)
  })
})
