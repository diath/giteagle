<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type AuthorStats } from '../api'
import { searchCommits } from '../stores/navigation'
import { repo } from '../stores/repo'
import { formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'
import AvatarStack from './AvatarStack.vue'

const AVATAR_SIZE = 36
/** Emails listed by name before the rest are counted as "N more…". */
const SHOWN_EMAILS = 2

const props = defineProps<{ active: boolean }>()

const authors = shallowRef<AuthorStats[]>([])
const loading = ref(false)
const countingLines = ref(false)
const error = ref<string | null>(null)
/** The `repo.revision` the list was last loaded for. */
let loadedRevision: number | null = null
let generation = 0

const authorCountLabel = computed(() => countLabel(authors.value.length, 'author', 'authors'))

function countLabel(count: number, singular: string, plural: string): string {
  return `${count.toLocaleString('en')} ${count === 1 ? singular : plural}`
}

function emailSummary(emails: string[]): string {
  const shown = emails.slice(0, SHOWN_EMAILS).join(', ')
  const hidden = emails.length - SHOWN_EMAILS
  return hidden > 0 ? `${shown}, ${hidden} more…` : shown
}

function viewCommits(author: AuthorStats): void {
  searchCommits(`author:"${author.emails.join(', ')}"`)
}

async function load(): Promise<void> {
  const startedGeneration = (generation += 1)
  loadedRevision = repo.revision
  loading.value = true
  countingLines.value = false
  error.value = null
  let head: string | null = null
  try {
    const list = await api.authors()
    if (startedGeneration !== generation) {
      return
    }
    authors.value = list.authors
    head = list.head
  } catch (caught) {
    if (startedGeneration === generation) {
      error.value = errorMessage(caught)
      authors.value = []
    }
  } finally {
    if (startedGeneration === generation) {
      loading.value = false
    }
  }
  if (startedGeneration !== generation || head === null) {
    return
  }
  // Counting lines diffs every commit, so the counts fill in after the list.
  // Without them the rows still work, so failures are left silent.
  countingLines.value = true
  try {
    const counted = await api.authorLines(head)
    if (startedGeneration === generation) {
      authors.value = counted.authors
    }
  } catch {
    // The rows keep showing commit counts only.
  } finally {
    if (startedGeneration === generation) {
      countingLines.value = false
    }
  }
}

// The list walks the whole history, so it waits until the tab is shown, and
// reloads only when shown again after the repository changed.
watch(
  [() => props.active, () => repo.revision],
  ([active, revision]) => {
    if (active && loadedRevision !== revision) {
      void load()
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="authors-tab">
    <div class="toolbar">
      <span v-if="loading">Loading authors…</span>
      <span v-else-if="!error">{{ authorCountLabel }}</span>
      <span v-if="countingLines" class="faint">Counting changed lines…</span>
    </div>

    <div class="list">
      <div v-if="error" class="status status--error">
        <span>{{ error }}</span>
        <button class="button" type="button" @click="load">Retry</button>
      </div>
      <p v-else-if="!loading && repo.overview?.head.kind === 'unborn'" class="empty-state">
        This repository has no commits yet.
      </p>

      <div v-for="author in authors" :key="author.person.email" class="author">
        <AvatarStack :authors="[author.person]" :size="AVATAR_SIZE" />
        <div class="content">
          <div class="name-line">
            <span class="name ellipsis">{{ author.person.name }}</span>
            <span class="emails ellipsis" :title="author.emails.join('\n')">({{ emailSummary(author.emails) }})</span>
            <span class="last-commit faint">(last commit {{ formatRelative(author.lastCommitTime) }})</span>
          </div>
          <div class="details">
            {{ countLabel(author.commits, 'commit', 'commits') }}<template v-if="author.lines">,
              <span class="additions">+{{ author.lines.additions.toLocaleString('en') }}</span>,
              <span class="deletions">−{{ author.lines.deletions.toLocaleString('en') }}</span>
            </template>
          </div>
        </div>
        <button class="button" type="button" @click="viewCommits(author)">View Commits</button>
      </div>

      <div v-if="repo.overview?.isShallow && authors.length > 0" class="marker">
        <AppIcon name="warning" />
        <span>Only the fetched history is counted: this is a shallow clone.</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.authors-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  align-items: baseline;
  gap: 1ch;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.status {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
}

.status--error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.author {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
  line-height: var(--line-height-dense);
}

.content {
  flex: 1;
  min-width: 0;
}

.name-line {
  display: flex;
  align-items: baseline;
  gap: 1ch;
}

.name {
  min-width: 0;
  color: var(--color-fg-strong);
}

.emails {
  min-width: 0;
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.last-commit {
  flex: none;
  font-size: var(--font-size-small);
}

.details {
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}

.marker {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-4);
  color: var(--color-warning);
  font-size: var(--font-size-small);
  font-weight: 500;
}
</style>
