// The single knob for how large the interface is drawn, set by the font size
// setting. Every size in the app is multiplied by it: in CSS through
// `var(--px)`, in scripts through `scaled()`. At 1 the interface is drawn at
// its design size.

import type { FontSize } from '../api'

/** How many times larger than its design size each font size draws the interface. */
export const FONT_SIZE_SCALES: Record<FontSize, number> = {
  small: 0.75,
  normal: 1,
  large: 1.25,
  extraLarge: 1.5,
}

let scale = 1

/** A length given in design pixels, in actual pixels at the current font size. */
export function scaled(pixels: number): number {
  return pixels * scale
}

/** Draws the interface at `fontSize`, through the `--ui-scale` custom property on `root`. */
export function applyFontSize(root: HTMLElement, fontSize: FontSize): void {
  // A hand-edited settings file can name a size that doesn't exist.
  scale = FONT_SIZE_SCALES[fontSize] ?? 1
  root.style.setProperty('--ui-scale', String(scale))
}
