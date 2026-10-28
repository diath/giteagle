<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage } from '../api'
import { capabilities, refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { formatRelative } from '../utils/dates'
import AppIcon from './AppIcon.vue'

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
    <div class="current" :title="repo.overview?.workdir">
      <AppIcon name="branch" />
      <span class="branch-name ellipsis" :class="{ detached: head?.kind === 'detached' }">{{ branchLabel }}</span>
      <template v-if="head?.tip">
        <span class="hash">{{ head.tip.shortId }}</span>
        <span class="faint">·</span>
        <span class="muted date">{{ formatRelative(head.tip.authorTime) }}</span>
      </template>
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

    <span v-if="capabilities.notice" class="notice">
      <AppIcon name="warning" />
      {{ capabilities.notice }} — finish it locally to switch, pull, or push.
    </span>
  </header>
</template>

<style scoped>
.branch-bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-height: 48px;
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.current {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  max-width: 45%;
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

.branch-name.detached {
  color: var(--color-warning);
}

.date {
  flex: none;
}

.divider {
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
  min-width: 0;
  margin-left: auto;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-warning);
  border-radius: var(--radius);
  background: var(--color-warning-muted);
  color: var(--color-warning);
  font-size: var(--font-size-small);
}
</style>
