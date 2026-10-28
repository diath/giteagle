<script setup lang="ts">
import { ref } from 'vue'

import type { CommitSummary } from '../api'
import { joinNames } from '../utils/authors'
import { formatAbsolute, formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'
import AvatarStack from './AvatarStack.vue'

defineProps<{ commit: CommitSummary }>()
const emit = defineEmits<{ open: [] }>()

const expanded = ref(false)
</script>

<template>
  <div
    class="commit-entry"
    role="button"
    tabindex="0"
    @click="emit('open')"
    @keydown.enter.self="emit('open')"
  >
    <AvatarStack class="avatars" :authors="commit.authors" />
    <div class="content">
      <div class="meta">
        <span class="hash">{{ commit.shortId }}</span>
        <span class="faint">by</span>
        <span class="authors ellipsis">{{ joinNames(commit.authors) }}</span>
        <span class="faint">@</span>
        <span class="date muted">{{ formatAbsolute(commit.authorTime) }}</span>
        <span class="faint">({{ formatRelative(commit.authorTime) }})</span>
      </div>
      <div class="subject-line">
        <span class="subject ellipsis">{{ commit.subject }}</span>
        <button
          v-if="commit.body"
          class="icon-button expand"
          type="button"
          :title="expanded ? 'Hide the full message' : 'Show the full message'"
          :aria-expanded="expanded"
          @click.stop="expanded = !expanded"
          @keydown.enter.stop
        >
          <AppIcon :name="expanded ? 'chevronDown' : 'chevronRight'" />
        </button>
      </div>
      <pre v-if="expanded" class="body" @click.stop>{{ commit.body }}</pre>
    </div>
  </div>
</template>

<style scoped>
.commit-entry {
  display: flex;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
  line-height: var(--line-height-dense);
  cursor: pointer;
  transition: background-color var(--transition);
}

.commit-entry:hover {
  background: var(--color-bg-hover);
}

.commit-entry:focus-visible {
  outline-offset: -2px;
}

.avatars {
  margin-top: 2px;
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

.authors {
  min-width: 0;
  color: var(--color-fg-muted);
}

.date {
  flex: none;
}

.meta .faint:last-child {
  flex: none;
}

.subject-line {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  min-height: 22px;
}

.subject {
  min-width: 0;
  color: var(--color-fg-strong);
}

.expand {
  width: 20px;
  height: 20px;
  flex: none;
}

.body {
  margin: var(--space-2) 0 var(--space-1);
  padding: var(--space-2) var(--space-3);
  border-left: 2px solid var(--color-border-strong);
  color: var(--color-fg-muted);
  font: inherit;
  font-size: var(--font-size-small);
  line-height: var(--line-height);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  cursor: text;
  user-select: text;
}
</style>
