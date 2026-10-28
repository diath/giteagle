<script setup lang="ts">
import type { FileStatus } from '../api'

defineProps<{ status: FileStatus }>()

const LABELS: Record<FileStatus, { letter: string; title: string }> = {
  modified: { letter: 'M', title: 'Modified locally' },
  staged: { letter: 'S', title: 'Changes staged' },
  added: { letter: 'A', title: 'Added to the index' },
  deleted: { letter: 'D', title: 'Deleted locally' },
  renamed: { letter: 'R', title: 'Renamed locally' },
  typeChange: { letter: 'T', title: 'File type changed locally' },
  conflicted: { letter: '!', title: 'Conflicted' },
}
</script>

<template>
  <span class="status-badge" :class="`status-badge--${status}`" :title="LABELS[status].title">
    {{ LABELS[status].letter }}
  </span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: 1px solid currentColor;
  border-radius: var(--radius);
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0;
  line-height: 1;
}

.status-badge--modified {
  color: var(--color-status-modified);
}

.status-badge--staged {
  color: var(--color-status-staged);
}

.status-badge--added {
  color: var(--color-status-added);
}

.status-badge--deleted {
  color: var(--color-status-deleted);
}

.status-badge--renamed {
  color: var(--color-status-renamed);
}

.status-badge--typeChange {
  color: var(--color-status-type-change);
}

.status-badge--conflicted {
  color: var(--color-status-conflicted);
}
</style>
