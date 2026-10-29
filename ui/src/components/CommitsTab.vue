<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { api, type HistoryCursor } from '../api'
import { openCommit, openPendingChanges } from '../stores/navigation'
import { repo } from '../stores/repo'
import { pendingSummary } from '../stores/workingTree'
import { hasPendingChanges } from '../utils/pending'
import CommitList from './CommitList.vue'
import PendingEntry from './PendingEntry.vue'
import SearchField from './SearchField.vue'

const SEARCH_DELAY_MS = 250
const PLACEHOLDER = 'Search commits - author:"name" message:"text" since:-7d'

defineProps<{ active: boolean }>()

const query = ref('')
const appliedQuery = ref('')
let searchTimer: number | undefined

watch(query, (value) => {
  window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    appliedQuery.value = value.trim()
  }, SEARCH_DELAY_MS)
})

const sourceKey = computed(() => `${repo.revision}\n${appliedQuery.value}`)
const load = (cursor: HistoryCursor | null) => api.history(cursor, appliedQuery.value)

// Hidden while searching, since uncommitted changes match no query.
const showPending = computed(() => !appliedQuery.value && hasPendingChanges(pendingSummary.value))

const emptyText = computed(() => {
  if (appliedQuery.value) {
    return 'No commits match this search.'
  }
  return repo.overview?.head.kind === 'unborn' ? 'This repository has no commits yet.' : 'No commits.'
})
</script>

<template>
  <div class="commits-tab">
    <div class="toolbar">
      <SearchField v-model="query" :placeholder="PLACEHOLDER" />
    </div>
    <CommitList
      class="list"
      :load="load"
      :source-key="sourceKey"
      :refresh-key="repo.metadataRevision"
      :active="active"
      :upstream="repo.overview?.head.upstream ?? null"
      :empty-text="emptyText"
      @open="(commit) => openCommit(commit.id)"
    >
      <template #top>
        <PendingEntry v-if="showPending && pendingSummary" :summary="pendingSummary" @open="openPendingChanges" />
      </template>
    </CommitList>
  </div>
</template>

<style scoped>
.commits-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
}

.list {
  flex: 1;
  min-height: 0;
}
</style>
