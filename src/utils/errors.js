// Tauri commands reject with a serialized AppError, now shaped as
// { code, message } (see src-tauri/src/error.rs). These helpers read that shape
// while staying robust to a plain string (older payloads) or a thrown JS Error,
// so a single call site works for any rejection value.

export function errMessage(e) {
  if (e == null) return 'Unknown error'
  if (typeof e === 'string') return e
  if (typeof e.message === 'string') return e.message
  return String(e)
}

export function errCode(e) {
  if (e && typeof e === 'object' && typeof e.code === 'string') return e.code
  return null
}
