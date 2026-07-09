// Colour-tag palette + resolution shared by the tab bar and the workspace frame,
// so a tab's colour and the workspace outline it drives can never drift apart.
// Matches the tree's Choose Color submenu.
export const TAG_COLORS = {
  blue:   '#3b82f6',
  green:  '#4caf78',
  purple: '#b07ddb',
  red:    '#e07a6b',
  orange: '#e0a35e',
}

export function colorHex(name) {
  return TAG_COLORS[name] || null
}

// The colour name in effect for a tab: a colour set on the tab itself wins;
// otherwise the tab takes its node's colour using the same own-colour-first
// cascade as the tree — check the collection, then the database, then the
// connection, and use the first that has a colour. Returns null when nothing set.
export function tabColorName(tab, tagOverrides) {
  if (!tab) return null
  if (tab.color) return tab.color
  const keys = []
  if (tab.kind === 'collection') {
    keys.push(`${tab.connectionId}/${tab.dbName}/${tab.collectionName}`)
    keys.push(`${tab.connectionId}/${tab.dbName}`)
    keys.push(tab.connectionId)
  } else if (tab.kind === 'shell') {
    keys.push(`${tab.connectionId}/${tab.dbName}`)
    keys.push(tab.connectionId)
  }
  for (const key of keys) {
    const name = tagOverrides ? tagOverrides[key] : null
    if (name && name !== 'none') return name
  }
  return null
}
