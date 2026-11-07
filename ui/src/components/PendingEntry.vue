<script setup lang="ts">
import { computed, ref } from 'vue'

import type { PendingSummary } from '../api'
import { useProject } from '../stores/project'
import { pendingCounts } from '../utils/pending'
import AppIcon from './AppIcon.vue'
import CommitDialog from './CommitDialog.vue'
import PickHunksDialog from './PickHunksDialog.vue'

const props = defineProps<{ summary: PendingSummary }>()
const emit = defineEmits<{ open: [] }>()

const { capabilities } = useProject()

const COMMIT_TITLE = 'Commit\nChoose the files to commit'
const PICK_HUNKS_TITLE = 'Commit (Pick Hunks)\nGo through the changes hunk by hunk, as with git add --patch'

const counts = computed(() => pendingCounts(props.summary))
const dialog = ref<'files' | 'hunks' | null>(null)
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
    <div class="actions">
      <button
        class="button button--icon"
        type="button"
        :disabled="!capabilities.canCommit"
        :title="COMMIT_TITLE"
        @click.stop="dialog = 'files'"
        @keydown.enter.stop
      >
        <AppIcon name="commit" />
      </button>
      <button
        class="button button--icon"
        type="button"
        :disabled="!capabilities.canCommit"
        :title="PICK_HUNKS_TITLE"
        @click.stop="dialog = 'hunks'"
        @keydown.enter.stop
      >
        <AppIcon name="pickHunks" />
      </button>
    </div>

    <CommitDialog v-if="dialog === 'files'" @close="dialog = null" />
    <PickHunksDialog v-else-if="dialog === 'hunks'" @close="dialog = null" />
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
  transition: background-color var(--transition);
}

.pending-entry:hover {
  background: var(--color-pending-hover);
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
  flex: 1;
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

/* Lined up with the actions of the commits below. */
.actions {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
}
</style>
