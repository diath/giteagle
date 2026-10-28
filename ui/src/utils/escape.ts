import { onBeforeUnmount, onMounted } from 'vue'

type Handler = () => void

/** Mounted handlers in mount order; Escape goes to the most recent one. */
const handlers: Handler[] = []

window.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape' || event.defaultPrevented || handlers.length === 0) {
    return
  }
  event.preventDefault()
  handlers[handlers.length - 1]()
})

/**
 * Lets the component handle Escape while it is the innermost layer, e.g. a
 * dialog over the branch switcher over the main view.
 */
export function useEscape(handler: Handler): void {
  onMounted(() => handlers.push(handler))
  onBeforeUnmount(() => {
    const index = handlers.lastIndexOf(handler)
    if (index !== -1) {
      handlers.splice(index, 1)
    }
  })
}
