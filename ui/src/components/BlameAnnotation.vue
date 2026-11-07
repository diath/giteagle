<script setup lang="ts">
import { computed } from 'vue'

import type { BlameCommit, EarlierVersion } from '../api'
import { useProject } from '../stores/project'
import { formatAbsolute, formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'

/**
 * Says which commit last changed a run of lines of a blamed file, on each of
 * the lines. Only the first line is drawn in full, so the runs stand apart.
 * Clicking it opens the commit. It is a component of its own so that the
 * ticking relative times redraw only the annotations, not the code.
 */
const props = defineProps<{
  /** `null` while the blame is found, or for lines it doesn't cover. */
  commit: BlameCommit | null
  earlier: EarlierVersion | null
  lineCount: number
  /** How recent the commit is among the file's, from 0 for the oldest to 1 for the newest. */
  age: number
  /** Whether the blame is still being found. */
  loading: boolean
}>()

const { openBlame, openCommit } = useProject()

/** Each column's text on the first line, and on the others as one text. */
const columns = computed(() => {
  const commit = props.commit
  if (!commit) {
    return null
  }
  const values = {
    time: formatRelative(commit.authorTime),
    hash: commit.shortId,
    author: commit.author.name,
    subject: commit.subject,
  }
  const repeat = (value: string) => `\n${value}`.repeat(props.lineCount - 1)
  return {
    first: values,
    rest: {
      time: repeat(values.time),
      hash: repeat(values.hash),
      author: repeat(values.author),
      subject: repeat(values.subject),
    },
  }
})

const details = computed(() => {
  const commit = props.commit
  if (!commit) {
    return undefined
  }
  const { author, authorTime } = commit
  return [
    `${commit.shortId} ${commit.subject}`,
    `${author.name} <${author.email}>`,
    `${formatAbsolute(authorTime)} (${formatRelative(authorTime)})`,
    'Click to open the commit',
  ].join('\n')
})

const earlierTitle = computed(() => {
  const shortId = props.commit?.shortId
  return props.earlier
    ? `Show the blame from before ${shortId} changed these lines`
    : `The file didn't exist before ${shortId}`
})

function open(): void {
  if (props.commit) {
    openCommit(props.commit.id)
  }
}

function showEarlier(): void {
  const earlier = props.earlier
  if (earlier) {
    openBlame(earlier.id, earlier.shortId, earlier.path, earlier.line)
  }
}
</script>

<template>
  <div
    v-if="commit && columns"
    class="annotation"
    role="button"
    tabindex="0"
    :title="details"
    :style="{ '--age': age }"
    @click="open"
    @keydown.enter.self="open"
  >
    <button
      class="icon-button earlier"
      type="button"
      :disabled="!earlier"
      :title="earlierTitle"
      @click.stop="showEarlier"
    >
      <AppIcon name="earlier" />
    </button>
    <span class="cell"><span class="time">{{ columns.first.time }}</span>{{ columns.rest.time }}</span>
    <span class="cell"><span class="hash">{{ columns.first.hash }}</span>{{ columns.rest.hash }}</span>
    <span class="cell"><span class="author">{{ columns.first.author }}</span>{{ columns.rest.author }}</span>
    <span class="cell subject-cell"><span class="subject">{{ columns.first.subject }}</span>{{ columns.rest.subject }}</span>
  </div>
  <div v-else class="annotation annotation--empty">
    <span v-if="loading" class="pending faint">Finding who changed each line…</span>
  </div>
</template>

<style scoped>
/* The columns have fixed widths, so they line up across the runs. */
.annotation {
  --earlier-width: calc(22 * var(--px));

  position: relative;
  display: grid;
  grid-template-columns: var(--earlier-width) 14ch 7ch 16ch minmax(0, 1fr);
  column-gap: 1ch;
  width: clamp(64ch, 38cqw, 88ch);
  padding: 0 var(--space-2) 0 var(--space-1);
  /* The repeats on the lines after the first. */
  color: var(--color-fg-disabled);
  cursor: pointer;
  transition: background-color var(--transition);
}

.annotation:hover {
  background: var(--color-bg-hover);
}

.annotation:focus-visible {
  outline-offset: -2px;
}

/* A strip along the line numbers that is stronger the newer the commit. */
.annotation::after {
  position: absolute;
  inset: 0 0 0 auto;
  width: calc(3 * var(--px));
  background: var(--color-accent);
  opacity: calc(0.15 + 0.85 * var(--age));
  content: '';
}

.annotation--empty {
  cursor: default;
}

.annotation--empty:hover {
  background: transparent;
}

.annotation--empty::after {
  content: none;
}

/* Stays at the top of the view, however far the lines it covers scroll. */
.pending {
  position: sticky;
  top: 0;
  grid-column: 2 / -1;
  align-self: start;
  white-space: nowrap;
}

/* One line tall, so the lines beside it stay level. */
.earlier {
  align-self: start;
  width: var(--earlier-width);
  height: 1lh;
}

/* The row under it is already lit up. */
.earlier:hover:not(:disabled) {
  background: var(--color-bg-active);
}

.earlier .app-icon {
  width: calc(15 * var(--px));
  height: calc(15 * var(--px));
}

.cell {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: pre;
}

.time,
.subject {
  color: var(--color-fg-muted);
}

.author {
  color: var(--color-fg);
}

/* In a narrow window the subjects are left to the tooltip, leaving the
   code more room. */
@container (max-width: 80em) {
  .annotation {
    grid-template-columns: var(--earlier-width) 14ch 7ch 16ch;
    width: auto;
  }

  .subject-cell {
    display: none;
  }
}
</style>
