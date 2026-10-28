// The single knob for how large the interface is drawn. Every size in the
// app is multiplied by it: in CSS through `var(--px)`, in scripts through
// `scaled()`. At 1 the interface is drawn at its design size.

/** How many times larger than its design size the interface is drawn. */
export const UI_SCALE = 1

/** A length given in design pixels, in actual pixels. */
export function scaled(pixels: number): number {
  return pixels * UI_SCALE
}

/** Publishes the scale as the `--ui-scale` custom property on `root`. */
export function applyScale(root: HTMLElement): void {
  root.style.setProperty('--ui-scale', String(UI_SCALE))
}
