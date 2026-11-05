/** `path` without the separators it ends with, as a project's workdir does. */
function withoutTrailingSeparators(path: string): string {
  return path.replace(/[\\/]+$/, '')
}

/** `path` split before its last component, so it can be cut in the middle. */
export function splitBeforeLast(path: string): { start: string; end: string } {
  const trimmed = withoutTrailingSeparators(path)
  const cut = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  return cut < 0 ? { start: '', end: trimmed } : { start: trimmed.slice(0, cut), end: trimmed.slice(cut) }
}

/** The last component of `path`, e.g. the name of a project's folder. */
export function folderName(path: string): string {
  return splitBeforeLast(path).end.replace(/^[\\/]/, '') || path
}
