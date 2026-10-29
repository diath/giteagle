/** How Git names a stash entry, e.g. `stash@{0}` for the newest. */
export function stashName(index: number): string {
  return `stash@{${index}}`
}
