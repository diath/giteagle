<script setup lang="ts">
import { computed } from 'vue'

import type { HistoryCursor } from '../api'
import CommitList from '../components/CommitList.vue'
import ViewBar from '../components/ViewBar.vue'
import { useProject } from '../stores/project'
import { useEscape } from '../utils/escape'

const props = defineProps<{ path: string; active: boolean }>()

const { api, goBack, navigation, openCommit, commitSelection, repo } = useProject()

useEscape(goBack)

const sourceKey = computed(() => `${repo.revision}\n${props.path}`)
const load = (cursor: HistoryCursor | null) => api.fileHistory(props.path, cursor)

// The view is only seen on top of the stack, so Back leads to the one below.
const backTitle = computed(() =>
  navigation.stack[navigation.stack.length - 2]?.kind === 'main' ? 'Back to the files (Esc)' : 'Back (Esc)',
)
</script>

<template>
  <div class="file-history">
    <ViewBar icon="history" :back-title="backTitle" :inert="commitSelection.active">
      <template #title>
        File history:
        <span class="path ellipsis" :title="path">{{ path }}</span>
      </template>
    </ViewBar>
    <p class="note faint">Commits on the current branch that changed this path, newest first. Renames are not followed.</p>
    <CommitList
      class="list"
      :load="load"
      :source-key="sourceKey"
      :refresh-key="repo.metadataRevision"
      :active="active"
      :upstream="repo.overview?.head.upstream ?? null"
      empty-text="No commits on this branch changed this file."
      @open="(commit, sequence) => openCommit(commit.id, { fromFile: path, sequence })"
    />
  </div>
</template>

<style scoped>
.file-history {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.note {
  margin: 0;
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  font-size: var(--font-size-small);
}

.list {
  flex: 1;
  min-height: 0;
}
</style>
