import type { Person } from '../api'
import { accentVar, avatarAccents, cssVar } from '../theme/palette'

export function initials(name: string): string {
  const words = name
    .trim()
    .split(/[\s._-]+/)
    .filter((word) => word.length > 0)
  if (words.length === 0) {
    return '?'
  }
  if (words.length === 1) {
    return Array.from(words[0]).slice(0, 2).join('').toUpperCase()
  }
  const first = Array.from(words[0])[0] ?? ''
  const last = Array.from(words[words.length - 1])[0] ?? ''
  return (first + last).toUpperCase()
}

/** FNV-1a, so an author keeps the same color across restarts. */
function hash(text: string): number {
  let value = 0x811c9dc5
  for (let index = 0; index < text.length; index += 1) {
    value ^= text.charCodeAt(index)
    value = Math.imul(value, 0x01000193)
  }
  return value >>> 0
}

export function placeholderColors(email: string): { background: string; foreground: string } {
  const accentName = avatarAccents[hash(email.trim().toLowerCase()) % avatarAccents.length]
  return { background: accentVar(accentName, 'base'), foreground: cssVar('textOnAccent') }
}

/** "Ann", "Ann and Bo", "Ann, Bo and Cy". */
export function joinNames(people: Person[]): string {
  const names = people.map((person) => person.name)
  if (names.length <= 1) {
    return names.join('')
  }
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`
}
