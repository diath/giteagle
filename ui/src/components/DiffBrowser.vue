<script setup lang="ts">
import { computed, provide, ref } from 'vue'

import type { DiffStats, FileDiff } from '../api'
import { buildFileTree, treeOrder, visibleRows, type FileTreeFolder, type FileTreeRow } from '../utils/fileTree'
import AppIcon from './AppIcon.vue'
import DiffFile, { DIFF_SCROLLER, KIND_LABELS } from './DiffFile.vue'

const props = withDefaults(
  defineProps<{
    files: FileDiff[]
    stats: DiffStats
    emptyText: string
    /** The file to mark, e.g. the one a file history was opened for. */
    highlighted?: number
    /** The commit the diffs are of, whose files can be viewed whole. */
    commit?: { id: string; shortId: string } | null
    /** The commit that changes other than a commit's are compared with, whose files are blamed. */
    base?: { id: string; shortId: string } | null
  }>(),
  { highlighted: -1, commit: null, base: null },
)

defineSlots<{
  /** Scrolls with the diffs, above the first file. */
  header?: () => unknown
  /** Shown after a file's name in the list. */
  'file-label'?: (props: { file: FileDiff }) => unknown
}>()

const tree = computed(() => buildFileTree(props.files.map((file) => file.path)))
/** Positions in `files`, in the order the list shows them. The diffs follow the same order. */
const order = computed(() => treeOrder(tree.value))
/** The paths of the collapsed folders. */
const collapsed = ref(new Set<string>())
const rows = computed(() => visibleRows(tree.value, collapsed.value))

const scroller = ref<HTMLElement | null>(null)
// Component instances, not elements: a child's own template ref is only
// filled in after this function ref runs, so elements are read on use.
// Indexed by position in `files`.
const fileComponents: (InstanceType<typeof DiffFile> | null)[] = []
const currentFile = ref(order.value[0] ?? 0)

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
  let current = order.value[0] ?? 0
  for (const index of order.value) {
    const element = fileComponents[index]?.element
    if (element && element.offsetTop <= top) {
      current = index
    }
  }
  currentFile.value = current
}

function toggleFolder(path: string): void {
  if (collapsed.value.has(path)) {
    collapsed.value.delete(path)
  } else {
    collapsed.value.add(path)
  }
}

/** A collapsed folder stands in for the current file when it hides it. */
function hidesCurrentFile(folder: FileTreeFolder): boolean {
  const file = props.files[currentFile.value] as FileDiff | undefined
  return collapsed.value.has(folder.path) && file !== undefined && file.path.startsWith(`${folder.path}/`)
}

// A file can turn into a folder of the same name, so the kinds key apart.
function rowKey(row: FileTreeRow): string {
  return row.node.kind === 'folder' ? `folder:${row.node.path}` : `file:${props.files[row.node.index].path}`
}

function fileTitle(file: FileDiff): string {
  return `${KIND_LABELS[file.kind]}: ${file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}`
}

defineExpose({ scrollToFile })
</script>

<template>
  <div class="diff-browser">
    <aside class="file-list" aria-label="Changed files">
      <div class="file-list-header faint">
        {{ stats.filesChanged }} {{ stats.filesChanged === 1 ? 'file' : 'files' }} changed
      </div>
      <template v-for="row in rows" :key="rowKey(row)">
        <button
          v-if="row.node.kind === 'folder'"
          class="tree-row"
          :class="{ current: hidesCurrentFile(row.node) }"
          :style="{ '--depth': row.depth }"
          type="button"
          :title="row.node.path"
          :aria-expanded="!collapsed.has(row.node.path)"
          @click="toggleFolder(row.node.path)"
        >
          <AppIcon :name="collapsed.has(row.node.path) ? 'chevronRight' : 'chevronDown'" class="chevron" />
          <AppIcon name="folder" class="folder-icon" />
          <span class="folder-name ellipsis">{{ row.node.name }}</span>
        </button>
        <button
          v-else
          class="tree-row"
          :class="{ current: row.node.index === currentFile, 'highlighted-entry': row.node.index === highlighted }"
          :style="{ '--depth': row.depth }"
          type="button"
          :title="fileTitle(files[row.node.index])"
          @click="scrollToFile(row.node.index)"
        >
          <span class="chevron"></span>
          <AppIcon name="file" class="file-icon" :class="`file-icon--${files[row.node.index].kind}`" />
          <span class="file-text">
            <span class="file-name ellipsis">{{ row.node.name }}</span>
            <span v-if="$slots['file-label']" class="file-label ellipsis">
              <slot name="file-label" :file="files[row.node.index]" />
            </span>
          </span>
          <span class="file-delta">
            (<span class="additions">+{{ files[row.node.index].additions }}</span>
            <span class="deletions">−{{ files[row.node.index].deletions }}</span>)
          </span>
        </button>
      </template>
    </aside>

    <div ref="scroller" class="diff-area" @scroll.passive="trackCurrentFile">
      <slot name="header" />

      <p v-if="files.length === 0" class="empty-state">{{ emptyText }}</p>
      <DiffFile
        v-for="index in order"
        :ref="(component) => setFileComponent(index, component)"
        :key="files[index].path"
        :file="files[index]"
        :highlighted="index === highlighted"
        :commit="commit"
        :base="base"
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

/* Each level of the tree steps in by --depth. */
.tree-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  width: 100%;
  padding: calc(2 * var(--px)) var(--space-3) calc(2 * var(--px))
    calc(var(--space-2) + var(--depth) * calc(15 * var(--px)));
  border: none;
  border-left: calc(3 * var(--px)) solid transparent;
  background: transparent;
  font-size: var(--font-size-small);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.tree-row:hover {
  background: var(--color-bg-hover);
}

.tree-row.current {
  border-left-color: var(--color-accent);
  background: var(--color-bg-raised);
}

.tree-row.highlighted-entry .file-name {
  color: var(--color-accent-bright);
}

.tree-row:focus-visible {
  outline-offset: -2px;
}

/* File rows keep the space, so their icons line up with their folders'. */
.chevron {
  flex: none;
  width: var(--icon-size);
  color: var(--color-fg-faint);
}

.folder-icon {
  color: var(--color-accent);
}

.folder-name {
  flex: 1;
  min-width: 0;
  color: var(--color-fg-muted);
}

.file-icon {
  color: var(--color-fg-faint);
}

.file-icon--added {
  color: var(--color-status-added);
}

.file-icon--deleted {
  color: var(--color-status-deleted);
}

.file-icon--modified {
  color: var(--color-status-modified);
}

.file-icon--renamed,
.file-icon--copied {
  color: var(--color-status-renamed);
}

.file-icon--typeChange {
  color: var(--color-status-type-change);
}

/* Only the labels shrink, so a narrow row cuts them before the name, which
   is cut only once it needs the whole width. */
.file-text {
  display: flex;
  flex: 1;
  gap: var(--space-1);
  min-width: 0;
}

.file-name {
  flex: none;
  max-width: 100%;
  color: var(--color-fg);
}

.file-label {
  min-width: 0;
}

.file-delta {
  flex: none;
  color: var(--color-fg-faint);
}

/* A margin, because Vue drops the line break between the counts. */
.file-delta .deletions {
  margin-left: 1ch;
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
