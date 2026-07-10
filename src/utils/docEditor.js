// CodeMirror 6 setup for the pop-out document editor. Mirrors utils/shellEditor.js
// but for a single JSON document: JS syntax highlighting (EJSON reads as JS object
// literals, so @codemirror/lang-javascript colors it well — no new dependency) themed
// with the app's cell/JSON tokens, plus a Save keybinding. No Mongo autocomplete and
// no run keymap — this editor only edits one document's text.
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { javascript } from '@codemirror/lang-javascript'
import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete'
import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'

// Syntax colors mapped to the app's existing cell/JSON token palette (same mapping as
// shellEditor.js so the two editors read identically).
const highlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: 'var(--cell-op)' },
  { tag: [t.string, t.special(t.string)], color: 'var(--cell-str)' },
  { tag: [t.number, t.bool, t.null], color: 'var(--cell-num)' },
  { tag: [t.propertyName, t.attributeName], color: 'var(--cell-key)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: 'var(--link)' },
  { tag: [t.comment], color: 'var(--text-faint)', fontStyle: 'italic' },
  { tag: [t.operator, t.punctuation], color: 'var(--text-dim)' },
  { tag: [t.variableName, t.definition(t.variableName)], color: 'var(--text)' },
])

const editorTheme = EditorView.theme(
  {
    '&': { height: '100%', color: 'var(--text)', backgroundColor: 'transparent', fontSize: '13px' },
    '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '1.7', overflow: 'auto' },
    '.cm-content': { padding: '12px 0', caretColor: 'var(--text)' },
    '.cm-gutters': { backgroundColor: 'var(--bg-panel-2)', color: 'var(--text-faint)', border: 'none', borderRight: '1px solid var(--border)' },
    '.cm-lineNumbers .cm-gutterElement': { padding: '0 12px 0 16px' },
    '.cm-activeLine': { backgroundColor: 'rgba(255,255,255,0.03)' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text-dim)' },
    '&.cm-focused .cm-cursor': { borderLeftColor: 'var(--text)' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'rgba(59,130,246,0.30)' },
  },
  { dark: true }
)

// Build the editor's extension set. `onChange` syncs edits back to the component (for
// the dirty flag); `onSave` runs when the user presses Cmd/Ctrl+S. `readOnly` makes the
// editor a non-editable viewer (used by the read-only "view" window): the document can be
// selected/copied and navigated, but not changed — so no Save keymap or change listener.
export function buildExtensions({ onChange, onSave, readOnly = false }) {
  const base = [
    lineNumbers(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    drawSelection(),
    history(),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    javascript(),
    syntaxHighlighting(highlightStyle),
    editorTheme,
  ]
  if (readOnly) {
    // Read-only but kept focusable (editable stays on) so the caret and selection look
    // identical to the editor; the readOnly facet blocks any actual content change.
    return [
      ...base,
      EditorState.readOnly.of(true),
      keymap.of([...defaultKeymap, ...historyKeymap]),
    ]
  }
  return [
    ...base,
    keymap.of([
      { key: 'Mod-s', preventDefault: true, run: () => { onSave(); return true } },
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...historyKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) onChange(update.state.doc.toString())
    }),
  ]
}

export { EditorView, EditorState }
