<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue'

import { errorMessage, type BlameCommit, type BlameHunk, type FileBlame } from '../api'
import AppIcon from '../components/AppIcon.vue'
import BlameAnnotation from '../components/BlameAnnotation.vue'
import FileVersionBar from '../components/FileVersionBar.vue'
import { useProject } from '../stores/project'
import type { Segment } from '../utils/diff'
import { useEscape } from '../utils/escape'
import { useFileAt } from '../utils/fileText'

const props = defineProps<{ commit: string; shortId: string; path: string; line: number | null }>()

const { api, goBack, openFileHistory } = useProject()
const { lines, rows, notice } = useFileAt(api, props.commit, props.path)

const blame = shallowRef<FileBlame | null>(null)
const blameError = ref<string | null>(null)
const scroller = ref<HTMLElement | null>(null)
/** Where the marked line was last scrolled to, to tell whether the view was scrolled since. */
let revealedAt: number | null = null

useEscape(goBack)

/** Lines in a row that the same commit last changed, or every line until the blame arrives. */
interface LineGroup {
  /** The number of its first line, counting from 1. */
  start: number
  rows: Segment[][]
  /** One number per line, as a single text, which lays out far faster than a node per line. */
  numbers: string
  hunk: BlameHunk | null
  commit: BlameCommit | null
}

function lineGroup(start: number, end: number, hunk: BlameHunk | null): LineGroup {
  const numbers = []
  for (let number = start + 1; number <= end; number += 1) {
    numbers.push(number)
  }
  return {
    start: start + 1,
    rows: rows.value.slice(start, end),
    numbers: numbers.join('\n'),
    hunk,
    commit: hunk ? (blame.value?.commits[hunk.commitId] ?? null) : null,
  }
}

const groups = computed(() => {
  const total = rows.value.length
  const result: LineGroup[] = []
  // Lines the blame doesn't cover, which only a version of the file it
  // splits into lines differently would leave, still show.
  let next = 0
  for (const hunk of blame.value?.hunks ?? []) {
    const start = hunk.startLine - 1
    const end = Math.min(start + hunk.lineCount, total)
    if (start < next || end <= start) {
      continue
    }
    if (start > next) {
      result.push(lineGroup(next, start, null))
    }
    result.push(lineGroup(start, end, hunk))
    next = end
  }
  if (next < total) {
    result.push(lineGroup(next, total, null))
  }
  return result
})

/**
 * How recent each commit is among the file's, from 0 for the oldest to 1 for
 * the newest. Spread by rank rather than by time, so a few old commits don't
 * leave all the others looking equally new.
 */
const ages = computed(() => {
  const commits = Object.values(blame.value?.commits ?? {})
  const times = [...new Set(commits.map((commit) => commit.authorTime))].sort((left, right) => left - right)
  const ranks = new Map(times.map((time, rank) => [time, rank]))
  const ages = new Map<string, number>()
  for (const commit of commits) {
    ages.set(commit.id, times.length > 1 ? (ranks.get(commit.authorTime) ?? 0) / (times.length - 1) : 1)
  }
  return ages
})

const lineNumberDigits = computed(() => String(lines.value.length).length)

/**
 * Scrolls the marked line to the middle of the view. The lines move once the
 * blame splits them into groups, so this runs again then, unless the view
 * was scrolled in between.
 */
function revealLine(): void {
  const view = scroller.value
  const target = view?.querySelector('.line--target')
  if (!view || !target) {
    return
  }
  if (revealedAt !== null && view.scrollTop !== revealedAt) {
    return
  }
  const offset = target.getBoundingClientRect().top - view.getBoundingClientRect().top
  view.scrollTop += offset - (view.clientHeight - target.clientHeight) / 2
  revealedAt = view.scrollTop
}

watch(groups, revealLine, { flush: 'post' })

onMounted(async () => {
  try {
    blame.value = await api.blame(props.commit, props.path)
  } catch (caught) {
    blameError.value = errorMessage(caught)
  }
})
</script>

<template>
  <div class="blame-view">
    <FileVersionBar icon="blame" :commit="commit" :short-id="shortId" :path="path">
      <template #end>
        <button
          class="button"
          type="button"
          title="Show the commits on this branch that changed this file"
          @click="openFileHistory(path)"
        >
          <AppIcon name="history" />
          History
        </button>
      </template>
    </FileVersionBar>

    <p v-if="notice" class="message" :class="notice.tone">{{ notice.text }}</p>
    <template v-else>
      <p v-if="blameError" class="blame-error">Couldn't find who changed each line: {{ blameError }}</p>
      <div ref="scroller" class="code-scroll">
        <div class="blame" :style="{ '--digits': lineNumberDigits }">
          <div v-for="group in groups" :key="group.start" class="group">
            <div class="gutter">
              <BlameAnnotation
                :commit="group.commit"
                :earlier="group.hunk?.earlier ?? null"
                :line-count="group.rows.length"
                :age="group.commit ? (ages.get(group.commit.id) ?? 0) : 0"
                :loading="!blame && !blameError"
              />
              <pre class="numbers" aria-hidden="true">{{ group.numbers }}</pre>
            </div>
            <div class="text">
              <div
                v-for="(segments, index) in group.rows"
                :key="index"
                class="line"
                :class="{ 'line--target': group.start + index === line }"
              ><span
                v-for="(segment, segmentIndex) in segments"
                :key="segmentIndex"
                :class="{ italic: segment.italic, bold: segment.bold }"
                :style="segment.color ? { color: segment.color } : undefined"
              >{{ segment.text }}</span></div>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.blame-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  container-type: inline-size;
}

.message {
  margin: 0;
  padding: var(--space-8);
  text-align: center;
}

.error {
  color: var(--color-danger-bright);
}

.blame-error {
  margin: 0;
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  color: var(--color-danger-bright);
  font-size: var(--font-size-small);
}

.code-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--color-bg-sunken);
}

/* The annotations, numbers and lines share one line height, so they stay
   level. */
.blame {
  min-width: max-content;
  font-size: var(--font-size-small);
  line-height: 1.55;
}

.group {
  display: flex;
}

.group + .group {
  border-top: 1px solid var(--color-border-subtle);
}

/* Stays in view while long lines scroll sideways. It can't be selected, so
   lines selected across groups copy as code alone. */
.gutter {
  position: sticky;
  left: 0;
  z-index: 1;
  display: flex;
  flex: none;
  border-right: 1px solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  user-select: none;
}

.numbers {
  width: calc(var(--digits) * 1ch + 2 * var(--space-2));
  margin: 0;
  padding: 0 var(--space-2);
  color: var(--color-fg-faint);
  font: inherit;
  text-align: right;
  user-select: none;
}

.text {
  flex: 1;
  padding: 0 var(--space-4);
  color: var(--color-syntax-variable);
  white-space: pre;
  tab-size: 4;
  transition: background-color var(--transition);
}

/* Pointing at an annotation lights up the lines it is about. */
.gutter:hover + .text {
  background: var(--color-bg-panel);
}

/* An empty line still takes its height. */
.line {
  min-height: 1lh;
}

/* Reaches across the padding, so the mark spans the whole width. */
.line--target {
  margin: 0 calc(-1 * var(--space-4));
  padding: 0 var(--space-4);
  background: var(--color-accent-muted);
}

.italic {
  font-style: italic;
}

.bold {
  font-weight: 600;
}
</style>
