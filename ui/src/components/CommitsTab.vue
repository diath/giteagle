<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import type { HistoryCursor } from '../api'
import { useProject } from '../stores/project'
import { hasPendingChanges } from '../utils/pending'
import AppIcon from './AppIcon.vue'
import CommitList from './CommitList.vue'
import PendingEntry from './PendingEntry.vue'
import SearchField from './SearchField.vue'
import SearchHelpDialog from './SearchHelpDialog.vue'

const SEARCH_DELAY_MS = 250
const PLACEHOLDER = 'Search commits… (try: author:"name", message:"text", since:7d)'

defineProps<{ active: boolean }>()

const { api, navigation, openCommit, openPendingChanges, pickSelection, repo, pendingSummary } = useProject()

const query = ref('')
const appliedQuery = ref('')
const helpOpen = ref(false)
let searchTimer: number | undefined

watch(query, (value) => {
  window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    appliedQuery.value = value.trim()
  }, SEARCH_DELAY_MS)
})

// Searches started from other tabs run at once instead of waiting for a
// pause in typing.
watch(
  () => navigation.commitSearch,
  (search) => {
    if (search === null) {
      return
    }
    navigation.commitSearch = null
    window.clearTimeout(searchTimer)
    query.value = search
    appliedQuery.value = search.trim()
  },
)

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
    <div class="toolbar" :inert="pickSelection.active">
      <SearchField v-model="query" class="search" :placeholder="PLACEHOLDER" />
      <button class="button button--icon" type="button" title="Search tags" @click="helpOpen = true">
        <AppIcon name="help" />
      </button>
    </div>
    <CommitList
      class="list"
      :load="load"
      :source-key="sourceKey"
      :refresh-key="repo.metadataRevision"
      :active="active"
      :upstream="repo.overview?.head.upstream ?? null"
      :empty-text="emptyText"
      @open="(commit, sequence) => openCommit(commit.id, { sequence })"
    >
      <template #top>
        <PendingEntry
          v-if="showPending && pendingSummary"
          :inert="pickSelection.active"
          :summary="pendingSummary"
          @open="openPendingChanges"
        />
      </template>
    </CommitList>
    <SearchHelpDialog v-if="helpOpen" @close="helpOpen = false" />
  </div>
</template>

<style scoped>
.commits-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--color-border);
}

.search {
  flex: 1;
  min-width: 0;
}

.list {
  flex: 1;
  min-height: 0;
}
</style>
