// Drag-to-resize handler factory, shared by the sidebar (horizontal) and the operations
// dock (vertical). Returns an onMouseDown handler that tracks the drag on document-level
// mousemove/mouseup, clamps the size to [min, max], and toggles a `resizing` ref for cursor
// styling. `axis` selects clientX + col-resize vs clientY + row-resize; `invert` is for the
// bottom dock, where dragging up (a smaller clientY) grows the pane.
export function makeResizer({ value, resizing, min, max, axis, invert = false }) {
  const horizontal = axis === 'x'
  return function onMouseDown(e) {
    e.preventDefault()
    const start = horizontal ? e.clientX : e.clientY
    const startVal = value.value
    resizing.value = true
    const onMove = (ev) => {
      const cur = horizontal ? ev.clientX : ev.clientY
      const delta = invert ? (start - cur) : (cur - start)
      value.value = Math.max(min, Math.min(max, startVal + delta))
    }
    const onUp = () => {
      resizing.value = false
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }
    document.body.style.cursor = horizontal ? 'col-resize' : 'row-resize'
    document.body.style.userSelect = 'none'
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }
}
