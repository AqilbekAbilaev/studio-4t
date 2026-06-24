// WebKitGTK (the Linux Tauri webview) doesn't implement native undo/redo for
// <input>/<textarea> — copy/paste/cut/select-all work, but Ctrl+Z does nothing. Our
// reactive fields are also "controlled", which defeats native undo on other engines
// too. This installs a lightweight per-element history so Ctrl+Z / Ctrl+Shift+Z /
// Ctrl+Y work in every text field. Call installInputUndo() once when the app mounts.

const COALESCE_MS = 350
const TEXT_INPUT_TYPES = new Set([
  'text', 'search', 'url', 'email', 'tel', 'password', 'number', '',
])

function isTextField(el) {
  if (!el) return false
  if (el.tagName === 'TEXTAREA') return true
  if (el.tagName === 'INPUT') {
    return TEXT_INPUT_TYPES.has((el.getAttribute('type') || '').toLowerCase())
  }
  return false
}

export function installInputUndo() {
  const histories = new WeakMap()
  let applying = false

  const snapshot = (el) => ({ value: el.value, start: el.selectionStart, end: el.selectionEnd })

  function historyFor(el) {
    let h = histories.get(el)
    if (!h) {
      h = { undo: [snapshot(el)], redo: [], last: 0 }
      histories.set(el, h)
    }
    return h
  }

  // Restore a snapshot and let Vue's @input / v-model handlers sync their state by
  // dispatching a real input event (guarded so it doesn't re-enter our own recorder).
  function apply(el, snap) {
    applying = true
    el.value = snap.value
    el.dispatchEvent(new Event('input', { bubbles: true }))
    applying = false
    try {
      el.setSelectionRange(snap.start, snap.end)
    } catch (_) {
      // some input types (e.g. number) don't support selection ranges
    }
  }

  // Capture the pre-edit value when a field gains focus, so the first undo can return
  // to it (the input event fires only after the value has already changed).
  function onFocusIn(e) {
    if (isTextField(e.target)) historyFor(e.target)
  }

  function onInput(e) {
    if (applying) return
    const el = e.target
    if (!isTextField(el)) return
    const h = historyFor(el)
    const now = Date.now()
    // Coalesce a fast burst of keystrokes into a single undo step.
    if (now - h.last < COALESCE_MS && h.undo.length) {
      h.undo[h.undo.length - 1] = snapshot(el)
    } else {
      h.undo.push(snapshot(el))
    }
    h.redo.length = 0
    h.last = now
  }

  function onKeydown(e) {
    if (!(e.ctrlKey || e.metaKey)) return
    const el = document.activeElement
    if (!isTextField(el)) return
    const key = e.key.toLowerCase()
    const isUndo = key === 'z' && !e.shiftKey
    const isRedo = (key === 'z' && e.shiftKey) || key === 'y'
    if (!isUndo && !isRedo) return
    const h = historyFor(el)
    if (isUndo) {
      if (h.undo.length < 2) return  // only the baseline left — nothing to undo
      const current = h.undo.pop()
      h.redo.push(current)
      apply(el, h.undo[h.undo.length - 1])
    } else {
      if (!h.redo.length) return
      const next = h.redo.pop()
      h.undo.push(next)
      apply(el, next)
    }
    e.preventDefault()
  }

  document.addEventListener('focusin', onFocusIn, true)
  document.addEventListener('input', onInput, true)
  document.addEventListener('keydown', onKeydown, true)

  return () => {
    document.removeEventListener('focusin', onFocusIn, true)
    document.removeEventListener('input', onInput, true)
    document.removeEventListener('keydown', onKeydown, true)
  }
}
