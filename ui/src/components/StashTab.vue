<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type StashSummary } from '../api'
import { openStash } from '../stores/navigation'
import { capabilities, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { pendingSummary, refreshWorkingTree } from '../stores/workingTree'
import { hasPendingChanges } from '../utils/pending'
import AppIcon from './AppIcon.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import StashEntry from './StashEntry.vue'

const props = defineProps<{ active: boolean }>()

const stashes = shallowRef<StashSummary[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const dialogOpen = ref(false)
const stashing = ref(false)
const message = ref('')
const includeUntracked = ref(false)
/** The `sourceKey` the list was last loaded for. */
let loadedKey: string | null = null
let generation = 0

const sourceKey = computed(() => `${repo.revision}:${repo.stashRevision}`)

const stashCountLabel = computed(() => {
  const count = stashes.value.length
  return `${count.toLocaleString('en')} ${count === 1 ? 'stash' : 'stashes'}`
})

const hasChanges = computed(() => hasPendingChanges(pendingSummary.value))

const stashTitle = computed(() => {
  if (!hasChanges.value) {
    return 'There are no uncommitted changes to stash'
  }
  return capabilities.value.canStash
    ? 'Save the uncommitted changes in a new stash'
    : 'Stashing is unavailable right now'
})

const untrackedCount = computed(() => pendingSummary.value?.untracked ?? 0)

/** Untracked files are left out unless included, which can leave nothing. */
const onlyUntracked = computed(() => {
  const summary = pendingSummary.value
  return summary !== null && summary.staged + summary.unstaged + summary.conflicted === 0
})

async function load(): Promise<void> {
  const startedGeneration = (generation += 1)
  loadedKey = sourceKey.value
  loading.value = true
  error.value = null
  try {
    const listed = await api.stashes()
    if (startedGeneration === generation) {
      stashes.value = listed
    }
  } catch (caught) {
    if (startedGeneration === generation) {
      error.value = errorMessage(caught)
      stashes.value = []
    }
  } finally {
    if (startedGeneration === generation) {
      loading.value = false
    }
  }
}

function openDialog(): void {
  message.value = ''
  includeUntracked.value = false
  dialogOpen.value = true
}

async function stashChanges(): Promise<void> {
  stashing.value = true
  repo.busy = 'stash'
  try {
    await api.saveStash(message.value, includeUntracked.value)
    showToast('success', 'Stashed the uncommitted changes as stash@{0}.')
    dialogOpen.value = false
  } catch (caught) {
    showToast('error', errorMessage(caught))
  } finally {
    stashing.value = false
    repo.busy = null
    repo.stashRevision += 1
    await refreshWorkingTree()
  }
}

// Loads when the tab is shown, and again when it is shown after the stash or
// the repository changed.
watch(
  [() => props.active, sourceKey],
  ([active, key]) => {
    if (active && loadedKey !== key) {
      void load()
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="stash-tab">
    <div class="toolbar">
      <span v-if="loading && stashes.length === 0">Loading stashes…</span>
      <span v-else-if="!error">{{ stashCountLabel }}</span>
      <button
        class="button stash-button"
        type="button"
        :disabled="!capabilities.canStash || !hasChanges"
        :title="stashTitle"
        @click="openDialog"
      >
        <AppIcon name="stash" />
        Stash Changes
      </button>
    </div>

    <div class="list">
      <div v-if="error" class="status status--error">
        <span>{{ error }}</span>
        <button class="button" type="button" @click="load">Retry</button>
      </div>
      <p v-else-if="!loading && stashes.length === 0" class="empty-state">The stash is empty.</p>

      <StashEntry v-for="stash in stashes" :key="stash.id" :stash="stash" @open="openStash(stash.id)" />
    </div>

    <ConfirmDialog
      v-if="dialogOpen"
      title="Stash the uncommitted changes"
      icon="stash"
      confirm-label="Stash"
      working-label="Stashing…"
      :working="stashing"
      :disabled="onlyUntracked && !includeUntracked"
      @confirm="stashChanges"
      @close="dialogOpen = false"
    >
      <p>Saves the changes to tracked files in a new stash and resets those files to the last commit.</p>

      <label class="field">
        <span class="muted">Message</span>
        <input
          v-model="message"
          class="text-input"
          type="text"
          spellcheck="false"
          autocomplete="off"
          placeholder="Optional - named after the last commit otherwise"
          :disabled="stashing"
        />
      </label>

      <label v-if="untrackedCount > 0" class="checkbox">
        <input v-model="includeUntracked" type="checkbox" :disabled="stashing" />
        <span>
          Also stash the {{ untrackedCount }} untracked {{ untrackedCount === 1 ? 'file' : 'files' }}, removing
          {{ untrackedCount === 1 ? 'it' : 'them' }} from the working tree
        </span>
      </label>
      <p v-if="onlyUntracked && !includeUntracked" class="faint">
        Only untracked files changed, so include them to have something to stash.
      </p>
    </ConfirmDialog>
  </div>
</template>

<style scoped>
.stash-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 1ch;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.stash-button {
  margin-left: auto;
  font-size: var(--font-size);
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

.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.checkbox {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
}

.checkbox input {
  accent-color: var(--color-accent);
}
</style>
