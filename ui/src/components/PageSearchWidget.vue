<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'

import { useEscape } from '../utils/escape'
import {
  createRanges,
  findMatches,
  firstMatchIndex,
  matchesNearView,
  readTextBlocks,
  revealMatch,
  type PageMatch,
  type TextBlock,
} from '../utils/pageSearch'
import AppIcon from './AppIcon.vue'

/** The names base.css styles the highlights by. */
const MATCH_HIGHLIGHT = 'page-search-match'
const CURRENT_HIGHLIGHT = 'page-search-current'

const props = defineProps<{ root: HTMLElement; active: boolean }>()
const term = defineModel<string>('term', { required: true })
const emit = defineEmits<{ close: [] }>()

useEscape(() => emit('close'))

const widget = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)
const matches = shallowRef<PageMatch[]>([])
const currentIndex = ref(-1)

let blocks: TextBlock[] = []
let observer: MutationObserver | null = null
let frame = 0
let rescanPending = false
/** The slice of the matches being painted, to skip painting it again. */
let painted = { first: -1, end: -1 }

const matchHighlight = new Highlight()
const currentHighlight = new Highlight()
currentHighlight.priority = 1

const countText = computed(() => {
  if (matches.value.length > 0) {
    return `${currentIndex.value + 1} of ${matches.value.length}`
  }
  return term.value === '' ? '' : 'No matches'
})

function currentMatch(): PageMatch | null {
  return (matches.value[currentIndex.value] as PageMatch | undefined) ?? null
}

function paintMatches(): void {
  const { first, end } = matchesNearView(matches.value, props.root)
  if (first === painted.first && end === painted.end) {
    return
  }
  painted = { first, end }
  matchHighlight.clear()
  for (const range of createRanges(matches.value.slice(first, end))) {
    matchHighlight.add(range)
  }
}

/** Paints the current match, which is always there however far it is from the view. */
function paintCurrent(): Range | null {
  currentHighlight.clear()
  const match = currentMatch()
  const range = match ? (createRanges([match]).at(0) ?? null) : null
  if (range) {
    currentHighlight.add(range)
  }
  return range
}

function repaint(): void {
  painted = { first: -1, end: -1 }
  paintMatches()
  paintCurrent()
}

/** Finds the term again, staying on the match at or after the current one. */
function updateMatches(): void {
  const anchor = currentMatch()
  matches.value = findMatches(blocks, term.value)
  currentIndex.value = firstMatchIndex(matches.value, anchor, props.root)
  repaint()
}

function showCurrent(): void {
  const match = currentMatch()
  const range = paintCurrent()
  if (match && range) {
    revealMatch(match, range, props.root)
  }
}

function step(delta: number): void {
  const count = matches.value.length
  if (count > 0) {
    currentIndex.value = (currentIndex.value + delta + count) % count
    showCurrent()
  }
}

function onEnter(event: KeyboardEvent): void {
  step(event.shiftKey ? -1 : 1)
}

// In a frame of its own, which still comes before the page is drawn, so that
// a burst of changes costs one search and no match is seen unpainted.
function scheduleUpdate(rescan: boolean): void {
  rescanPending ||= rescan
  if (frame === 0) {
    frame = requestAnimationFrame(() => {
      frame = 0
      if (rescanPending) {
        rescanPending = false
        blocks = readTextBlocks(props.root)
        updateMatches()
      } else {
        paintMatches()
      }
    })
  }
}

function onScroll(): void {
  scheduleUpdate(false)
}

// One name holds the highlights of the view on screen, as only that one shows.
function registerHighlights(): void {
  CSS.highlights.set(MATCH_HIGHLIGHT, matchHighlight)
  CSS.highlights.set(CURRENT_HIGHLIGHT, currentHighlight)
}

watch(term, () => {
  updateMatches()
  showCurrent()
})

watch(
  () => props.active,
  (active) => {
    if (active) {
      registerHighlights()
    }
  },
)

onMounted(() => {
  if (props.active) {
    registerHighlights()
  }
  blocks = readTextBlocks(props.root)
  updateMatches()
  showCurrent()
  input.value?.focus()
  input.value?.select()

  // Highlighting a file replaces the spans of its lines and showing more of
  // it adds lines, either of which outdates the matches. The widget's own
  // count changing doesn't.
  observer = new MutationObserver((records) => {
    if (records.some((record) => !widget.value?.contains(record.target))) {
      scheduleUpdate(true)
    }
  })
  observer.observe(props.root, { subtree: true, childList: true, characterData: true })
  // Scroll events don't bubble, but they can be caught on their way down.
  props.root.addEventListener('scroll', onScroll, { capture: true, passive: true })
})

onBeforeUnmount(() => {
  cancelAnimationFrame(frame)
  observer?.disconnect()
  props.root.removeEventListener('scroll', onScroll, { capture: true })
  if (CSS.highlights.get(MATCH_HIGHLIGHT) === matchHighlight) {
    CSS.highlights.delete(MATCH_HIGHLIGHT)
    CSS.highlights.delete(CURRENT_HIGHLIGHT)
  }
})
</script>

<template>
  <!-- A line without height under the view's bar, so the widget hangs just
       below the bar however tall that is. -->
  <div ref="widget" class="anchor">
    <div class="widget" role="search">
      <input
        ref="input"
        v-model="term"
        class="text-input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="Search this page"
        aria-label="Search this page"
        @keydown.enter.prevent="onEnter"
      />
      <span class="count faint" aria-live="polite">{{ countText }}</span>
      <button
        class="icon-button"
        type="button"
        title="Previous match (Shift+Enter)"
        :disabled="matches.length === 0"
        @click="step(-1)"
      >
        <AppIcon name="chevronUp" />
      </button>
      <button
        class="icon-button"
        type="button"
        title="Next match (Enter)"
        :disabled="matches.length === 0"
        @click="step(1)"
      >
        <AppIcon name="chevronDown" />
      </button>
      <button class="icon-button" type="button" title="Close (Esc)" @click="emit('close')">
        <AppIcon name="close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Above the sticky file headers of the diffs. */
.anchor {
  position: relative;
  z-index: 10;
  height: 0;
}

/* Views whose content has a sticky header set --page-search-top to clear it. */
.widget {
  position: absolute;
  top: var(--page-search-top, var(--space-2));
  right: var(--space-6);
  display: flex;
  align-items: center;
  gap: var(--space-1);
  max-width: calc(100% - 2 * var(--space-6));
  padding: var(--space-1);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius);
  background: var(--color-bg-raised);
}

.text-input {
  flex: 0 1 calc(250 * var(--px));
  min-width: calc(80 * var(--px));
}

/* Wide enough that the buttons stay put as the count changes. */
.count {
  flex: none;
  min-width: 11ch;
  font-size: var(--font-size-small);
  text-align: center;
  white-space: nowrap;
}

.icon-button {
  flex: none;
}
</style>
