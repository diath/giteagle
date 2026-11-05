<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type CommitDetails } from '../api'
import AppIcon from '../components/AppIcon.vue'
import AvatarStack from '../components/AvatarStack.vue'
import DiffBrowser from '../components/DiffBrowser.vue'
import ViewBar from '../components/ViewBar.vue'
import type { CommitSequence } from '../stores/navigation'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import { formatAbsolute, formatRelative } from '../utils/dates'
import { useEscape } from '../utils/escape'

const props = defineProps<{ id: string; fromFile: string | null; sequence: CommitSequence | null }>()

const { api, goBack, openCommit, replaceCommit } = useProject()

const details = shallowRef<CommitDetails | null>(null)
const error = ref<string | null>(null)
const browser = ref<InstanceType<typeof DiffBrowser> | null>(null)
const loadingOlder = ref(false)

useEscape(goBack)

/** Where the commit is in the list it was opened from, newest first. */
const position = computed(() => props.sequence?.commits.findIndex((commit) => commit.id === props.id) ?? -1)

const newerCommit = computed(() => (position.value > 0 ? (props.sequence?.commits[position.value - 1] ?? null) : null))

/** Null at the end of the loaded commits, even when more can be loaded. */
const olderCommit = computed(() =>
  position.value === -1 ? null : (props.sequence?.commits[position.value + 1] ?? null),
)

const canShowOlder = computed(
  () => olderCommit.value !== null || (position.value !== -1 && props.sequence?.hasMore === true),
)

const notInList = computed(() =>
  props.sequence ? "This commit isn't in the list it was opened from" : 'Open a commit from a list to step through it',
)

const olderTitle = computed(() => {
  if (position.value === -1) {
    return notInList.value
  }
  if (olderCommit.value) {
    return `Previous (older) commit: ${olderCommit.value.shortId} ${olderCommit.value.subject}`
  }
  return canShowOlder.value ? 'Previous (older) commit' : 'This is the oldest commit in the list'
})

const newerTitle = computed(() => {
  if (position.value === -1) {
    return notInList.value
  }
  if (newerCommit.value) {
    return `Next (newer) commit: ${newerCommit.value.shortId} ${newerCommit.value.subject}`
  }
  return 'This is the newest commit in the list'
})

async function showOlder(): Promise<void> {
  // At the end of the loaded commits, the list loads its next page first.
  if (!olderCommit.value && props.sequence) {
    loadingOlder.value = true
    try {
      await props.sequence.loadMore()
    } finally {
      loadingOlder.value = false
    }
  }
  if (olderCommit.value) {
    replaceCommit(props.id, olderCommit.value.id)
  }
}

function showNewer(): void {
  if (newerCommit.value) {
    replaceCommit(props.id, newerCommit.value.id)
  }
}

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

async function copyId(): Promise<void> {
  if (!details.value) {
    return
  }
  try {
    await navigator.clipboard.writeText(details.value.id)
    showToast('success', 'Copied the commit hash to the clipboard.')
  } catch (caught) {
    showToast('error', `Couldn't copy the commit hash: ${errorMessage(caught)}`)
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
    browser.value?.scrollToFile(fromFileIndex.value)
  }
})
</script>

<template>
  <div class="commit-view">
    <ViewBar icon="commit">
      <template #before-title>
        <button
          class="button"
          type="button"
          :disabled="!canShowOlder || loadingOlder"
          :title="olderTitle"
          @click="showOlder"
        >
          <AppIcon name="chevronLeft" />
          Previous
        </button>
      </template>
      <template #title>
        Commit <span class="hash">{{ details?.shortId ?? id.slice(0, 7) }}</span>
      </template>
      <template #after-title>
        <button class="button" type="button" :disabled="!newerCommit" :title="newerTitle" @click="showNewer">
          Next
          <AppIcon name="chevronRight" />
        </button>
      </template>
      <template #end>
        <span v-if="fromFile" class="from-file" :title="`Opened from file history: ${fromFile}`">
          <AppIcon name="history" />
          <span class="from-file-text">
            <span>Opened from file history:</span>
            <span class="path ellipsis">{{ fromFile }}</span>
          </span>
        </span>
      </template>
    </ViewBar>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="!details" class="loading faint">Loading commit…</p>

    <DiffBrowser
      v-else
      ref="browser"
      :files="details.files"
      :stats="details.stats"
      :highlighted="fromFileIndex"
      :commit="details"
      empty-text="This commit changes no files."
    >
      <template #header>
        <header class="commit-header">
          <dl class="detail-list">
            <dt>Commit</dt>
            <dd>
              <span class="hash commit-hash" :title="details.id">{{ details.shortId }}</span>
              <button class="icon-button copy" type="button" title="Copy the commit id" @click="copyId">
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
                @click="openCommit(parent.id, { sequence })"
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
              <!-- One span, so the commas aren't flex items set apart by the gap. -->
              <span>
                {{ details.stats.filesChanged }} {{ details.stats.filesChanged === 1 ? 'file' : 'files' }},
                <span class="additions">+{{ details.stats.additions }}</span>,
                <span class="deletions">−{{ details.stats.deletions }}</span>
              </span>
            </dd>
          </dl>

          <h1 class="subject">{{ details.subject }}</h1>
          <pre v-if="details.body" class="body">{{ details.body }}</pre>

          <ul class="authors">
            <li v-for="author in details.authors" :key="author.email" class="author">
              <AvatarStack :authors="[author]" :size="30" />
              <span>{{ author.name }} <span class="faint">&lt;{{ author.email }}&gt;</span></span>
            </li>
          </ul>
        </header>
      </template>
    </DiffBrowser>
  </div>
</template>

<style scoped>
.commit-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.from-file {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius);
  background: var(--color-accent-muted);
  color: var(--color-accent-bright);
  font-size: var(--font-size-small);
  white-space: nowrap;
}

/* When the bar is narrow, the path moves to a line of its own instead of
   being cut short. */
.from-file-text {
  display: flex;
  flex-wrap: wrap;
  column-gap: 1ch;
  min-width: 0;
  line-height: 1.2;
}

.from-file .path {
  min-width: 0;
  max-width: 100%;
  color: var(--color-fg-strong);
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

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
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
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.author {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  color: var(--color-fg);
  overflow-wrap: anywhere;
}

.commit-hash {
  user-select: text;
}

/* The button overhangs its row instead of making it taller than the rest. */
.copy {
  margin-block: calc((1lh - var(--icon-button-size)) / 2);
}

/* The margin cancels the padding, so the hash lines up with the other
   values while its hover background keeps some room. */
.parent {
  margin-inline: calc(-1 * var(--space-1));
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
