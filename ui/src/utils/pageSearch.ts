// Finds text on the page shown by a view. Elements opt in with the
// `data-searchable` attribute, so buttons, labels and line numbers aren't
// searched. Each one is a block of text, such as a line of code, that a match
// can't run past, but it may hold many spans of highlighted tokens.

import { scaled } from '../theme/scale'

const SEARCHABLE_SELECTOR = '[data-searchable]'
const STICKY_SELECTOR = '[data-searchable="sticky"]'

/** How close to the edge of its scroller, in design pixels, a match gets centered instead. */
const REVEAL_MARGIN = 100

/** The most matches painted at once. See `matchesNearView`. */
const MAX_PAINTED_MATCHES = 1000

export interface TextBlock {
  element: Element
  text: string
}

/** A match, as offsets into the text of its block. */
export interface PageMatch {
  element: Element
  start: number
  end: number
}

export function readTextBlocks(root: Element): TextBlock[] {
  return Array.from(root.querySelectorAll(SEARCHABLE_SELECTOR), (element) => ({
    element,
    text: element.textContent,
  }))
}

function escapeForRegExp(text: string): string {
  return text.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&')
}

/**
 * Finds `term` in `blocks`, ignoring case, in page order. A regular expression
 * does the folding, as lowercasing a text can change its length and with it
 * the offsets.
 */
export function findMatches(blocks: readonly TextBlock[], term: string): PageMatch[] {
  if (term === '') {
    return []
  }
  const pattern = new RegExp(escapeForRegExp(term), 'giu')
  const matches: PageMatch[] = []
  for (const { element, text } of blocks) {
    for (let found = pattern.exec(text); found !== null; found = pattern.exec(text)) {
      matches.push({ element, start: found.index, end: found.index + found[0].length })
    }
  }
  return matches
}

/** Binary search for the first match `isPast` holds for; it must hold for every later one too. */
function firstIndexWhere(matches: readonly PageMatch[], isPast: (match: PageMatch) => boolean): number {
  let low = 0
  let high = matches.length
  while (low < high) {
    const middle = Math.floor((low + high) / 2)
    if (isPast(matches[middle])) {
      high = middle
    } else {
      low = middle + 1
    }
  }
  return low
}

/**
 * The top of a block in the viewport. A sticky header reports where it is
 * stuck, so its section is measured instead, keeping the blocks in page order.
 */
function blockTop(element: Element): number {
  const measured = element.matches(STICKY_SELECTOR) ? (element.closest('section') ?? element) : element
  return measured.getBoundingClientRect().top
}

function isAtOrAfter(match: PageMatch, anchor: PageMatch): boolean {
  if (match.element === anchor.element) {
    return match.start >= anchor.start
  }
  return (anchor.element.compareDocumentPosition(match.element) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0
}

/**
 * The match to show first: the one at or after `anchor` if it is still on the
 * page, or else the first in view, wrapping around to the top of the page.
 * It is -1 without matches.
 */
export function firstMatchIndex(matches: readonly PageMatch[], anchor: PageMatch | null, root: Element): number {
  if (matches.length === 0) {
    return -1
  }
  const viewTop = root.getBoundingClientRect().top
  const index =
    anchor?.element.isConnected === true
      ? firstIndexWhere(matches, (match) => isAtOrAfter(match, anchor))
      : firstIndexWhere(matches, (match) => blockTop(match.element) >= viewTop)
  return index === matches.length ? 0 : index
}

/**
 * The part of `matches` worth painting, as the bounds of a slice: those within
 * a screen of the view. WebKit compares every run of text it paints with every
 * range of a highlight, so painting all the matches of a common term would
 * make the whole page crawl.
 */
export function matchesNearView(matches: readonly PageMatch[], root: Element): { first: number; end: number } {
  const box = root.getBoundingClientRect()
  const top = box.top - box.height
  const bottom = box.bottom + box.height
  const first = firstIndexWhere(matches, (match) => blockTop(match.element) >= top)
  const end = firstIndexWhere(matches, (match) => blockTop(match.element) > bottom)
  return { first, end: Math.min(end, first + MAX_PAINTED_MATCHES) }
}

/** Builds a range for each of `matches`, which must be in page order. */
export function createRanges(matches: readonly PageMatch[]): Range[] {
  const ranges: Range[] = []
  let element: Element | null = null
  let walker: TreeWalker | null = null
  let node: Text | null = null
  /** Where `node` starts in the text of its block. */
  let nodeStart = 0

  function moveToCharacter(index: number): void {
    while (node !== null && index >= nodeStart + node.length) {
      nodeStart += node.length
      node = walker?.nextNode() as Text | null
    }
  }

  for (const match of matches) {
    if (match.element !== element) {
      element = match.element
      walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT)
      node = walker.nextNode() as Text | null
      nodeStart = 0
    }
    const range = document.createRange()
    moveToCharacter(match.start)
    if (node === null) {
      continue
    }
    range.setStart(node, match.start - nodeStart)
    moveToCharacter(match.end - 1)
    if (node === null) {
      continue
    }
    range.setEnd(node, match.end - nodeStart)
    ranges.push(range)
  }
  return ranges
}

/** How far to scroll along one axis to center a target, or 0 if it is well in view already. */
function scrollDelta(targetStart: number, targetEnd: number, viewStart: number, viewLength: number): number {
  const margin = Math.min(scaled(REVEAL_MARGIN), viewLength / 3)
  const comfortable = targetStart >= viewStart + margin && targetEnd <= viewStart + viewLength - margin
  return comfortable ? 0 : (targetStart + targetEnd) / 2 - (viewStart + viewLength / 2)
}

function scrolls(overflow: string): boolean {
  return overflow === 'auto' || overflow === 'scroll'
}

function revealIn(scroller: Element, match: PageMatch, range: Range): void {
  const style = getComputedStyle(scroller)
  const box = scroller.getBoundingClientRect()
  const target = range.getBoundingClientRect()
  if (scrolls(style.overflowY) && scroller.scrollHeight > scroller.clientHeight) {
    // A stuck header is in view wherever its section is, which is the place to scroll to.
    const top = match.element.matches(STICKY_SELECTOR) ? blockTop(match.element) : target.top
    scroller.scrollTop += scrollDelta(top, top + target.height, box.top + scroller.clientTop, scroller.clientHeight)
  }
  if (scrolls(style.overflowX) && scroller.scrollWidth > scroller.clientWidth) {
    scroller.scrollLeft += scrollDelta(target.left, target.right, box.left + scroller.clientLeft, scroller.clientWidth)
  }
}

/** Scrolls every scroller between `match` and `root` so that the match shows. */
export function revealMatch(match: PageMatch, range: Range, root: Element): void {
  for (let element: Element | null = match.element; element && element !== root; element = element.parentElement) {
    revealIn(element, match, range)
  }
}
