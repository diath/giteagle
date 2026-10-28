<script setup lang="ts">
import { computed, nextTick, onMounted, provide, ref, shallowRef } from 'vue'

import { api, errorMessage, type CommitDetails } from '../api'
import AppIcon from '../components/AppIcon.vue'
import AvatarStack from '../components/AvatarStack.vue'
import DiffFile, { DIFF_SCROLLER } from '../components/DiffFile.vue'
import { goBack, openCommit } from '../stores/navigation'
import { formatAbsolute, formatRelative } from '../utils/dates'
import { useEscape } from '../utils/escape'

const props = defineProps<{ id: string; fromFile: string | null }>()

const details = shallowRef<CommitDetails | null>(null)
const error = ref<string | null>(null)
const scroller = ref<HTMLElement | null>(null)
// Component instances, not elements: a child's own template ref is only
// filled in after this function ref runs, so elements are read on use.
const fileComponents: (InstanceType<typeof DiffFile> | null)[] = []
const currentFile = ref(0)

useEscape(goBack)

const committerDiffers = computed(() => {
  const commit = details.value
  if (!commit) {
    return false
  }
  const author = commit.authors[0]
  return (
    commit.committerTime !== commit.authorTime ||
    author.email.toLowerCase() !== commit.committer.email.toLowerCase()
  )
})

const fromFileIndex = computed(() => {
  if (!details.value || !props.fromFile) {
    return -1
  }
  return details.value.files.findIndex(
    (file) => file.path === props.fromFile || file.oldPath === props.fromFile,
  )
})

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

async function copyId(): Promise<void> {
  if (details.value) {
    await navigator.clipboard.writeText(details.value.id)
  }
}

onMounted(async () => {
  try {
    details.value = await api.commit(props.id)
  } catch (caught) {
    error.value = errorMessage(caught)
    return
  }
  await nextTick()
  if (fromFileIndex.value !== -1) {
    scrollToFile(fromFileIndex.value)
  }
})
</script>

<template>
  <div class="commit-view">
    <div class="top-bar">
      <button class="button button--quiet" type="button" title="Back (Esc)" @click="goBack">
        <AppIcon name="back" />
        Back
      </button>
      <span class="title">
        Commit <span class="hash">{{ details?.shortId ?? id.slice(0, 7) }}</span>
      </span>
      <span v-if="fromFile" class="from-file">
        <AppIcon name="history" />
        Opened from file history: <span class="path">{{ fromFile }}</span>
      </span>
    </div>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="!details" class="loading faint">Loading commit…</p>

    <div v-else class="layout">
      <aside class="file-list" aria-label="Changed files">
        <div class="file-list-header faint">
          {{ details.stats.filesChanged }} {{ details.stats.filesChanged === 1 ? 'file' : 'files' }} changed
        </div>
        <button
          v-for="(file, index) in details.files"
          :key="file.path + index"
          class="file-entry"
          :class="{ current: index === currentFile, 'from-file-entry': index === fromFileIndex }"
          type="button"
          :title="file.oldPath ? `${file.oldPath} → ${file.path}` : file.path"
          @click="scrollToFile(index)"
        >
          <span class="file-path ellipsis">
            <span class="file-name">{{ baseName(file.path) }}</span>
            <span class="file-directory faint">{{ directoryName(file.path) }}</span>
          </span>
          <span class="file-delta">
            (<span class="additions">+{{ file.additions }}</span>
            <span class="deletions">−{{ file.deletions }}</span>)
          </span>
        </button>
      </aside>

      <div ref="scroller" class="diff-area" @scroll.passive="trackCurrentFile">
        <header class="commit-header">
          <h1 class="subject">{{ details.subject }}</h1>
          <pre v-if="details.body" class="body">{{ details.body }}</pre>

          <div class="authors">
            <AvatarStack :authors="details.authors" :size="30" />
            <div class="author-names">
              <span v-for="author in details.authors" :key="author.email" class="author">
                {{ author.name }} <span class="faint">&lt;{{ author.email }}&gt;</span>
              </span>
            </div>
          </div>

          <dl class="meta">
            <dt>Commit</dt>
            <dd>
              <span class="hash full-hash">{{ details.id }}</span>
              <button class="icon-button" type="button" title="Copy the commit id" @click="copyId">
                <AppIcon name="copy" />
              </button>
            </dd>
            <dt>{{ details.parents.length === 1 ? 'Parent' : 'Parents' }}</dt>
            <dd>
              <span v-if="details.parents.length === 0" class="faint">none (root commit)</span>
              <button
                v-for="parent in details.parents"
                :key="parent.id"
                class="parent hash"
                type="button"
                title="Open this parent commit"
                @click="openCommit(parent.id)"
              >
                {{ parent.shortId }}
              </button>
              <span v-if="details.parents.length > 1" class="faint">(diff against the first parent)</span>
            </dd>
            <dt>Authored</dt>
            <dd>{{ formatAbsolute(details.authorTime) }} <span class="faint">({{ formatRelative(details.authorTime) }})</span></dd>
            <template v-if="committerDiffers">
              <dt>Committed</dt>
              <dd>
                {{ formatAbsolute(details.committerTime) }}
                <span class="faint">({{ formatRelative(details.committerTime) }})</span>
                <span class="muted"> by {{ details.committer.name }}</span>
              </dd>
            </template>
            <dt>Changes</dt>
            <dd>
              {{ details.stats.filesChanged }} {{ details.stats.filesChanged === 1 ? 'file' : 'files' }},
              <span class="additions">+{{ details.stats.additions }}</span>
              <span class="deletions">−{{ details.stats.deletions }}</span>
            </dd>
          </dl>
        </header>

        <p v-if="details.files.length === 0" class="empty-state">This commit changes no files.</p>
        <DiffFile
          v-for="(file, index) in details.files"
          :ref="(component) => setFileComponent(index, component)"
          :key="file.path + index"
          :file="file"
          :highlighted="index === fromFileIndex"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.commit-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.top-bar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--bar-height);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.title {
  color: var(--color-fg-strong);
  font-weight: 600;
}

.from-file {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  margin-left: auto;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius);
  background: var(--color-accent-muted);
  color: var(--color-accent-bright);
  font-size: var(--font-size-small);
  white-space: nowrap;
}

.from-file .path {
  overflow: hidden;
  color: var(--color-fg-strong);
  text-overflow: ellipsis;
}

.error {
  margin: 0;
  padding: var(--space-4);
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.loading {
  margin: 0;
  padding: var(--space-4);
}

.layout {
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

.file-entry.from-file-entry .file-name {
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

.commit-header {
  padding: var(--space-4) 0;
}

.subject {
  margin: 0 0 var(--space-2);
  color: var(--color-fg-strong);
  font-size: var(--font-size-title);
  font-weight: 600;
  line-height: 1.35;
  user-select: text;
}

.body {
  margin: 0 0 var(--space-4);
  color: var(--color-fg-muted);
  font: inherit;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}

.authors {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}

.author-names {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1) var(--space-4);
}

.author {
  color: var(--color-fg);
}

.meta {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-1) var(--space-4);
  margin: 0;
  font-size: var(--font-size-small);
}

.meta dt {
  color: var(--color-fg-faint);
}

.meta dd {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
}

.full-hash {
  user-select: text;
}

.parent {
  padding: 0 var(--space-1);
  border: none;
  border-radius: var(--radius);
  background: transparent;
  cursor: pointer;
  transition: background-color var(--transition);
}

.parent:hover {
  background: var(--color-bg-hover);
}
</style>
