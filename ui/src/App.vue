<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

import { api, errorMessage, events } from './api'
import ReloadPrompt from './components/ReloadPrompt.vue'
import ToastHost from './components/ToastHost.vue'
import { goBack, navigation } from './stores/navigation'
import { applyFetchStatus, refreshOverview, repo } from './stores/repo'
import { showToast } from './stores/toasts'
import { applyWorkingTreeStatus, refreshWorkingTree } from './stores/workingTree'
import CommitView from './views/CommitView.vue'
import FileHistoryView from './views/FileHistoryView.vue'
import MainView from './views/MainView.vue'
import PendingChangesView from './views/PendingChangesView.vue'

/** The "back" button found on many mice. */
const MOUSE_BACK_BUTTON = 3

const unlisteners: (() => void)[] = []

function onMouseUp(event: MouseEvent): void {
  if (event.button === MOUSE_BACK_BUTTON) {
    goBack()
  }
}

function viewKey(index: number): string {
  const view = navigation.stack[index]
  switch (view.kind) {
    case 'main':
      return 'main'
    case 'fileHistory':
      return `${index}:history:${view.path}`
    case 'commit':
      return `${index}:commit:${view.id}`
    case 'pending':
      return `${index}:pending`
  }
  return String(index)
}

onMounted(async () => {
  window.addEventListener('mouseup', onMouseUp)
  // The repository loads first, so a failure in the live updates below
  // can't leave the window empty.
  await refreshOverview()
  void refreshWorkingTree()
  try {
    unlisteners.push(
      await events.onRepoChanged(() => {
        repo.reloadPromptVisible = true
      }),
      await events.onWorkingTreeChanged(applyWorkingTreeStatus),
      await events.onFetchStatus(applyFetchStatus),
    )
    applyFetchStatus(await api.fetchStatus())
  } catch (error) {
    showToast('error', `Live updates are unavailable: ${errorMessage(error)}`)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('mouseup', onMouseUp)
  unlisteners.forEach((unlisten) => unlisten())
})
</script>

<template>
  <div v-if="repo.error && !repo.overview" class="fatal">
    <p>GitEagle couldn't read the repository.</p>
    <pre>{{ repo.error }}</pre>
  </div>
  <div v-else-if="repo.overview" class="views">
    <!-- Views below the top stay mounted but hidden so going back restores
         them exactly as they were. -->
    <div
      v-for="(view, index) in navigation.stack"
      :key="viewKey(index)"
      class="view"
      :class="{ covered: index !== navigation.stack.length - 1 }"
    >
      <MainView v-if="view.kind === 'main'" :active="index === navigation.stack.length - 1" />
      <FileHistoryView
        v-else-if="view.kind === 'fileHistory'"
        :path="view.path"
        :active="index === navigation.stack.length - 1"
      />
      <PendingChangesView v-else-if="view.kind === 'pending'" />
      <CommitView v-else :id="view.id" :from-file="view.fromFile" />
    </div>
  </div>
  <p v-else class="loading">Loading the repository…</p>
  <ReloadPrompt v-if="repo.reloadPromptVisible" />
  <ToastHost />
</template>

<style scoped>
.views {
  position: relative;
  height: 100%;
}

.view {
  position: absolute;
  inset: 0;
  background: var(--color-bg-app);
}

.view.covered {
  visibility: hidden;
  pointer-events: none;
}

.loading {
  margin: 0;
  padding: var(--space-8);
  color: var(--color-fg-faint);
  text-align: center;
}

.fatal {
  padding: var(--space-8);
  color: var(--color-danger-bright);
}

.fatal pre {
  color: var(--color-fg-muted);
  font: inherit;
  white-space: pre-wrap;
}
</style>
