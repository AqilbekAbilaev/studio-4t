// Shared formatting for MongoDB result values: a shell-style stringifier that
// renders EJSON wrappers (e.g. {"$oid": "..."}) the way mongosh does, plus a
// JSON syntax highlighter that wraps tokens in spans for theming. Used by both
// the results panel (JSON view / Explain) and the IntelliShell console.

/** Render a value as mongosh-style text — `{"$oid": "…"}` becomes `ObjectId("…")`. */
export function mongoStringify(value, indent = '') {
  if (value === null) return 'null'
  if (Array.isArray(value)) {
    if (!value.length) return '[]'
    const inner = indent + '  '
    const items = value.map((v) => inner + mongoStringify(v, inner))
    return '[\n' + items.join(',\n') + '\n' + indent + ']'
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value)
    if (keys.length === 1 && keys[0] === '$oid' && typeof value.$oid === 'string') {
      return `ObjectId("${value.$oid}")`
    }
    if (!keys.length) return '{}'
    const inner = indent + '  '
    const items = keys.map((k) => `${inner}${JSON.stringify(k)} : ${mongoStringify(value[k], inner)}`)
    return '{\n' + items.join(',\n') + '\n' + indent + '}'
  }
  return JSON.stringify(value)
}

/** Wrap JSON tokens in <span> elements (classes jk/jop/js/jn/jb/jl/joid) for theming. */
export function syntaxHighlight(json) {
  return json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(
      /(ObjectId\("[0-9a-fA-F]{24}"\)|"(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        if (match.startsWith('ObjectId(')) return `<span class="joid">${match}</span>`
        if (match[0] === '"') {
          if (/:$/.test(match)) {
            return match[1] === '$'
              ? `<span class="jop">${match}</span>`
              : `<span class="jk">${match}</span>`
          }
          return `<span class="js">${match}</span>`
        }
        if (match === 'true' || match === 'false') return `<span class="jb">${match}</span>`
        if (match === 'null') return `<span class="jl">${match}</span>`
        return `<span class="jn">${match}</span>`
      }
    )
}
