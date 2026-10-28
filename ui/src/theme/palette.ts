// The single source of every color in the app. Components read colors only
// through the CSS custom properties applied from here, or through the
// helpers below; raw hex values in components are rejected by ESLint.

/**
 * Greys, darkest to lightest. The dark steps used for backgrounds are only
 * faintly brown; the light steps used for text are warmer. None is pure
 * black or white.
 */
const neutral = {
  950: '#0e0d0c',
  900: '#141210',
  850: '#181614',
  800: '#1e1c19',
  750: '#24211e',
  700: '#2c2925',
  600: '#393430',
  500: '#585047',
  400: '#7b7266',
  300: '#9e9487',
  200: '#c2b8aa',
  100: '#ddd4c6',
  50: '#ece4d7',
} as const

export type AccentName =
  | 'amber'
  | 'green'
  | 'red'
  | 'blue'
  | 'purple'
  | 'teal'
  | 'orange'
  | 'rose'
  | 'yellow'

/**
 * `bright` for text and highlights, `base` for icons and fills, `dim` for
 * subdued text, `muted` for tinted backgrounds.
 */
export type AccentVariant = 'bright' | 'base' | 'dim' | 'muted'

const accents: Record<AccentName, Record<AccentVariant, string>> = {
  amber: { bright: '#f2c078', base: '#dba352', dim: '#a07a43', muted: '#3a2e1f' },
  green: { bright: '#b5d68a', base: '#93ba68', dim: '#6c8a4d', muted: '#252f1c' },
  red: { bright: '#f0998a', base: '#dc7462', dim: '#a1564a', muted: '#3a221e' },
  blue: { bright: '#9cc0e3', base: '#76a1cc', dim: '#58789a', muted: '#1f2a36' },
  purple: { bright: '#cfb0e0', base: '#ae8ec4', dim: '#836b94', muted: '#2d2534' },
  teal: { bright: '#98d6c7', base: '#71b8a8', dim: '#55897e', muted: '#1d302c' },
  orange: { bright: '#f3ac80', base: '#de8a58', dim: '#a66843', muted: '#3a271b' },
  rose: { bright: '#eeaac0', base: '#d38aa3', dim: '#9d677a', muted: '#38232b' },
  yellow: { bright: '#ecdb94', base: '#cfbd6e', dim: '#9a8d53', muted: '#333020' },
}

const roles = {
  bgApp: neutral[900],
  bgSunken: neutral[950],
  bgPanel: neutral[850],
  bgRaised: neutral[800],
  bgHover: neutral[750],
  bgActive: neutral[700],
  border: neutral[700],
  borderSubtle: neutral[750],
  borderStrong: neutral[600],
  fg: neutral[100],
  fgStrong: neutral[50],
  fgMuted: neutral[300],
  fgFaint: neutral[400],
  fgDisabled: neutral[500],
  textOnAccent: neutral[950],
  scrollbar: neutral[600],
  scrollbarHover: neutral[500],
  scrim: 'rgba(12, 10, 8, 0.6)',
  shadow: 'rgba(8, 6, 4, 0.35)',

  accent: accents.amber.base,
  accentBright: accents.amber.bright,
  accentMuted: accents.amber.muted,
  focus: accents.amber.bright,
  hash: accents.teal.bright,
  link: accents.blue.bright,
  success: accents.green.base,
  warning: accents.yellow.base,
  warningMuted: accents.yellow.muted,
  danger: accents.red.base,
  dangerBright: accents.red.bright,
  dangerMuted: accents.red.muted,
  info: accents.blue.base,

  markerUnpushed: accents.amber.base,
  markerPushed: neutral[400],

  statusModified: accents.yellow.base,
  statusStaged: accents.green.base,
  statusAdded: accents.green.bright,
  statusDeleted: accents.red.base,
  statusRenamed: accents.blue.base,
  statusTypeChange: accents.purple.base,
  statusConflicted: accents.orange.bright,

  diffAddedBg: '#1f2a19',
  diffAddedWord: '#36492a',
  diffAddedGutter: '#273520',
  diffAddedSign: accents.green.base,
  diffRemovedBg: '#2d1c19',
  diffRemovedWord: '#52302a',
  diffRemovedGutter: '#382320',
  diffRemovedSign: accents.red.base,
  diffHunkBg: '#1c2026',
  diffHunkFg: accents.blue.dim,
  diffGapBg: neutral[850],

  syntaxComment: neutral[400],
  syntaxString: accents.green.bright,
  syntaxNumber: accents.orange.bright,
  syntaxKeyword: accents.purple.bright,
  syntaxOperator: neutral[300],
  syntaxFunction: accents.blue.bright,
  syntaxType: accents.yellow.bright,
  syntaxVariable: neutral[100],
  syntaxParameter: accents.rose.bright,
  syntaxTag: accents.rose.bright,
  syntaxAttribute: accents.amber.bright,
  syntaxConstant: accents.orange.bright,
  syntaxNamespace: accents.teal.bright,
  syntaxRegex: accents.teal.bright,
  syntaxProperty: accents.blue.base,
  syntaxHeading: accents.amber.bright,
} as const

export type Role = keyof typeof roles

/** Accents used for avatar placeholders, picked by a hash of the email. */
export const avatarAccents: readonly AccentName[] = [
  'amber',
  'green',
  'red',
  'blue',
  'purple',
  'teal',
  'orange',
  'rose',
  'yellow',
]

export function color(role: Role): string {
  return roles[role]
}

export function accent(name: AccentName, variant: AccentVariant = 'base'): string {
  return accents[name][variant]
}

function kebab(name: string): string {
  return name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)
}

/** The CSS variable for a role, e.g. `var(--color-bg-app)`. */
export function cssVar(role: Role): string {
  return `var(--color-${kebab(role)})`
}

/** The CSS variable for an accent, e.g. `var(--accent-amber-bright)`. */
export function accentVar(name: AccentName, variant: AccentVariant = 'base'): string {
  return `var(--accent-${name}-${variant})`
}

/** Publishes the palette as CSS custom properties on `root`. */
export function applyPalette(root: HTMLElement): void {
  for (const [role, value] of Object.entries(roles)) {
    root.style.setProperty(`--color-${kebab(role)}`, value)
  }
  for (const [name, variants] of Object.entries(accents)) {
    for (const [variant, value] of Object.entries(variants)) {
      root.style.setProperty(`--accent-${name}-${variant}`, value)
    }
  }
}
