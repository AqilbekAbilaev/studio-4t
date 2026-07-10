// Shared validation for the document editors (in-app modal + the pop-out window).
// Parses a document editor's text into a plain object, or throws a friendly Error
// whose message is safe to show inline. Kept tiny and pure so it's unit-testable and
// both editors report identical errors.
export function parseDocumentJson(text) {
  let parsed
  try {
    parsed = JSON.parse(text)
  } catch (e) {
    throw new Error(`Invalid JSON: ${e.message}`)
  }
  if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
    throw new Error('Document must be a JSON object')
  }
  return parsed
}
