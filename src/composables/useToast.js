import { inject } from 'vue'

// The app-wide toast function, provided once by App.vue. Any component that needs to
// surface a transient message calls showToast(message) directly instead of emitting a
// `toast` event and relying on every ancestor to relay it. Falls back to a no-op so a
// component rendered outside the main window (which has no provider) never crashes.
export function useToast() {
  const showToast = inject('showToast', () => {})
  return { showToast: showToast }
}
