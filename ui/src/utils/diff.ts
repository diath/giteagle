import { diffWordsWithSpace } from 'diff'

import type { DiffLine, Hunk } from '../api'
import type { Token } from './highlight'

export interface Range {
  start: number
  end: number
}

export interface Segment {
  text: string
  color?: string
  italic: boolean
  bold: boolean
  changed: boolean
}

/** Word diffs of lines longer than this cost more than they help. */
const MAX_WORD_DIFF_LENGTH = 1000
/** Below this share of unchanged text the lines are just different. */
const MIN_SHARED_RATIO = 0.35

/**
 * Pairs each run of removed lines with the added lines that follow it, line
 * by line, and returns the changed character ranges keyed by line index
 * within the hunk.
 */
export function wordChanges(hunk: Hunk): Map<number, Range[]> {
  const changes = new Map<number, Range[]>()
  const lines = hunk.lines
  let index = 0
  while (index < lines.length) {
    if (lines[index].kind !== 'removed') {
      index += 1
      continue
    }
    const removedStart = index
    while (index < lines.length && lines[index].kind === 'removed') {
      index += 1
    }
    const addedStart = index
    while (index < lines.length && lines[index].kind === 'added') {
      index += 1
    }
    const pairs = Math.min(addedStart - removedStart, index - addedStart)
    for (let offset = 0; offset < pairs; offset += 1) {
      const ranges = changedRanges(lines[removedStart + offset].text, lines[addedStart + offset].text)
      if (ranges) {
        changes.set(removedStart + offset, ranges.removed)
        changes.set(addedStart + offset, ranges.added)
      }
    }
  }
  return changes
}

function changedRanges(before: string, after: string): { removed: Range[]; added: Range[] } | null {
  if (before.length > MAX_WORD_DIFF_LENGTH || after.length > MAX_WORD_DIFF_LENGTH) {
    return null
  }
  const removed: Range[] = []
  const added: Range[] = []
  let beforePosition = 0
  let afterPosition = 0
  let shared = 0
  for (const part of diffWordsWithSpace(before, after)) {
    const length = part.value.length
    if (part.added) {
      pushRange(added, afterPosition, afterPosition + length)
      afterPosition += length
    } else if (part.removed) {
      pushRange(removed, beforePosition, beforePosition + length)
      beforePosition += length
    } else {
      shared += length
      beforePosition += length
      afterPosition += length
    }
  }
  const longest = Math.max(before.length, after.length)
  if (longest > 0 && shared / longest < MIN_SHARED_RATIO) {
    return null
  }
  return { removed, added }
}

function pushRange(ranges: Range[], start: number, end: number): void {
  const last = ranges[ranges.length - 1]
  if (last && last.end === start) {
    last.end = end
  } else {
    ranges.push({ start, end })
  }
}

/**
 * Splits a line into spans carrying both its syntax colors and its
 * word-level change marks. Falls back to plain text when the tokens don't
 * match the line, e.g. for lines that weren't valid UTF-8.
 */
export function buildSegments(text: string, tokens: Token[] | undefined, ranges: Range[] | undefined): Segment[] {
  const tokenList =
    tokens && tokens.map((token) => token.content).join('') === text ? tokens : [{ content: text }]
  const boundaries = ranges ?? []
  const segments: Segment[] = []
  let position = 0
  for (const token of tokenList) {
    const tokenEnd = position + token.content.length
    let cursor = position
    while (cursor < tokenEnd) {
      const inRange = boundaries.find((range) => range.start <= cursor && cursor < range.end)
      let next = tokenEnd
      if (inRange) {
        next = Math.min(next, inRange.end)
      } else {
        const upcoming = boundaries.find((range) => range.start > cursor)
        if (upcoming) {
          next = Math.min(next, upcoming.start)
        }
      }
      segments.push({
        text: text.slice(cursor, next),
        color: token.color,
        italic: ((token.fontStyle ?? 0) & 1) !== 0,
        bold: ((token.fontStyle ?? 0) & 2) !== 0,
        changed: inRange !== undefined,
      })
      cursor = next
    }
    position = tokenEnd
  }
  return segments
}

export type DiffRow =
  | { type: 'hunk'; key: string; section: string; header: string }
  | { type: 'line'; key: string; line: DiffLine; ranges?: Range[] }
  | { type: 'gap'; key: string; gapIndex: number; count: number }

interface Gap {
  newStart: number
  newEnd: number
  /** Old line number minus new line number inside the gap. */
  offset: number
}

/**
 * Lays out hunks with the unchanged regions between them. Collapsed regions
 * become a single expandable row; expanded ones show their lines, taken from
 * the new file's contents.
 */
export function layoutRows(
  hunks: Hunk[],
  newLines: string[] | null,
  expandedGaps: ReadonlySet<number>,
): DiffRow[] {
  const rows: DiffRow[] = []
  const gaps = findGaps(hunks, newLines?.length ?? 0)
  const pushGap = (gapIndex: number) => {
    const gap = gaps[gapIndex]
    if (!gap || !newLines || gap.newEnd < gap.newStart) {
      return
    }
    if (!expandedGaps.has(gapIndex)) {
      rows.push({ type: 'gap', key: `gap-${gapIndex}`, gapIndex, count: gap.newEnd - gap.newStart + 1 })
      return
    }
    for (let newLine = gap.newStart; newLine <= gap.newEnd; newLine += 1) {
      rows.push({
        type: 'line',
        key: `gap-${gapIndex}-${newLine}`,
        line: { kind: 'context', oldLine: newLine + gap.offset, newLine, text: newLines[newLine - 1] ?? '' },
      })
    }
  }

  hunks.forEach((hunk, hunkIndex) => {
    pushGap(hunkIndex)
    // Once the region above is expanded the lines run on without a break,
    // so the hunk header would only interrupt them.
    if (!expandedGaps.has(hunkIndex)) {
      rows.push({
        type: 'hunk',
        key: `hunk-${hunkIndex}`,
        section: hunk.section,
        header: `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`,
      })
    }
    const changes = wordChanges(hunk)
    hunk.lines.forEach((line, lineIndex) => {
      rows.push({ type: 'line', key: `hunk-${hunkIndex}-${lineIndex}`, line, ranges: changes.get(lineIndex) })
    })
  })
  pushGap(hunks.length)
  return rows
}

function findGaps(hunks: Hunk[], newLineCount: number): Gap[] {
  const gaps: Gap[] = []
  let nextNew = 1
  let offset = 0
  for (const hunk of hunks) {
    // A hunk of an empty side reports start 0; its content begins at line 1.
    const newStart = Math.max(hunk.newStart, 1)
    gaps.push({ newStart: nextNew, newEnd: newStart - 1, offset: hunk.oldStart - hunk.newStart })
    nextNew = newStart + hunk.newLines
    offset = hunk.oldStart + hunk.oldLines - (hunk.newStart + hunk.newLines)
  }
  gaps.push({ newStart: nextNew, newEnd: newLineCount, offset })
  return gaps
}

/** Splits file contents into lines numbered like Git's, dropping the empty piece after a final newline. */
export function splitLines(content: string | null): string[] | null {
  if (content === null) {
    return null
  }
  const lines = content.replace(/\r\n/g, '\n').split('\n')
  if (lines.length > 0 && lines[lines.length - 1] === '') {
    lines.pop()
  }
  return lines
}
