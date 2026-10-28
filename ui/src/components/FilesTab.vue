<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type CommitBrief, type DirectoryEntry } from '../api'
import type { IconName } from '../icons/paths'
import { navigation, openFileHistory } from '../stores/navigation'
import { repo } from '../stores/repo'
import { directoryHasChanges, statusOf } from '../stores/workingTree'
import { formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'
import SearchField from './SearchField.vue'
import StatusBadge from './StatusBadge.vue'

const SEARCH_DELAY_MS = 200

const entries = shallowRef<DirectoryEntry[]>([])
const lastCommits = shallowRef<Record<string, CommitBrief>>({})
const loading = ref(false)
const error = ref<string | null>(null)
const searchError = ref<string | null>(null)
const query = ref('')
const searchResults = shallowRef<DirectoryEntry[] | null>(null)
const searchTruncated = ref(false)
let listGeneration = 0
let searchGeneration = 0
let searchTimer: number | undefined

const directory = computed(() => navigation.directory)

const breadcrumbs = computed(() => {
  const crumbs = [{ label: repo.overview?.repoName ?? 'root', path: '' }]
  if (directory.value) {
    const segments = directory.value.split('/')
    segments.forEach((segment, index) => {
      crumbs.push({ label: segment, path: segments.slice(0, index + 1).join('/') })
    })
  }
  return crumbs
})

const parentDirectory = computed(() => {
  const slash = directory.value.lastIndexOf('/')
  return slash === -1 ? '' : directory.value.slice(0, slash)
})

async function loadDirectory(): Promise<void> {
  const generation = (listGeneration += 1)
  const path = directory.value
  loading.value = true
  error.value = null
  try {
    const listed = await api.listDirectory(path)
    if (generation !== listGeneration) {
      return
    }
    entries.value = listed
    lastCommits.value = {}
    loading.value = false
  } catch (caught) {
    if (generation !== listGeneration) {
      return
    }
    // The directory may be gone after a switch or reload; fall back to the root.
    if (path !== '') {
      navigation.directory = ''
      return
    }
    error.value = errorMessage(caught)
    entries.value = []
  } finally {
    if (generation === listGeneration) {
      loading.value = false
    }
  }
  if (generation !== listGeneration || error.value !== null) {
    return
  }
  // Last commits take a history walk, so they fill in after the listing.
  // Without them the rows still work, so failures are left silent.
  try {
    const commits = await api.lastCommits(path)
    if (generation === listGeneration) {
      lastCommits.value = commits
    }
  } catch {
    // Nothing to show; the columns stay empty.
  }
}

async function runSearch(): Promise<void> {
  const generation = (searchGeneration += 1)
  const text = query.value.trim()
  searchError.value = null
  if (!text) {
    searchResults.value = null
    return
  }
  try {
    const result = await api.searchFiles(directory.value, text)
    if (generation === searchGeneration) {
      searchResults.value = result.entries
      searchTruncated.value = result.truncated
    }
  } catch (caught) {
    if (generation === searchGeneration) {
      searchResults.value = []
      searchError.value = errorMessage(caught)
    }
  }
}

watch(query, () => {
  window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(runSearch, SEARCH_DELAY_MS)
})

watch([directory, () => repo.revision], () => {
  void loadDirectory()
  if (query.value.trim()) {
    void runSearch()
  }
}, { immediate: true })

function navigate(path: string): void {
  // Discards any search still running for the old directory.
  searchGeneration += 1
  window.clearTimeout(searchTimer)
  query.value = ''
  searchResults.value = null
  searchError.value = null
  navigation.directory = path
}

function open(entry: DirectoryEntry): void {
  if (entry.kind === 'directory') {
    navigate(entry.path)
  } else if (entry.kind !== 'submodule') {
    openFileHistory(entry.path)
  }
}

const ICONS: Record<DirectoryEntry['kind'], IconName> = {
  directory: 'folder',
  file: 'file',
  symlink: 'symlink',
  submodule: 'submodule',
}

function relativeToDirectory(path: string): string {
  return directory.value ? path.slice(directory.value.length + 1) : path
}
</script>

<template>
  <div class="files-tab">
    <div class="toolbar">
      <nav class="breadcrumbs" aria-label="Directory">
        <template v-for="(crumb, index) in breadcrumbs" :key="crumb.path">
          <span v-if="index > 0" class="faint">/</span>
          <button
            class="crumb"
            type="button"
            :class="{ current: index === breadcrumbs.length - 1 }"
            @click="navigate(crumb.path)"
          >
            {{ crumb.label }}
          </button>
        </template>
      </nav>
      <SearchField
        v-model="query"
        class="search"
        :placeholder="directory ? `Search files in ${directory}/…` : 'Search files…'"
      />
    </div>

    <div class="list">
      <p v-if="error" class="message error">{{ error }}</p>

      <template v-if="searchResults !== null">
        <p v-if="searchError" class="message error">{{ searchError }}</p>
        <p v-else-if="searchResults.length === 0" class="empty-state">No tracked files match.</p>
        <button
          v-for="entry in searchResults"
          :key="entry.path"
          class="row row--search"
          type="button"
          @click="open(entry)"
        >
          <AppIcon :name="ICONS[entry.kind]" class="icon" />
          <span class="name ellipsis">{{ relativeToDirectory(entry.path) }}</span>
          <StatusBadge v-if="statusOf(entry.path)" :status="statusOf(entry.path)!" />
        </button>
        <p v-if="searchTruncated" class="message faint">Showing the first {{ searchResults.length }} matches.</p>
      </template>

      <template v-else>
        <p v-if="repo.overview?.head.kind === 'unborn'" class="empty-state">This repository has no commits yet.</p>
        <button v-if="directory" class="row" type="button" title="Parent directory" @click="navigate(parentDirectory)">
          <AppIcon name="parent" class="icon" />
          <span class="name">..</span>
        </button>
        <button
          v-for="entry in entries"
          :key="entry.path"
          class="row"
          type="button"
          :disabled="entry.kind === 'submodule'"
          :title="entry.kind === 'submodule' ? 'Submodules are not supported' : entry.path"
          @click="open(entry)"
        >
          <AppIcon :name="ICONS[entry.kind]" class="icon" :class="`icon--${entry.kind}`" />
          <span class="name ellipsis">{{ entry.name }}</span>
          <span class="status">
            <StatusBadge v-if="statusOf(entry.path)" :status="statusOf(entry.path)!" />
            <span
              v-else-if="entry.kind === 'directory' && directoryHasChanges(entry.path)"
              class="changes-dot"
              title="Contains local changes"
            ></span>
          </span>
          <span v-if="lastCommits[entry.name]" class="commit ellipsis">
            <span class="hash">{{ lastCommits[entry.name].shortId }}</span><span class="faint">:</span>
            {{ lastCommits[entry.name].subject }}
          </span>
          <span v-else class="commit"></span>
          <span class="date faint">
            {{ lastCommits[entry.name] ? formatRelative(lastCommits[entry.name].authorTime) : '' }}
          </span>
        </button>
        <p v-if="loading && entries.length === 0" class="message faint">Loading…</p>
      </template>
    </div>
  </div>
</template>

<style scoped>
.files-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
}

.breadcrumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--font-size-small);
}

.crumb {
  padding: 0 var(--space-1);
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--color-link);
  cursor: pointer;
  transition: background-color var(--transition);
}

.crumb:hover {
  background: var(--color-bg-hover);
}

.crumb.current {
  color: var(--color-fg-strong);
  font-weight: 600;
}

.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.message {
  margin: 0;
  padding: var(--space-3) var(--space-4);
}

.error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.row {
  display: grid;
  grid-template-columns: var(--icon-size) minmax(10ch, 1fr) var(--icon-size) minmax(0, 2fr) 15ch;
  gap: var(--space-3);
  align-items: center;
  width: 100%;
  min-height: 40px;
  padding: var(--space-1) var(--space-4);
  border: none;
  border-bottom: 1px solid var(--color-border-subtle);
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.row--search {
  grid-template-columns: var(--icon-size) minmax(0, 1fr) var(--icon-size);
}

.row:hover:not(:disabled) {
  background: var(--color-bg-hover);
}

.row:disabled {
  cursor: default;
}

.row:focus-visible {
  outline-offset: -2px;
}

.icon {
  color: var(--color-fg-faint);
}

.icon--directory {
  color: var(--color-accent);
}

.name {
  color: var(--color-fg-strong);
}

.status {
  display: flex;
  justify-content: center;
}

.changes-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-status-modified);
}

.commit {
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.date {
  font-size: var(--font-size-small);
  text-align: right;
}
</style>
