// Dynamic date tags: reusable `#tokens` that expand to a concrete date at query time,
// like Studio-3T's date tags. You write e.g. `{ createdAt: { $gte: #last7days } }` and the
// tag becomes `ISODate("…")` before the query is parsed, so a saved query always means
// "the last 7 days" relative to when it runs — no hand-editing dates.
//
// Expansion is string-aware: a `#` inside a quoted string value is left untouched, so a
// filter like `{ note: "call #today" }` is never corrupted. Boundaries (day/week/month/
// year) use local time; relative windows count back from now.

// Each tag: token (without the leading #), a label + hint for the help palette, and a
// resolver that returns the Date it expands to. Longest tokens are matched first so
// `#last12months` wins over any shorter prefix.
export const DATE_TAGS = [
  { token: 'now',          label: '#now',          hint: 'Current date & time' },
  { token: 'today',        label: '#today',        hint: 'Start of today (local midnight)' },
  { token: 'yesterday',    label: '#yesterday',    hint: 'Start of yesterday' },
  { token: 'tomorrow',     label: '#tomorrow',     hint: 'Start of tomorrow' },
  { token: 'startOfWeek',  label: '#startOfWeek',  hint: 'Monday 00:00 this week' },
  { token: 'startOfMonth', label: '#startOfMonth', hint: 'First day of this month' },
  { token: 'startOfYear',  label: '#startOfYear',  hint: 'January 1st this year' },
  { token: 'lastHour',     label: '#lastHour',     hint: 'One hour ago' },
  { token: 'last24hours',  label: '#last24hours',  hint: '24 hours ago' },
  { token: 'last7days',    label: '#last7days',    hint: '7 days ago' },
  { token: 'last30days',   label: '#last30days',   hint: '30 days ago' },
  { token: 'last90days',   label: '#last90days',   hint: '90 days ago' },
  { token: 'last12months', label: '#last12months', hint: '12 months ago' },
]

function startOfDay(date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}
function addDays(date, days) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days)
}

// token (lowercased) → Date, given the reference "now".
function resolveToken(token, now) {
  switch (token) {
    case 'now':          return now
    case 'today':        return startOfDay(now)
    case 'yesterday':    return addDays(startOfDay(now), -1)
    case 'tomorrow':     return addDays(startOfDay(now), 1)
    case 'startofweek': {
      // Monday as the first day of the week (getDay(): Sun=0 … Sat=6).
      const offset = (now.getDay() + 6) % 7
      return addDays(startOfDay(now), -offset)
    }
    case 'startofmonth': return new Date(now.getFullYear(), now.getMonth(), 1)
    case 'startofyear':  return new Date(now.getFullYear(), 0, 1)
    case 'lasthour':     return new Date(now.getTime() - 60 * 60 * 1000)
    case 'last24hours':  return new Date(now.getTime() - 24 * 60 * 60 * 1000)
    case 'last7days':    return new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000)
    case 'last30days':   return new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000)
    case 'last90days':   return new Date(now.getTime() - 90 * 24 * 60 * 60 * 1000)
    case 'last12months': return new Date(now.getFullYear() - 1, now.getMonth(), now.getDate())
    default:             return null
  }
}

// Tokens lowercased and sorted longest-first, so matching is greedy and case-insensitive.
const TOKENS_BY_LENGTH = DATE_TAGS
  .map(tag => tag.token.toLowerCase())
  .sort((a, b) => b.length - a.length)

function isIdentChar(ch) {
  return /[A-Za-z0-9]/.test(ch)
}

// At `rest` (which begins with '#'), return the matched token and the total match length
// (including '#'), or null. A match must be followed by a non-identifier boundary so
// `#today` matches in `#today}` but `#todays` does not match `#today`.
function matchAt(rest) {
  const afterHash = rest.slice(1).toLowerCase()
  for (const token of TOKENS_BY_LENGTH) {
    if (afterHash.startsWith(token)) {
      const boundary = rest[1 + token.length]
      if (boundary === undefined || !isIdentChar(boundary)) {
        return { token: token, length: 1 + token.length }
      }
    }
  }
  return null
}

/**
 * Replace every date tag outside a string literal with `ISODate("<iso>")`.
 * @param {string} source - a query field or pipeline in shell syntax.
 * @param {Date} [now] - reference "now" (injectable for tests).
 * @returns {string} the source with tags expanded (unchanged if it has no tags).
 */
export function expandDateTags(source, now = new Date()) {
  if (!source || source.indexOf('#') === -1) return source

  let out = ''
  let i = 0
  let quote = null // the open quote char while inside a string, else null

  while (i < source.length) {
    const ch = source[i]

    if (quote) {
      out += ch
      if (ch === '\\' && i + 1 < source.length) {
        out += source[i + 1]
        i += 2
        continue
      }
      if (ch === quote) quote = null
      i += 1
      continue
    }

    if (ch === '"' || ch === "'") {
      quote = ch
      out += ch
      i += 1
      continue
    }

    if (ch === '#') {
      const match = matchAt(source.slice(i))
      if (match) {
        const date = resolveToken(match.token, now)
        out += `ISODate("${date.toISOString()}")`
        i += match.length
        continue
      }
    }

    out += ch
    i += 1
  }

  return out
}
