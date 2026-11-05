<script setup lang="ts">
import { ref } from 'vue'

import type { CommitSummary } from '../api'
import { useProject } from '../stores/project'
import { joinNames } from '../utils/authors'
import { formatAbsolute, formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'
import AvatarStack from './AvatarStack.vue'
import CherryPickDialog from './CherryPickDialog.vue'
import DropDialog from './DropDialog.vue'
import RevertDialog from './RevertDialog.vue'
import RewordDialog from './RewordDialog.vue'
import SquashDialog from './SquashDialog.vue'

defineProps<{ commit: CommitSummary }>()
const emit = defineEmits<{ open: [] }>()

const { capabilities } = useProject()

const expanded = ref(false)
const dialog = ref<'reword' | 'squash' | 'drop' | 'cherryPick' | 'revert' | null>(null)
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
          <AppIcon name="more" />
        </button>
      </div>
      <pre v-if="expanded" class="body" @click.stop>{{ commit.body }}</pre>
    </div>

    <!-- Cherry-pick and Revert come last, so they line up across pushed and
         unpushed commits. -->
    <div v-if="commit.isPushed !== null" class="actions">
      <template v-if="commit.isPushed === false">
        <button
          class="button button--icon"
          type="button"
          :disabled="!capabilities.canRewrite"
          title="Reword this commit"
          @click.stop="dialog = 'reword'"
          @keydown.enter.stop
        >
          <AppIcon name="pencil" />
        </button>
        <button
          class="button button--icon"
          type="button"
          :disabled="!capabilities.canRewrite"
          title="Squash other commits into this one"
          @click.stop="dialog = 'squash'"
          @keydown.enter.stop
        >
          <AppIcon name="squash" />
        </button>
        <button
          class="button button--icon"
          type="button"
          :disabled="!capabilities.canRewrite"
          title="Drop this commit"
          @click.stop="dialog = 'drop'"
          @keydown.enter.stop
        >
          <AppIcon name="trash" />
        </button>
        <span class="divider" aria-hidden="true"></span>
      </template>
      <button
        class="button button--icon"
        type="button"
        :disabled="!capabilities.canCherryPick"
        title="Cherry-pick to other branches"
        @click.stop="dialog = 'cherryPick'"
        @keydown.enter.stop
      >
        <AppIcon name="cherryPick" />
      </button>
      <button
        class="button button--icon"
        type="button"
        :disabled="!capabilities.canRevert"
        title="Revert this commit"
        @click.stop="dialog = 'revert'"
        @keydown.enter.stop
      >
        <AppIcon name="revert" />
      </button>
    </div>

    <RewordDialog v-if="dialog === 'reword'" :commit="commit" @close="dialog = null" />
    <SquashDialog v-else-if="dialog === 'squash'" :commit="commit" @close="dialog = null" />
    <DropDialog v-else-if="dialog === 'drop'" :commit="commit" @close="dialog = null" />
    <CherryPickDialog v-else-if="dialog === 'cherryPick'" :commit="commit" @close="dialog = null" />
    <RevertDialog v-else-if="dialog === 'revert'" :commit="commit" @close="dialog = null" />
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
  min-height: calc(28 * var(--px));
}

.subject {
  min-width: 0;
  color: var(--color-fg-strong);
}

.actions {
  display: flex;
  flex: none;
  align-self: center;
  align-items: center;
  gap: var(--space-2);
}

/* Sets the commit's own actions apart from Cherry-pick and Revert. */
.divider {
  flex: none;
  width: 1px;
  height: calc(25 * var(--px));
  margin: 0 var(--space-1);
  background: var(--color-border);
}

.expand {
  width: calc(25 * var(--px));
  height: calc(25 * var(--px));
  flex: none;
}

/* The icon stays the same, so the open state shows as a pressed button. */
.expand[aria-expanded='true'] {
  background: var(--color-bg-active);
  color: var(--color-fg);
}

.body {
  margin: var(--space-2) 0 var(--space-1);
  padding: var(--space-2) var(--space-3);
  border-left: calc(3 * var(--px)) solid var(--color-border-strong);
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
