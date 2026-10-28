<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage } from '../api'
import { capabilities, refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'
import ProjectInfo from './ProjectInfo.vue'

defineEmits<{ openSwitcher: [] }>()

const head = computed(() => repo.overview?.head ?? null)

const branchLabel = computed(() => {
  const current = head.value
  if (!current) {
    return ''
  }
  switch (current.kind) {
    case 'branch':
      return current.name ?? ''
    case 'detached':
      return 'detached HEAD'
    case 'unborn':
      return current.name ?? 'no branch'
  }
  return ''
})

const pushTitle = computed(() => {
  const current = head.value
  if (!current || current.kind !== 'branch') {
    return 'Push needs a checked-out branch'
  }
  if (current.upstream) {
    return `Push ${current.name} to ${current.upstream}`
  }
  return repo.overview?.hasOrigin
    ? `Publish ${current.name} to origin and track it`
    : 'There is no origin remote to publish to'
})

const pullTitle = computed(() => {
  const current = head.value
  if (!current || current.kind !== 'branch') {
    return 'Pull needs a checked-out branch'
  }
  return current.upstream
    ? `Fast-forward ${current.name} from ${current.upstream}`
    : 'The branch has no upstream to pull from'
})

async function sync(operation: 'pull' | 'push'): Promise<void> {
  repo.busy = operation
  try {
    await (operation === 'pull' ? api.pull() : api.push())
    showToast('success', operation === 'pull' ? 'Pulled from the upstream.' : 'Pushed to the remote.')
  } catch (error) {
    showToast('error', errorMessage(error))
  } finally {
    repo.busy = null
    await refreshEverything()
  }
}
</script>

<template>
  <header class="branch-bar">
    <div class="column column--left">
      <ProjectInfo />
    </div>

    <div class="column column--center">
      <div class="controls">
        <div class="current" :title="branchLabel">
          <AppIcon name="branch" />
          <span class="branch-name ellipsis" :class="{ detached: head?.kind === 'detached' }">{{ branchLabel }}</span>
          <span v-if="head?.tip" class="tip ellipsis">
            <span class="hash">{{ head.tip.shortId }}</span>
            <span class="faint"> · </span>
            <span class="muted">{{ formatRelative(head.tip.authorTime) }}</span>
          </span>
          <span v-else-if="head?.kind === 'unborn'" class="faint">no commits yet</span>
        </div>

        <button
          class="button"
          type="button"
          :disabled="!capabilities.canSwitch"
          title="Switch to another branch"
          @click="$emit('openSwitcher')"
        >
          <AppIcon name="switch" />
          Switch
          <span class="count">{{ repo.overview?.branchCount ?? 0 }}</span>
        </button>

        <span class="divider" aria-hidden="true"></span>

        <button class="button" type="button" :disabled="!capabilities.canPull" :title="pullTitle" @click="sync('pull')">
          <AppIcon name="pull" />
          {{ repo.busy === 'pull' ? 'Pulling…' : 'Pull' }}
          <span v-if="head?.behind" class="count count--behind">↓{{ head.behind }}</span>
        </button>
        <button class="button" type="button" :disabled="!capabilities.canPush" :title="pushTitle" @click="sync('push')">
          <AppIcon name="push" />
          {{ repo.busy === 'push' ? 'Pushing…' : head?.kind === 'branch' && !head.upstream ? 'Publish' : 'Push' }}
          <span v-if="head?.ahead" class="count count--ahead">↑{{ head.ahead }}</span>
        </button>
      </div>

      <span v-if="capabilities.notice" class="notice">
        <AppIcon name="warning" />
        {{ capabilities.notice }} - finish it locally to switch, pull, or push.
      </span>
    </div>

    <div class="column column--right"></div>
  </header>
</template>

<style scoped>
/* The middle column takes what its contents need, but the outer columns
   always keep a share, so a long branch name can't push them out. They split
   the rest equally, which keeps the middle one centered in the window. */
.branch-bar {
  display: grid;
  grid-template-columns: minmax(15%, 1fr) minmax(0, max-content) minmax(15%, 1fr);
  align-items: center;
  gap: var(--space-4);
  min-height: 48px;
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.column {
  display: flex;
  align-items: center;
  min-width: 0;
}

.column--center {
  flex-direction: column;
  gap: var(--space-2);
}

.column--right {
  justify-content: flex-end;
}

/* The controls never wrap; the branch name gives up its space first. */
.controls {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  max-width: 100%;
}

.controls .button {
  flex: none;
}

.current {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  height: var(--control-height);
  padding: 0 var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  color: var(--color-accent-bright);
}

.branch-name {
  min-width: 0;
  color: var(--color-fg-strong);
  font-weight: 600;
}

/* The commit gives up nearly all of the space before the branch name loses
   any. */
.tip {
  flex: 0 1000 auto;
  min-width: 0;
}

.branch-name.detached {
  color: var(--color-warning);
}

.divider {
  flex: none;
  width: 1px;
  height: 20px;
  margin: 0 var(--space-1);
  background: var(--color-border);
}

.count {
  padding: 0 var(--space-1);
  border-radius: var(--radius);
  background: var(--color-bg-active);
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.count--ahead {
  color: var(--color-accent-bright);
}

.count--behind {
  color: var(--color-link);
}

.notice {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  max-width: 100%;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-warning);
  border-radius: var(--radius);
  background: var(--color-warning-muted);
  color: var(--color-warning);
  font-size: var(--font-size-small);
}
</style>
