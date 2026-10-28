<script setup lang="ts">
import { computed } from 'vue'

import { api, type HistoryCursor } from '../api'
import AppIcon from '../components/AppIcon.vue'
import CommitList from '../components/CommitList.vue'
import { goBack, openCommit } from '../stores/navigation'
import { repo } from '../stores/repo'
import { useEscape } from '../utils/escape'

const props = defineProps<{ path: string; active: boolean }>()

useEscape(goBack)

const sourceKey = computed(() => `${repo.revision}\n${props.path}`)
const load = (cursor: HistoryCursor | null) => api.fileHistory(props.path, cursor)
</script>

<template>
  <div class="file-history">
    <div class="top-bar">
      <button class="button button--quiet" type="button" title="Back to the files (Esc)" @click="goBack">
        <AppIcon name="back" />
        Back
      </button>
      <span class="title">
        <AppIcon name="history" />
        File history:
        <span class="path">{{ path }}</span>
      </span>
    </div>
    <p class="note faint">Commits on the current branch that changed this path, newest first. Renames are not followed.</p>
    <CommitList
      class="list"
      :load="load"
      :source-key="sourceKey"
      :refresh-key="repo.metadataRevision"
      :active="active"
      :upstream="repo.overview?.head.upstream ?? null"
      empty-text="No commits on this branch changed this file."
      @open="(commit) => openCommit(commit.id, path)"
    />
  </div>
</template>

<style scoped>
.file-history {
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
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius);
  background: var(--color-accent-muted);
  color: var(--color-accent-bright);
  white-space: nowrap;
}

.path {
  overflow: hidden;
  color: var(--color-fg-strong);
  font-weight: 600;
  text-overflow: ellipsis;
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
