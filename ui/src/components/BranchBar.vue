<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage } from '../api'
import { applyFetchStatus, capabilities, refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { formatRelative } from '../utils/dates'
import { joinWithAnd } from '../utils/lists'
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

const remotes = computed(() => repo.overview?.remotes ?? [])
const hasRemotes = computed(() => remotes.value.length > 0)
const fetchFailed = computed(() => repo.fetch.lastError !== null && !repo.fetch.inProgress)

const fetchTitle = computed(() => {
  if (!hasRemotes.value) {
    return 'There are no remotes to fetch from'
  }
  if (repo.fetch.inProgress) {
    return 'Fetching from all remotes…'
  }
  if (repo.fetch.lastError) {
    return `The last fetch failed:\n${repo.fetch.lastError}\n\nClick to try again.`
  }
  const last =
    repo.fetch.lastSuccess !== null ? `Last fetched ${formatRelative(repo.fetch.lastSuccess)}.` : 'Not fetched yet.'
  return `Fetch from ${joinWithAnd(remotes.value)}. ${last} Fetches also run in the background every 5 minutes.`
})

async function fetchNow(): Promise<void> {
  try {
    await api.fetchNow()
    // The outcome arrives as a status event, which may still be on its way.
    applyFetchStatus(await api.fetchStatus())
  } catch (error) {
    showToast('error', errorMessage(error))
    return
  }
  if (repo.fetch.lastError) {
    showToast('error', `Fetching failed: ${repo.fetch.lastError}`)
  } else {
    showToast('success', 'Fetched from the remotes.')
  }
}

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
  <header class="top-bars">
    <div class="bar">
      <ProjectInfo />
    </div>

    <div class="bar bar--branch">
      <div class="controls">
        <div class="inset current" :title="branchLabel">
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

        <button
          class="button"
          :class="{ 'fetch--failed': fetchFailed }"
          type="button"
          :disabled="!hasRemotes || repo.fetch.inProgress"
          :title="fetchTitle"
          @click="fetchNow"
        >
          <AppIcon :name="fetchFailed ? 'warning' : 'refresh'" />
          {{ repo.fetch.inProgress ? 'Fetching…' : 'Fetch' }}
        </button>
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
  </header>
</template>

<style scoped>
/* The project and the branch each get a bar of their own, with their
   contents centered. */
.bar {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  min-height: var(--bar-height);
  padding: var(--space-1) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.bar > * {
  max-width: 100%;
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
  height: var(--control-height);
}

/* Only the commit shrinks, and the branch name gives way only once the
   commit is gone. */
.branch-name {
  flex: none;
  max-width: calc(100% - var(--icon-size) - 2 * var(--space-2));
  color: var(--color-fg-strong);
  font-weight: 600;
}

.tip {
  min-width: 0;
}

.branch-name.detached {
  color: var(--color-warning);
}

.divider {
  flex: none;
  width: 1px;
  height: calc(25 * var(--px));
  margin: 0 var(--space-1);
  background: var(--color-border);
}

/* A failed background fetch shows here, without interrupting. */
.fetch--failed {
  color: var(--color-warning);
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
