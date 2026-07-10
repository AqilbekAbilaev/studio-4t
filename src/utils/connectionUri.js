// Routing for connection-string query parameters when importing a URI in the New
// Connection dialog. Studio 3T preserves *every* option from an imported string; our
// editor has three destinations for them — the Advanced-tab catalog (known keys), a few
// dedicated fields (read preference, TLS file paths), and a verbatim "extra" bucket for
// anything else — so nothing is silently dropped.

// Params that structural parsing already consumes into their own fields, matched
// case-insensitively so they aren't also treated as passthrough options.
const STRUCTURAL_KEYS = [
  'replicaSet', 'authSource', 'authMechanism',
  'tls', 'ssl', 'tlsAllowInvalidCertificates',
]

/**
 * Split a URI's options into the editor's destinations.
 * @param {URLSearchParams} params - the parsed query string.
 * @param {string[]} knownKeys - catalog keys (KNOWN_OPTION_KEYS) the Advanced tab models.
 * @returns {{ known: Object, extra: Object, readPreference: (string|null),
 *            tlsCaFile: (string|null), tlsCertKeyFile: (string|null), tls: boolean }}
 *   `known` is keyed by canonical catalog key; `extra` keeps unrecognized keys verbatim.
 */
export function partitionUriOptions(params, knownKeys) {
  const knownByLower = new Map(knownKeys.map(key => [key.toLowerCase(), key]))
  const structuralLower = new Set(STRUCTURAL_KEYS.map(key => key.toLowerCase()))

  const result = {
    known: {},
    extra: {},
    readPreference: null,
    tlsCaFile: null,
    tlsCertKeyFile: null,
    tls: false,
  }

  for (const [rawKey, value] of params.entries()) {
    const lower = rawKey.toLowerCase()
    if (structuralLower.has(lower)) continue

    if (lower === 'readpreference') {
      result.readPreference = value
      continue
    }
    if (lower === 'tlscafile' || lower === 'sslcertificateauthorityfile') {
      result.tlsCaFile = value
      result.tls = true
      continue
    }
    if (lower === 'tlscertificatekeyfile' || lower === 'sslpemkeyfile') {
      result.tlsCertKeyFile = value
      result.tls = true
      continue
    }

    const canonical = knownByLower.get(lower)
    if (canonical) {
      result.known[canonical] = value
    } else {
      result.extra[rawKey] = value
    }
  }

  return result
}
