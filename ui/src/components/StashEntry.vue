<script setup lang="ts">
import { computed } from 'vue'

import type { StashSummary } from '../api'
import { formatAbsolute, formatRelative } from '../utils/dates'
import { pendingCounts } from '../utils/pending'
import { stashName } from '../utils/stashes'
import AvatarStack from './AvatarStack.vue'
import StashActions from './StashActions.vue'

const props = defineProps<{ stash: StashSummary }>()
const emit = defineEmits<{ open: [] }>()

const counts = computed(() => pendingCounts(props.stash.changes))
</script>

<template>
  <div
    class="stash-entry"
    role="button"
    tabindex="0"
    @click="emit('open')"
    @keydown.enter.self="emit('open')"
  >
    <AvatarStack class="avatars" :authors="[stash.author]" />
    <div class="content">
      <div class="meta">
        <span class="hash">{{ stashName(stash.index) }}</span>
        <span class="faint">on</span>
        <span v-if="stash.branch" class="branch ellipsis">{{ stash.branch }}</span>
        <span v-else class="branch faint">detached HEAD</span>
        <span class="faint">@</span>
        <span class="date muted">{{ formatAbsolute(stash.authorTime) }}</span>
        <span class="faint">({{ formatRelative(stash.authorTime) }})</span>
      </div>
      <div v-if="stash.message" class="message ellipsis">{{ stash.message }}</div>
      <!-- One line, so the spaces between the spans are kept. -->
      <div v-else class="message ellipsis muted">WIP on <span class="hash">{{ stash.base.shortId }}</span> {{ stash.base.subject }}</div>
      <div class="details">
        <template v-for="({ state, count }, index) in counts" :key="state">
          {{ index > 0 ? ', ' : '' }}{{ count }} {{ state }}
        </template>
        <span class="separator faint">·</span>
        <span class="additions">+{{ stash.changes.additions }}</span>,
        <span class="deletions">−{{ stash.changes.deletions }}</span>
      </div>
    </div>
    <StashActions :stash="stash" />
  </div>
</template>

<style scoped>
.stash-entry {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
  line-height: var(--line-height-dense);
  cursor: pointer;
  transition: background-color var(--transition);
}

.stash-entry:hover {
  background: var(--color-bg-hover);
}

.stash-entry:focus-visible {
  outline-offset: -2px;
}

/* Level with the first line, as in the commit list. */
.avatars {
  align-self: flex-start;
  margin-top: calc(3 * var(--px));
}

.content {
  flex: 1;
  min-width: 0;
}

.meta {
  display: flex;
  align-items: baseline;
  gap: 0.6ch;
  font-size: var(--font-size-small);
}

.branch {
  min-width: 0;
  color: var(--color-fg-muted);
}

.date {
  flex: none;
}

.meta .faint:last-child {
  flex: none;
}

.message {
  padding-block: calc(3 * var(--px));
  color: var(--color-fg-strong);
}

.message.muted {
  color: var(--color-fg-muted);
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
