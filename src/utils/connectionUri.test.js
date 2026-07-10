import { describe, it, expect } from 'vitest'
import { partitionUriOptions } from './connectionUri'

const KNOWN = ['retryWrites', 'w', 'appName', 'socketTimeoutMS', 'readConcernLevel']

function partition(query) {
  return partitionUriOptions(new URLSearchParams(query), KNOWN)
}

describe('partitionUriOptions', () => {
  it('routes catalog keys to `known`', () => {
    const out = partition('retryWrites=true&w=majority&socketTimeoutMS=5000')
    expect(out.known).toEqual({ retryWrites: 'true', w: 'majority', socketTimeoutMS: '5000' })
    expect(out.extra).toEqual({})
  })

  it('matches catalog keys case-insensitively but keeps the canonical key', () => {
    const out = partition('retrywrites=false&APPNAME=svc')
    expect(out.known).toEqual({ retryWrites: 'false', appName: 'svc' })
  })

  it('keeps unrecognized keys verbatim in `extra`', () => {
    const out = partition('directConnection=true&compressors=zstd')
    expect(out.extra).toEqual({ directConnection: 'true', compressors: 'zstd' })
  })

  it('pulls read preference into its dedicated field', () => {
    const out = partition('readPreference=secondaryPreferred')
    expect(out.readPreference).toBe('secondaryPreferred')
    expect(out.known.readPreference).toBeUndefined()
    expect(out.extra.readPreference).toBeUndefined()
  })

  it('pulls TLS file paths into dedicated fields and enables TLS', () => {
    const out = partition('tlsCAFile=/etc/ca.pem&tlsCertificateKeyFile=/etc/client.pem')
    expect(out.tlsCaFile).toBe('/etc/ca.pem')
    expect(out.tlsCertKeyFile).toBe('/etc/client.pem')
    expect(out.tls).toBe(true)
  })

  it('accepts the legacy ssl* file aliases', () => {
    const out = partition('sslCertificateAuthorityFile=/ca.pem&sslPEMKeyFile=/client.pem')
    expect(out.tlsCaFile).toBe('/ca.pem')
    expect(out.tlsCertKeyFile).toBe('/client.pem')
  })

  it('skips structural keys handled elsewhere', () => {
    const out = partition('replicaSet=rs0&authSource=admin&authMechanism=SCRAM-SHA-256&tls=true')
    expect(out.known).toEqual({})
    expect(out.extra).toEqual({})
  })
})
