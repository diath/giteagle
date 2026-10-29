<script setup lang="ts">
import { computed } from 'vue'

import { api } from '../api'
import { repo } from '../stores/repo'
import { formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'

const overview = computed(() => repo.overview)

const commitCountLabel = computed(() => {
  const count = overview.value?.commitCount ?? 0
  return `${count.toLocaleString('en')} ${count === 1 ? 'commit' : 'commits'}`
})

const fetchLabel = computed(() => {
  if (repo.fetch.inProgress) {
    return 'Fetching…'
  }
  if (repo.fetch.lastError) {
    return 'Fetch failed'
  }
  if (repo.fetch.lastSuccess !== null) {
    return `Fetched ${formatRelative(repo.fetch.lastSuccess)}`
  }
  return 'Not fetched yet'
})

const fetchTitle = computed(() =>
  repo.fetch.lastError
    ? `${repo.fetch.lastError}\n\nClick to try again.`
    : 'Fetches run in the background every 5 minutes. Click to fetch now.',
)

function fetchNow(): void {
  if (!repo.fetch.inProgress) {
    void api.fetchNow()
  }
}
</script>

<template>
  <div v-if="overview" class="branch-info">
    <span class="item">{{ commitCountLabel }}</span>
    <template v-if="overview.isShallow">
      <span class="item faint">(shallow)</span>
    </template>

    <span class="separator" aria-hidden="true"></span>
    <span v-if="overview.remotes.length === 0" class="item faint">no remotes</span>
    <template v-else>
      <span class="item muted" :title="`Remotes: ${overview.remotes.join(', ')}`">
        {{ overview.remotes.length === 1 ? overview.remotes[0] : `${overview.remotes.length} remotes` }}
      </span>
      <template v-if="overview.head.kind === 'branch'">
        <span class="faint">→</span>
        <span v-if="overview.head.upstream" class="upstream ellipsis" :title="overview.head.upstream">
          {{ overview.head.upstream }}
        </span>
        <span v-else class="item faint">no upstream</span>
      </template>
    </template>

    <template v-if="overview.head.ahead !== null && overview.head.behind !== null">
      <span class="separator" aria-hidden="true"></span>
      <span class="item ahead" :class="{ zero: overview.head.ahead === 0 }" title="Commits not pushed yet">
        ↑{{ overview.head.ahead }}
      </span>
      <span class="item behind" :class="{ zero: overview.head.behind === 0 }" title="Commits to pull">
        ↓{{ overview.head.behind }}
      </span>
    </template>

    <button
      v-if="overview.remotes.length > 0"
      class="fetch"
      :class="{ 'fetch--failed': repo.fetch.lastError, 'fetch--running': repo.fetch.inProgress }"
      type="button"
      :title="fetchTitle"
      @click="fetchNow"
    >
      <AppIcon :name="repo.fetch.lastError ? 'warning' : 'refresh'" />
      {{ fetchLabel }}
    </button>
  </div>
</template>

<style scoped>
.branch-info {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-height: calc(43 * var(--px));
  padding: var(--space-1) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-app);
  font-size: var(--font-size-small);
  white-space: nowrap;
}

.separator {
  flex: none;
  width: 1px;
  height: calc(18 * var(--px));
  background: var(--color-border);
}

.item {
  flex: none;
}

/* Everything else keeps its width, so a narrow window cuts the upstream. */
.upstream {
  min-width: 0;
}

.ahead {
  color: var(--color-accent-bright);
}

.behind {
  color: var(--color-link);
}

.ahead.zero,
.behind.zero {
  color: var(--color-fg-faint);
}

.fetch {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-1);
  margin-left: auto;
  padding: calc(3 * var(--px)) var(--space-2);
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--color-fg-faint);
  font-size: var(--font-size-small);
  cursor: pointer;
  transition:
    background-color var(--transition),
    color var(--transition);
}

.fetch:hover {
  background: var(--color-bg-hover);
  color: var(--color-fg-muted);
}

.fetch--failed,
.fetch--failed:hover {
  color: var(--color-warning);
}

.fetch--running {
  color: var(--color-fg-muted);
}
</style>
