<script setup lang="ts">
import { computed } from 'vue'

import { repo } from '../stores/repo'

const overview = computed(() => repo.overview)

const commitCountLabel = computed(() => {
  const count = overview.value?.commitCount ?? 0
  return `${count.toLocaleString('en')} ${count === 1 ? 'commit' : 'commits'}`
})
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
</style>
