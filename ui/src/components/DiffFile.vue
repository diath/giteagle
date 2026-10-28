<script lang="ts">
import type { InjectionKey, Ref } from 'vue'

/** The element the diffs scroll in, which the highlight observer watches. */
export const DIFF_SCROLLER: InjectionKey<Ref<HTMLElement | null>> = Symbol('diff-scroller')
</script>

<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'

import type { ChangeKind, FileDiff } from '../api'
import { scaled } from '../theme/scale'
import { buildSegments, layoutRows, splitLines } from '../utils/diff'
import { languageFor, tokenizeFile, type Token } from '../utils/highlight'
import AppIcon from './AppIcon.vue'

/** Highlighting starts a little before a file scrolls into view. */
const HIGHLIGHT_MARGIN_PX = scaled(1000)

const props = defineProps<{ file: FileDiff; highlighted: boolean }>()

const root = ref<HTMLElement | null>(null)
const scroller = inject(DIFF_SCROLLER, null)
const collapsed = ref(false)
const expandedGaps = shallowRef(new Set<number>())
const oldTokens = shallowRef<Token[][] | null>(null)
const newTokens = shallowRef<Token[][] | null>(null)
let observer: IntersectionObserver | null = null

const newLines = computed(() => splitLines(props.file.newContent))

const rows = computed(() =>
  layoutRows(props.file.hunks, newLines.value, expandedGaps.value).map((row) => {
    if (row.type !== 'line') {
      return row
    }
    const { line } = row
    const tokens =
      line.kind === 'removed'
        ? line.oldLine !== null
          ? oldTokens.value?.[line.oldLine - 1]
          : undefined
        : line.newLine !== null
          ? newTokens.value?.[line.newLine - 1]
          : undefined
    return { ...row, segments: buildSegments(line.text, tokens, row.ranges) }
  }),
)

const gutterWidth = computed(() => {
  const largest = props.file.hunks.reduce(
    (max, hunk) => Math.max(max, hunk.oldStart + hunk.oldLines, hunk.newStart + hunk.newLines),
    newLines.value?.length ?? 0,
  )
  return `${Math.max(String(largest).length, 3) + 1}ch`
})

const KIND_LABELS: Record<ChangeKind, string> = {
  added: 'Added',
  deleted: 'Deleted',
  modified: 'Modified',
  renamed: 'Renamed',
  copied: 'Copied',
  typeChange: 'Type changed',
}

const MODE_NAMES: Record<number, string> = {
  0o100644: 'regular file',
  0o100755: 'executable file',
  0o120000: 'symbolic link',
  0o160000: 'submodule',
}

function describeMode(mode: number): string {
  return `${mode.toString(8)} (${MODE_NAMES[mode] ?? 'unknown'})`
}

const modeChange = computed(() => {
  const { oldMode, newMode } = props.file
  if (oldMode === null || newMode === null || oldMode === newMode) {
    return null
  }
  return `Mode changed from ${describeMode(oldMode)} to ${describeMode(newMode)}.`
})

/** Shown instead of the contents when there is nothing to diff line by line. */
const summary = computed(() => {
  const file = props.file
  if (file.isSubmodule) {
    return 'The submodule now points to a different commit.'
  }
  if (file.isBinary) {
    return {
      added: 'Binary file added.',
      deleted: 'Binary file deleted.',
      renamed: `Binary file renamed from ${file.oldPath}.`,
      copied: `Binary file copied from ${file.oldPath}.`,
      modified: 'Binary file changed.',
      typeChange: 'Binary file changed.',
    }[file.kind]
  }
  if (file.tooLarge) {
    const delta = file.additions + file.deletions > 0 ? ` (+${file.additions} −${file.deletions})` : ''
    return `This diff is too large to show${delta}.`
  }
  if (file.hunks.length > 0) {
    return null
  }
  switch (file.kind) {
    case 'renamed':
      return `Renamed from ${file.oldPath} without content changes.`
    case 'copied':
      return `Copied from ${file.oldPath} without content changes.`
    case 'added':
      return 'Empty file added.'
    case 'deleted':
      return 'Empty file deleted.'
    default:
      return modeChange.value ? null : 'No content changes.'
  }
})

function expandGap(index: number): void {
  expandedGaps.value = new Set([...expandedGaps.value, index])
}

async function highlight(): Promise<void> {
  const file = props.file
  if (file.hunks.length === 0) {
    return
  }
  const [oldResult, newResult] = await Promise.all([
    file.oldContent !== null ? tokenizeFile(file.oldContent, languageFor(file.oldPath ?? file.path)) : null,
    file.newContent !== null ? tokenizeFile(file.newContent, languageFor(file.path)) : null,
  ])
  oldTokens.value = oldResult
  newTokens.value = newResult
}

onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        observer?.disconnect()
        void highlight()
      }
    },
    // The margin only extends the root's own box, so the root must be the
    // scroll container rather than the viewport that clips it.
    { root: scroller?.value ?? null, rootMargin: `${HIGHLIGHT_MARGIN_PX}px 0px` },
  )
  if (root.value) {
    observer.observe(root.value)
  }
})

onBeforeUnmount(() => observer?.disconnect())

defineExpose({ element: root })
</script>

<template>
  <section ref="root" class="diff-file" :class="{ highlighted }">
    <header class="file-header">
      <button
        class="icon-button"
        type="button"
        :title="collapsed ? 'Show this file' : 'Hide this file'"
        :aria-expanded="!collapsed"
        @click="collapsed = !collapsed"
      >
        <AppIcon :name="collapsed ? 'chevronRight' : 'chevronDown'" />
      </button>
      <span class="kind" :class="`kind--${file.kind}`">{{ KIND_LABELS[file.kind] }}</span>
      <span class="path ellipsis" :title="file.path">
        <template v-if="file.oldPath">
          <span class="muted">{{ file.oldPath }}</span>
          <span class="faint"> → </span>
        </template>
        {{ file.path }}
      </span>
      <span class="stats">
        <span class="additions">+{{ file.additions }}</span>
        <span class="deletions">−{{ file.deletions }}</span>
      </span>
    </header>

    <div v-if="!collapsed" class="file-body">
      <p v-if="modeChange" class="note">{{ modeChange }}</p>
      <p v-if="summary" class="note">{{ summary }}</p>
      <div v-if="file.hunks.length > 0 && !file.tooLarge" class="code-scroll">
        <table class="code" :style="{ '--gutter-width': gutterWidth }">
          <tbody>
            <template v-for="row in rows" :key="row.key">
              <tr v-if="row.type === 'hunk'" class="hunk-row">
                <td class="gutter" colspan="2"></td>
                <td class="hunk-text" colspan="2">
                  {{ row.header }}<span v-if="row.section" class="section">{{ row.section }}</span>
                </td>
              </tr>
              <tr v-else-if="row.type === 'gap'" class="gap-row">
                <td colspan="4">
                  <button class="gap-button" type="button" @click="expandGap(row.gapIndex)">
                    <AppIcon name="expand" />
                    Show {{ row.count }} unchanged {{ row.count === 1 ? 'line' : 'lines' }}
                  </button>
                </td>
              </tr>
              <tr v-else class="line" :class="`line--${row.line.kind}`">
                <td class="gutter">{{ row.line.oldLine ?? '' }}</td>
                <td class="gutter">{{ row.line.newLine ?? '' }}</td>
                <td class="sign">{{ row.line.kind === 'added' ? '+' : row.line.kind === 'removed' ? '−' : '' }}</td>
                <td class="text"><span
                  v-for="(segment, index) in row.segments"
                  :key="index"
                  :class="{ changed: segment.changed, italic: segment.italic, bold: segment.bold }"
                  :style="segment.color ? { color: segment.color } : undefined"
                >{{ segment.text }}</span></td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>

<style scoped>
.diff-file {
  margin-bottom: var(--space-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-large);
  background: var(--color-bg-sunken);
}

.diff-file.highlighted {
  border-color: var(--color-accent);
}

.file-header {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3) var(--space-1) var(--space-1);
  border-bottom: 1px solid var(--color-border);
  border-radius: var(--radius-large) var(--radius-large) 0 0;
  background: var(--color-bg-raised);
}

.kind {
  flex: none;
  padding: 0 var(--space-1);
  border-radius: var(--radius);
  background: var(--color-bg-active);
  font-size: var(--font-size-tiny);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.kind--added {
  color: var(--color-status-added);
}

.kind--deleted {
  color: var(--color-status-deleted);
}

.kind--modified {
  color: var(--color-status-modified);
}

.kind--renamed,
.kind--copied {
  color: var(--color-status-renamed);
}

.kind--typeChange {
  color: var(--color-status-type-change);
}

.path {
  flex: 1;
  min-width: 0;
  color: var(--color-fg-strong);
}

.stats {
  display: flex;
  flex: none;
  gap: var(--space-2);
  font-size: var(--font-size-small);
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}

.note {
  margin: 0;
  padding: var(--space-2) var(--space-4);
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.note + .code-scroll {
  border-top: 1px solid var(--color-border-subtle);
}

.code-scroll {
  overflow-x: auto;
}

.code {
  min-width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-small);
  line-height: 1.55;
  letter-spacing: 0;
}

.gutter {
  width: var(--gutter-width);
  min-width: var(--gutter-width);
  padding: 0 var(--space-2);
  color: var(--color-fg-faint);
  text-align: right;
  vertical-align: top;
  user-select: none;
}

.sign {
  width: 2ch;
  padding: 0 var(--space-1);
  text-align: center;
  vertical-align: top;
  user-select: none;
}

.text {
  width: 100%;
  padding: 0 var(--space-4) 0 var(--space-1);
  color: var(--color-syntax-variable);
  white-space: pre;
  tab-size: 4;
}

.line--added .text {
  background: var(--color-diff-added-bg);
}

.line--added .gutter,
.line--added .sign {
  background: var(--color-diff-added-gutter);
  color: var(--color-diff-added-sign);
}

.line--removed .text {
  background: var(--color-diff-removed-bg);
}

.line--removed .gutter,
.line--removed .sign {
  background: var(--color-diff-removed-gutter);
  color: var(--color-diff-removed-sign);
}

.line--added .changed {
  border-radius: calc(3 * var(--px));
  background: var(--color-diff-added-word);
}

.line--removed .changed {
  border-radius: calc(3 * var(--px));
  background: var(--color-diff-removed-word);
}

.italic {
  font-style: italic;
}

.bold {
  font-weight: 600;
}

.hunk-row td {
  padding-block: calc(3 * var(--px));
  background: var(--color-diff-hunk-bg);
  color: var(--color-diff-hunk-fg);
}

.hunk-text {
  padding: 0 var(--space-2);
  white-space: pre;
}

/* A margin, because Vue drops a space that starts an element. */
.section {
  margin-left: 1ch;
  color: var(--color-fg-muted);
}

.gap-row td {
  padding: 0;
  background: var(--color-diff-gap-bg);
}

.gap-button {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: calc(3 * var(--px)) var(--space-3);
  border: none;
  background: transparent;
  color: var(--color-link);
  font-size: var(--font-size-small);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.gap-button:hover {
  background: var(--color-bg-hover);
}
</style>
