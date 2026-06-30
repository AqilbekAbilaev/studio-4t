// CodeMirror 6 setup for the IntelliShell editor: JS syntax highlighting themed
// with the app's tokens, plus a Mongo-aware autocomplete (collection names after
// `db.`, collection/cursor methods after a member access, shell globals
// otherwise). Kept out of ShellConsole.vue so the component stays focused on
// wiring the editor to the active shell tab.
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, placeholder } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { javascript } from '@codemirror/lang-javascript'
import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete'
import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'

const GLOBALS = ['db', 'print', 'printjson', 'ObjectId', 'ISODate', 'UUID', 'NumberLong', 'NumberInt', 'NumberDecimal']
const DB_METHODS = ['getCollection', 'getCollectionNames', 'runCommand', 'stats', 'createCollection', 'dropDatabase']
const COLL_METHODS = [
  'find', 'findOne', 'insertOne', 'insertMany', 'updateOne', 'updateMany', 'replaceOne',
  'deleteOne', 'deleteMany', 'aggregate', 'countDocuments', 'estimatedDocumentCount', 'distinct',
  'createIndex', 'dropIndex', 'getIndexes', 'drop', 'count', 'bulkWrite',
]
const CURSOR_METHODS = ['limit', 'skip', 'sort', 'projection', 'toArray', 'forEach', 'map', 'count', 'hasNext', 'next']

// `getCollections` is a function returning the current db's collection names (for
// completion after `db.`), kept as a getter so it can populate asynchronously.
function mongoCompletionSource(getCollections) {
  return (context) => {
    // `.member` access — offer methods (or collection names after `db`).
    const member = context.matchBefore(/\.[\w$]*$/)
    if (member) {
      const before = context.state.sliceDoc(0, member.from)
      const idMatch = /([\w$]+)\s*$/.exec(before)
      const object = idMatch ? idMatch[1] : ''
      let options
      if (object === 'db') {
        options = [
          ...getCollections().map((name) => ({ label: name, type: 'class' })),
          ...DB_METHODS.map((name) => ({ label: name, type: 'method' })),
        ]
      } else {
        options = [...COLL_METHODS, ...CURSOR_METHODS].map((name) => ({ label: name, type: 'method' }))
      }
      return { from: member.from + 1, options: options, validFor: /^[\w$]*$/ }
    }

    // Bare identifier — offer shell globals.
    const word = context.matchBefore(/[\w$]+/)
    if (!word || (word.from === word.to && !context.explicit)) return null
    return {
      from: word.from,
      options: GLOBALS.map((name) => ({ label: name, type: 'variable' })),
      validFor: /^[\w$]*$/,
    }
  }
}

// Syntax colors mapped to the app's existing cell/JSON token palette.
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
    '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '1.9', overflow: 'auto' },
    '.cm-content': { padding: '12px 0', caretColor: 'var(--text)' },
    '.cm-gutters': { backgroundColor: 'var(--bg-panel-2)', color: 'var(--text-faint)', border: 'none', borderRight: '1px solid var(--border)' },
    '.cm-lineNumbers .cm-gutterElement': { padding: '0 12px 0 16px' },
    '.cm-activeLine': { backgroundColor: 'rgba(255,255,255,0.03)' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text-dim)' },
    '&.cm-focused .cm-cursor': { borderLeftColor: 'var(--text)' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'rgba(59,130,246,0.30)' },
    '.cm-tooltip': { backgroundColor: 'var(--bg-panel)', border: '1px solid var(--border-soft)', borderRadius: '6px' },
    '.cm-tooltip-autocomplete ul li[aria-selected]': { backgroundColor: 'var(--bg-hover)', color: 'var(--text)' },
  },
  { dark: true }
)

// Build the editor's extension set. `onRun` runs the whole buffer; `onRunLine`
// receives the text of the line under the cursor; `onChange` syncs edits back to
// the tab; `getCollections` feeds db-collection completions.
export function buildExtensions({ onRun, onRunLine, onChange, getCollections }) {
  return [
    lineNumbers(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    drawSelection(),
    history(),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    javascript(),
    placeholder('db.getCollection("myCollection").find({}).limit(20)'),
    syntaxHighlighting(highlightStyle),
    autocompletion({ override: [mongoCompletionSource(getCollections)] }),
    editorTheme,
    keymap.of([
      { key: 'Mod-Enter', preventDefault: true, run: () => { onRun(); return true } },
      {
        key: 'Mod-Shift-Enter',
        preventDefault: true,
        run: (view) => {
          const line = view.state.doc.lineAt(view.state.selection.main.head)
          onRunLine(line.text)
          return true
        },
      },
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...historyKeymap,
      ...completionKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) onChange(update.state.doc.toString())
    }),
  ]
}

export { EditorView, EditorState }
