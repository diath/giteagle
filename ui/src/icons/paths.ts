// Stroke icons drawn on a 16×16 grid. They are rendered with `currentColor`,
// so they take the color of the surrounding text.

export const ICON_PATHS = {
  branch:
    'M5 5.5v5 M11 5.5c0 2.5-6 2.5-6 5 M3.5 4a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0 M9.5 4a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0 M3.5 12a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0',
  switch: 'M3 5h9.5 M10 2.5l2.5 2.5L10 7.5 M13 11H3.5 M6 8.5L3.5 11 6 13.5',
  pull: 'M8 2.5v8 M4.5 7.5L8 11l3.5-3.5 M3 13.5h10',
  push: 'M8 13.5v-8 M4.5 8.5L8 5l3.5 3.5 M3 2.5h10',
  folder: 'M2 4.5v8h12v-6.5H7.5L6 4.5z',
  file: 'M4 1.5h5l3 3v10H4z M9 1.5v3h3',
  symlink: 'M4 1.5h5l3 3v10H4z M9 1.5v3h3 M6 11.5l3.5-3.5 M6.5 8h3v3',
  submodule: 'M2.5 4.5l5.5-3 5.5 3v7l-5.5 3-5.5-3z M2.5 4.5L8 7.5l5.5-3 M8 7.5v7',
  parent: 'M7 3.5L3.5 7 7 10.5 M3.5 7h6a3 3 0 0 1 3 3v2.5',
  back: 'M9.5 3.5L5 8l4.5 4.5',
  chevronDown: 'M4 6l4 4 4-4',
  chevronLeft: 'M10 4l-4 4 4 4',
  chevronRight: 'M6 4l4 4-4 4',
  search: 'M2.5 7a4.5 4.5 0 1 0 9 0a4.5 4.5 0 1 0-9 0 M10.3 10.3l3.2 3.2',
  info: 'M2.5 8a5.5 5.5 0 1 0 11 0a5.5 5.5 0 1 0-11 0 M8 7.5v3.5 M8 5v.01',
  help: 'M2.5 8a5.5 5.5 0 1 0 11 0a5.5 5.5 0 1 0-11 0 M6.25 6.5a1.75 1.75 0 1 1 2.5 1.58c-.5.27-.75.62-.75 1.17 M8 11.25v.01',
  eye: 'M1.5 8c1.5-3 3.75-4.5 6.5-4.5s5 1.5 6.5 4.5c-1.5 3-3.75 4.5-6.5 4.5S3 11 1.5 8z M6 8a2 2 0 1 0 4 0a2 2 0 1 0-4 0',
  trash: 'M2.5 4.5h11 M6 4.5v-2h4v2 M4 4.5l.75 9h6.5l.75-9 M6.75 7v4 M9.25 7v4',
  close: 'M4 4l8 8 M12 4l-8 8',
  check: 'M3 8.5l3 3 7-7',
  warning: 'M8 2l6.5 11.5h-13z M8 6.5v3.5 M8 12v.01',
  refresh: 'M13 3.5v3h-3 M12.7 6.5A5 5 0 1 0 13 9',
  history: 'M2.5 8a5.5 5.5 0 1 0 1.6-3.9 M2.5 2.5v2.5H5 M8 5v3.25l2 1.5',
  blame: 'M2 4h2 M2 8h2 M2 12h2 M6.25 2v12 M8.5 4h5.5 M8.5 8h3.5 M8.5 12h5',
  expand: 'M8 2.5v3.5 M5.5 4L8 6.5 10.5 4 M8 13.5V10 M5.5 12L8 9.5l2.5 2.5',
  dot: 'M6 8a2 2 0 1 0 4 0a2 2 0 1 0-4 0',
  pencil: 'M11 2.5l2.5 2.5L6 12.5l-3.5 1 1-3.5z M9.5 4l2.5 2.5',
  more: 'M2.75 8a.75.75 0 1 0 1.5 0a.75.75 0 1 0-1.5 0 M7.25 8a.75.75 0 1 0 1.5 0a.75.75 0 1 0-1.5 0 M11.75 8a.75.75 0 1 0 1.5 0a.75.75 0 1 0-1.5 0',
  commit: 'M5.5 8a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0 M1.5 8h4 M10.5 8h4',
  copy: 'M5.5 5.5h8v8h-8z M10.5 5.5v-3h-8v8h3',
  stash: 'M2 3h12v3H2z M3 6v7h10V6 M6.5 8.5h3',
  stashApply: 'M2 3h12v3H2z M3 6v7h10V6 M8 8v3.5 M6.25 9.75L8 8l1.75 1.75',
  stashPop: 'M3 7.5v6h10v-6 M2 4.5l11-2.75 M8 12V5.5 M6 7.5l2-2 2 2',
  cherryPick:
    'M7 7V5.5a1 1 0 0 0-2 0V9a1.25 1.25 0 0 0-2.5 0v.5A3.5 3.5 0 0 0 6 13h3.5a3.5 3.5 0 0 0 3.5-3.5V6a1 1 0 0 0-2 0v1 M11 6.5V5a1 1 0 0 0-2 0v2 M9 6.5V4.5a1 1 0 0 0-2 0V7',
  revert: 'M3 3.5v3h3 M3.3 6.5A5 5 0 1 1 3 9',
  squash: 'M2.5 8h11 M8 2v4 M6 4l2 2 2-2 M8 14v-4 M6 12l2-2 2 2',
  issue: 'M2.5 8a5.5 5.5 0 1 0 11 0a5.5 5.5 0 1 0-11 0 M7 8a1 1 0 1 0 2 0a1 1 0 1 0-2 0',
  pullRequest:
    'M3 3.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0 M3 12.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0 M4.5 5v6 M10 12.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0 M11.5 11V6a2 2 0 0 0-2-2H8 M9.5 2.5L8 4l1.5 1.5',
} as const

export type IconName = keyof typeof ICON_PATHS
