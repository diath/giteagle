<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type StashSummary } from '../api'
import { openStash } from '../stores/navigation'
import { repo } from '../stores/repo'
import StashEntry from './StashEntry.vue'

const props = defineProps<{ active: boolean }>()

const stashes = shallowRef<StashSummary[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
/** The `sourceKey` the list was last loaded for. */
let loadedKey: string | null = null
let generation = 0

const sourceKey = computed(() => `${repo.revision}:${repo.stashRevision}`)

const stashCountLabel = computed(() => {
  const count = stashes.value.length
  return `${count.toLocaleString('en')} ${count === 1 ? 'stash' : 'stashes'}`
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
    </div>

    <div class="list">
      <div v-if="error" class="status status--error">
        <span>{{ error }}</span>
        <button class="button" type="button" @click="load">Retry</button>
      </div>
      <p v-else-if="!loading && stashes.length === 0" class="empty-state">The stash is empty.</p>

      <StashEntry v-for="stash in stashes" :key="stash.id" :stash="stash" @open="openStash(stash.id)" />
    </div>
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
</style>
