<script setup lang="ts">
import { computed } from 'vue'

import type { PendingSummary } from '../api'
import { pendingCounts } from '../utils/pending'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ summary: PendingSummary }>()
const emit = defineEmits<{ open: [] }>()

const counts = computed(() => pendingCounts(props.summary))
</script>

<template>
  <div
    class="pending-entry"
    role="button"
    tabindex="0"
    title="Browse the uncommitted changes"
    @click="emit('open')"
    @keydown.enter.self="emit('open')"
  >
    <span class="icon">
      <AppIcon name="pencil" />
    </span>
    <div class="content">
      <div class="title">Uncommitted changes</div>
      <div class="details">
        <template v-for="({ state, count }, index) in counts" :key="state">
          {{ index > 0 ? ', ' : '' }}{{ count }} {{ state }}
        </template>
        <span class="separator faint">·</span>
        <span class="additions">+{{ summary.additions }}</span>,
        <span class="deletions">−{{ summary.deletions }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pending-entry {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
  background: var(--color-pending-muted);
  line-height: var(--line-height-dense);
  cursor: pointer;
  transition: filter var(--transition);
}

.pending-entry:hover {
  filter: brightness(1.25);
}

.pending-entry:focus-visible {
  outline-offset: -2px;
}

/* As wide as a commit's avatar with its border, so the text lines up with
   the commits'. */
.icon {
  display: flex;
  flex: none;
  justify-content: center;
  width: calc(29 * var(--px));
  color: var(--color-pending);
}

.content {
  min-width: 0;
}

.title {
  color: var(--color-pending);
  font-weight: 600;
}

.details {
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

/* A margin, because Vue drops the line breaks around it. */
.separator {
  margin-inline: 1ch;
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}
</style>
