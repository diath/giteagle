import { inject, onBeforeUnmount, onMounted, provide, type InjectionKey } from 'vue'

type Handler = () => void

interface Layer {
  handler: Handler
  /** Whether the layer is on screen; hidden tabs keep their views mounted. */
  shown: () => boolean
}

/** Mounted layers in mount order; Escape goes to the most recent one on screen. */
const layers: Layer[] = []
const SHOWN: InjectionKey<() => boolean> = Symbol('escape scope shown')

window.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape' || event.defaultPrevented) {
    return
  }
  const layer = layers.findLast((candidate) => candidate.shown())
  if (layer) {
    event.preventDefault()
    layer.handler()
  }
})

/**
 * Lets the component handle Escape while it is the innermost layer, e.g. a
 * dialog over the branch switcher over the main view.
 */
export function useEscape(handler: Handler): void {
  const layer: Layer = { handler, shown: inject(SHOWN, () => true) }
  onMounted(() => layers.push(layer))
  onBeforeUnmount(() => {
    const index = layers.lastIndexOf(layer)
    if (index !== -1) {
      layers.splice(index, 1)
    }
  })
}

/** Makes Escape skip the layers below the calling component while `shown` is false. */
export function provideEscapeScope(shown: () => boolean): void {
  provide(SHOWN, shown)
}
