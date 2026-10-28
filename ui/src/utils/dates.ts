import { ref } from 'vue'

const CLOCK_TICK_MS = 30_000

/** Current time, ticking so relative dates in the UI stay current. */
export const now = ref(Date.now())
window.setInterval(() => {
  now.value = Date.now()
}, CLOCK_TICK_MS)

const relativeFormat = new Intl.RelativeTimeFormat('en', { numeric: 'always' })

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 60 * 60],
  ['month', 30 * 24 * 60 * 60],
  ['week', 7 * 24 * 60 * 60],
  ['day', 24 * 60 * 60],
  ['hour', 60 * 60],
  ['minute', 60],
]

/** "2 hours ago", relative to the ticking clock. Times are Unix seconds. */
export function formatRelative(seconds: number): string {
  const elapsed = Math.round(now.value / 1000 - seconds)
  if (Math.abs(elapsed) < 45) {
    return 'just now'
  }
  for (const [unit, unitSeconds] of UNITS) {
    if (Math.abs(elapsed) >= unitSeconds) {
      return relativeFormat.format(-Math.round(elapsed / unitSeconds), unit)
    }
  }
  return relativeFormat.format(-Math.round(elapsed / 60), 'minute')
}

function pad(value: number): string {
  return value.toString().padStart(2, '0')
}

/** "2026-10-28 14:02" in the local timezone. */
export function formatAbsolute(seconds: number): string {
  const date = new Date(seconds * 1000)
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ` +
    `${pad(date.getHours())}:${pad(date.getMinutes())}`
  )
}
