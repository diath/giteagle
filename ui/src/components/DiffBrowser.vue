<script setup lang="ts">
import { provide, ref } from 'vue'

import type { DiffStats, FileDiff } from '../api'
import DiffFile, { DIFF_SCROLLER } from './DiffFile.vue'

withDefaults(
  defineProps<{
    files: FileDiff[]
    stats: DiffStats
    emptyText: string
    /** The file to mark, e.g. the one a file history was opened for. */
    highlighted?: number
  }>(),
  { highlighted: -1 },
)

defineSlots<{
  /** Scrolls with the diffs, above the first file. */
  header?: () => unknown
  /** Shown after a file's name in the list. */
  'file-label'?: (props: { file: FileDiff }) => unknown
}>()

const scroller = ref<HTMLElement | null>(null)
// Component instances, not elements: a child's own template ref is only
// filled in after this function ref runs, so elements are read on use.
const fileComponents: (InstanceType<typeof DiffFile> | null)[] = []
const currentFile = ref(0)

provide(DIFF_SCROLLER, scroller)

function setFileComponent(index: number, component: unknown): void {
  fileComponents[index] = component as InstanceType<typeof DiffFile> | null
}

// The diff area is the sections' offset parent, so offsetTop is already
// relative to its scroll position.
function scrollToFile(index: number): void {
  const element = fileComponents[index]?.element
  if (element && scroller.value) {
    scroller.value.scrollTo({ top: element.offsetTop })
    currentFile.value = index
  }
}

/** Marks the file whose header is at the top of the diff as current. */
function trackCurrentFile(): void {
  if (!scroller.value) {
    return
  }
  const top = scroller.value.scrollTop + 1
  let index = 0
  fileComponents.forEach((component, elementIndex) => {
    const element = component?.element
    if (element && element.offsetTop <= top) {
      index = elementIndex
    }
  })
  currentFile.value = index
}

function baseName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1)
}

function directoryName(path: string): string {
  return path.slice(0, Math.max(path.lastIndexOf('/'), 0))
}

defineExpose({ scrollToFile })
</script>

<template>
  <div class="diff-browser">
    <aside class="file-list" aria-label="Changed files">
      <div class="file-list-header faint">
        {{ stats.filesChanged }} {{ stats.filesChanged === 1 ? 'file' : 'files' }} changed
      </div>
      <button
        v-for="(file, index) in files"
        :key="file.path + index"
        class="file-entry"
        :class="{ current: index === currentFile, 'highlighted-entry': index === highlighted }"
        type="button"
        :title="file.oldPath ? `${file.oldPath} → ${file.path}` : file.path"
        @click="scrollToFile(index)"
      >
        <span class="file-path ellipsis">
          <span class="file-name">{{ baseName(file.path) }}</span>
          <span class="file-directory faint">{{ directoryName(file.path) }}</span>
        </span>
        <slot name="file-label" :file="file" />
        <span class="file-delta">
          (<span class="additions">+{{ file.additions }}</span>
          <span class="deletions">−{{ file.deletions }}</span>)
        </span>
      </button>
    </aside>

    <div ref="scroller" class="diff-area" @scroll.passive="trackCurrentFile">
      <slot name="header" />

      <p v-if="files.length === 0" class="empty-state">{{ emptyText }}</p>
      <DiffFile
        v-for="(file, index) in files"
        :ref="(component) => setFileComponent(index, component)"
        :key="file.path + index"
        :file="file"
        :highlighted="index === highlighted"
      />
    </div>
  </div>
</template>

<style scoped>
.diff-browser {
  display: flex;
  flex: 1;
  min-height: 0;
}

.file-list {
  flex: none;
  width: clamp(calc(275 * var(--px)), 24%, calc(450 * var(--px)));
  overflow-y: auto;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.file-list-header {
  padding: var(--space-3) var(--space-3) var(--space-2);
  font-size: var(--font-size-small);
}

.file-entry {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  width: 100%;
  padding: calc(4 * var(--px)) var(--space-3);
  border: none;
  border-left: calc(3 * var(--px)) solid transparent;
  background: transparent;
  font-size: var(--font-size-small);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.file-entry:hover {
  background: var(--color-bg-hover);
}

.file-entry.current {
  border-left-color: var(--color-accent);
  background: var(--color-bg-raised);
}

.file-entry.highlighted-entry .file-name {
  color: var(--color-accent-bright);
}

.file-entry:focus-visible {
  outline-offset: -2px;
}

.file-path {
  flex: 1;
  min-width: 0;
}

.file-name {
  color: var(--color-fg);
}

/* A margin, because Vue drops a space that starts an element. */
.file-directory {
  margin-left: 1ch;
}

.file-delta {
  flex: none;
  color: var(--color-fg-faint);
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}

.diff-area {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 0 var(--space-4) var(--space-4);
}
</style>
