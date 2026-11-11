// The single source of every color in the app, in each of its themes.
// Components read colors only through the CSS custom properties applied
// from here; raw hex values in components are rejected by ESLint.

import type { ThemeName } from '../api'

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
 * subdued text, `muted` for tinted backgrounds. From `bright` to `muted` they
 * stand out less and less from the background.
 */
export type AccentVariant = 'bright' | 'base' | 'dim' | 'muted'

type Accents = Record<AccentName, Record<AccentVariant, string>>

type NeutralStep = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11

/** The colors of a theme that are neither a grey nor an accent. */
interface Extras {
  scrim: string
  shadow: string
  /** The background of the uncommitted changes row under the pointer. */
  pendingHover: string
  diffAddedBg: string
  diffAddedWord: string
  diffAddedGutter: string
  diffRemovedBg: string
  diffRemovedWord: string
  diffRemovedGutter: string
  diffHunkBg: string
}

interface Theme {
  /** Tells the platform how to draw the controls it draws itself, such as checkboxes. */
  scheme: 'dark' | 'light'
  /**
   * Greys from the background end to the text end: 0 is the deepest
   * background and 11 the strongest text. In a dark theme they run from dark
   * to light and in a light theme from light to dark, so the same roles fit
   * every theme. None is pure black or white.
   */
  neutral: Record<NeutralStep, string>
  accents: Accents
  extras: Extras
}

const darkAccents: Accents = {
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

/** Darker than the dark themes' accents, so text in them reads on a light background. */
const lightAccents: Accents = {
  amber: { bright: '#764a0b', base: '#93621a', dim: '#93784a', muted: '#f0dfbc' },
  green: { bright: '#3b5b19', base: '#4f7428', dim: '#71865a', muted: '#d1e0b7' },
  red: { bright: '#922d1e', base: '#ad412e', dim: '#a27069', muted: '#f0cdc3' },
  blue: { bright: '#244e79', base: '#3a6690', dim: '#6b829a', muted: '#c9d9e9' },
  purple: { bright: '#5c3d78', base: '#73538d', dim: '#887897', muted: '#e0d1ea' },
  teal: { bright: '#185b4e', base: '#267163', dim: '#5f8880', muted: '#c3ded5' },
  orange: { bright: '#8a3f13', base: '#a55223', dim: '#a17a5d', muted: '#f1d1ba' },
  rose: { bright: '#84314c', base: '#9c4561', dim: '#9c7886', muted: '#eecdd8' },
  yellow: { bright: '#615007', base: '#7a6613', dim: '#8c8459', muted: '#efe4b9' },
}

const darkExtras: Extras = {
  scrim: 'rgba(12, 10, 8, 0.6)',
  shadow: 'rgba(8, 6, 4, 0.35)',
  pendingHover: '#403c28',
  diffAddedBg: '#1f2a19',
  diffAddedWord: '#36492a',
  diffAddedGutter: '#273520',
  diffRemovedBg: '#2d1c19',
  diffRemovedWord: '#52302a',
  diffRemovedGutter: '#382320',
  diffHunkBg: '#1c2026',
}

const themes: Record<ThemeName, Theme> = {
  // Nearly black, with greys that lean only faintly cool.
  darkBlack: {
    scheme: 'dark',
    neutral: {
      0: '#0a0a0b',
      1: '#0f0f11',
      2: '#141416',
      3: '#1a1a1d',
      4: '#202023',
      5: '#28282c',
      6: '#36363b',
      7: '#54545b',
      8: '#77777f',
      9: '#9a9aa2',
      10: '#d8d8de',
      11: '#e9e9ee',
    },
    accents: darkAccents,
    extras: { ...darkExtras, scrim: 'rgba(6, 6, 7, 0.6)', shadow: 'rgba(4, 4, 5, 0.4)' },
  },
  // Lighter than Dark (Black). The backgrounds are only faintly brown; the
  // text is warmer.
  darkBrown: {
    scheme: 'dark',
    neutral: {
      0: '#0e0d0c',
      1: '#141210',
      2: '#181614',
      3: '#1e1c19',
      4: '#24211e',
      5: '#2c2925',
      6: '#393430',
      7: '#585047',
      8: '#7b7266',
      9: '#9e9487',
      10: '#ddd4c6',
      11: '#ece4d7',
    },
    accents: darkAccents,
    extras: darkExtras,
  },
  // Warm paper, with brown text.
  lightBeige: {
    scheme: 'light',
    neutral: {
      0: '#faf6ee',
      1: '#f3ede2',
      2: '#ede6d8',
      3: '#e7dfcf',
      4: '#e0d7c5',
      5: '#d6cbb6',
      6: '#c2b49c',
      7: '#a3967f',
      8: '#776b59',
      9: '#5c5345',
      10: '#3d362d',
      11: '#29231c',
    },
    accents: lightAccents,
    extras: {
      scrim: 'rgba(52, 44, 32, 0.32)',
      shadow: 'rgba(70, 56, 36, 0.14)',
      pendingHover: '#e8dba2',
      diffAddedBg: '#e5ecd2',
      diffAddedWord: '#c9dba8',
      diffAddedGutter: '#d9e5bf',
      diffRemovedBg: '#f6e0d6',
      diffRemovedWord: '#ecc0b0',
      diffRemovedGutter: '#efd1c4',
      diffHunkBg: '#e0e3e2',
    },
  },
  // Near-white, with greys that lean only faintly cool.
  lightWhite: {
    scheme: 'light',
    neutral: {
      0: '#fcfcfd',
      1: '#f6f6f8',
      2: '#f0f0f3',
      3: '#eaeaee',
      4: '#e3e3e8',
      5: '#d9d9e0',
      6: '#c3c3cc',
      7: '#a3a3ad',
      8: '#70707b',
      9: '#55555f',
      10: '#33333a',
      11: '#1f1f24',
    },
    accents: lightAccents,
    extras: {
      scrim: 'rgba(32, 32, 40, 0.3)',
      shadow: 'rgba(30, 30, 45, 0.12)',
      pendingHover: '#e8dba2',
      diffAddedBg: '#e6f2df',
      diffAddedWord: '#c8e3b9',
      diffAddedGutter: '#d8eacd',
      diffRemovedBg: '#fae6e3',
      diffRemovedWord: '#f0c4bd',
      diffRemovedGutter: '#f4d6d1',
      diffHunkBg: '#e3e9f2',
    },
  },
}

/** The theme used when the settings name one that doesn't exist. */
const DEFAULT_THEME: ThemeName = 'darkBlack'

function rolesOf({ neutral, accents, extras }: Theme) {
  return {
    bgApp: neutral[1],
    bgSunken: neutral[0],
    bgPanel: neutral[2],
    bgRaised: neutral[3],
    bgHover: neutral[4],
    bgActive: neutral[5],
    border: neutral[5],
    borderSubtle: neutral[4],
    borderStrong: neutral[6],
    fg: neutral[10],
    fgStrong: neutral[11],
    fgMuted: neutral[9],
    fgFaint: neutral[8],
    fgDisabled: neutral[7],
    textOnAccent: neutral[0],
    scrollbar: neutral[6],
    scrollbarHover: neutral[7],
    scrim: extras.scrim,
    shadow: extras.shadow,

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
    markerPushed: neutral[8],

    statusModified: accents.yellow.base,
    statusStaged: accents.green.base,
    statusAdded: accents.green.bright,
    statusDeleted: accents.red.base,
    statusRenamed: accents.blue.base,
    statusTypeChange: accents.purple.base,
    statusConflicted: accents.orange.bright,
    statusUntracked: accents.teal.base,

    pending: accents.yellow.bright,
    pendingMuted: accents.yellow.muted,
    pendingHover: extras.pendingHover,

    diffAddedBg: extras.diffAddedBg,
    diffAddedWord: extras.diffAddedWord,
    diffAddedGutter: extras.diffAddedGutter,
    diffAddedSign: accents.green.base,
    diffRemovedBg: extras.diffRemovedBg,
    diffRemovedWord: extras.diffRemovedWord,
    diffRemovedGutter: extras.diffRemovedGutter,
    diffRemovedSign: accents.red.base,
    diffHunkBg: extras.diffHunkBg,
    diffHunkFg: accents.blue.dim,
    diffGapBg: neutral[2],

    searchMatch: accents.yellow.dim,
    searchMatchCurrent: accents.orange.bright,

    syntaxComment: neutral[8],
    syntaxString: accents.green.bright,
    syntaxNumber: accents.orange.bright,
    syntaxKeyword: accents.purple.bright,
    syntaxOperator: neutral[9],
    syntaxFunction: accents.blue.bright,
    syntaxType: accents.yellow.bright,
    syntaxVariable: neutral[10],
    syntaxParameter: accents.rose.bright,
    syntaxTag: accents.rose.bright,
    syntaxAttribute: accents.amber.bright,
    syntaxConstant: accents.orange.bright,
    syntaxNamespace: accents.teal.bright,
    syntaxRegex: accents.teal.bright,
    syntaxProperty: accents.blue.base,
    syntaxHeading: accents.amber.bright,
  }
}

export type Role = keyof ReturnType<typeof rolesOf>

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

/**
 * Publishes the colors of `name` as CSS custom properties on `root`. Every
 * color in the app follows them, so a new theme shows at once.
 */
export function applyTheme(root: HTMLElement, name: ThemeName): void {
  // A hand-edited settings file can name a theme that doesn't exist.
  const theme = themes[name] ?? themes[DEFAULT_THEME]
  for (const [role, value] of Object.entries(rolesOf(theme))) {
    root.style.setProperty(`--color-${kebab(role)}`, value)
  }
  for (const [accentName, variants] of Object.entries(theme.accents)) {
    for (const [variant, value] of Object.entries(variants)) {
      root.style.setProperty(`--accent-${accentName}-${variant}`, value)
    }
  }
  root.style.colorScheme = theme.scheme
}
