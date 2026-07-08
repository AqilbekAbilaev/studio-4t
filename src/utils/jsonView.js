// Read-only CodeMirror 6 viewer for the results JSON view. CodeMirror virtualizes
// rendering (only on-screen lines exist in the DOM) and folds objects/arrays
// natively via the JS language's syntax tree — which is why the JSON view uses it
// instead of a hand-rolled row list, which mounted every line at once and re-diffed
// the whole list on each fold. The buffer is each document rendered by mongoStringify
// at the top level (no enclosing array), one after another with a divider drawn
// between them; the JS parser still folds every document, sub-object and array.
import { EditorView, Decoration, lineNumbers } from '@codemirror/view'
import { EditorState, StateField, RangeSetBuilder } from '@codemirror/state'
import { javascript } from '@codemirror/lang-javascript'
import { foldGutter, codeFolding, syntaxHighlighting, HighlightStyle } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'

// Divider between documents. Each result document is a top-level object, so its first
// line is exactly "{" (nested and array-element braces are indented). Draw a divider
// above every document except the first. Marking the start line — rather than the
// closing brace — keeps the divider visible even when the preceding document is folded
// down to a single line.
const docDivider = Decoration.line({ class: 'cm-doc-divider' })

function buildDividers(state) {
  const builder = new RangeSetBuilder()
  let seenFirst = false
  for (let i = 1; i <= state.doc.lines; i++) {
    const line = state.doc.line(i)
    if (line.text !== '{') continue
    if (seenFirst) builder.add(line.from, line.from, docDivider)
    else seenFirst = true
  }
  return builder.finish()
}

const docDividerField = StateField.define({
  create: buildDividers,
  update(value, tr) {
    if (tr.docChanged) return buildDividers(tr.state)
    return value
  },
  provide: (field) => EditorView.decorations.from(field),
})

// Token colors mapped to the app's JSON palette — the same tokens the old regex
// highlighter used (keys, strings, numbers, null faint, ObjectId()/ISODate() as
// function calls in the link color).
const jsonHighlightStyle = HighlightStyle.define([
  { tag: [t.propertyName, t.attributeName],                     color: 'var(--cell-key)' },
  { tag: [t.string, t.special(t.string)],                       color: 'var(--cell-str)' },
  { tag: [t.number, t.bool],                                    color: 'var(--cell-num)' },
  { tag: t.null,                                                color: 'var(--text-faint)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: 'var(--link)' },
  { tag: [t.punctuation, t.separator, t.operator],              color: 'var(--text-dim)' },
])

const jsonTheme = EditorView.theme({
  '&':                                { height: '100%', color: 'var(--text)', backgroundColor: 'transparent' },
  '.cm-scroller':                     { fontFamily: 'var(--mono)', fontSize: '12.5px', lineHeight: '1.5', overflow: 'auto' },
  '.cm-content':                      { padding: '10px 0' },
  // The app's global reset (theme.css) sets `user-select: none` on every element,
  // which would otherwise make the read-only content unselectable. Re-enable it on
  // the code content (and its highlight token spans) so text can be selected/copied
  // — the gutter is left untouched so copies omit the line numbers.
  '.cm-content, .cm-content *':       { userSelect: 'text', WebkitUserSelect: 'text' },
  '.cm-gutters':                      { backgroundColor: 'transparent', color: 'var(--text-faint)', border: 'none' },
  '.cm-lineNumbers .cm-gutterElement':{ padding: '0 6px 0 14px', minWidth: '34px' },
  '.cm-foldGutter .cm-gutterElement': { padding: '0 4px', cursor: 'pointer', color: 'var(--text-faint)' },
  '.cm-foldPlaceholder':              { backgroundColor: 'transparent', border: 'none', color: 'var(--text-faint)', padding: '0 4px' },
  '.cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'rgba(59,130,246,0.30)' },
  // Divider drawn above each document after the first (see docDividerField).
  '.cm-doc-divider':                  { borderTop: '1px solid var(--border-soft)' },
})

/** Create a read-only JSON viewer mounted into `parent`, showing `doc`. */
export function createJsonView(parent, doc) {
  const state = EditorState.create({
    doc: doc,
    extensions: [
      lineNumbers(),
      codeFolding(),
      foldGutter(),
      javascript(),
      syntaxHighlighting(jsonHighlightStyle),
      docDividerField,
      jsonTheme,
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
    ],
  })
  return new EditorView({ state: state, parent: parent })
}

/** Replace the viewer's whole buffer (also clears any folds, which are positional). */
export function setJsonView(view, doc) {
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: doc } })
}
