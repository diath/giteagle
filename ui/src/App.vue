<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from 'vue'

import { appApi, errorMessage, events } from './api'
import ProjectTabs from './components/ProjectTabs.vue'
import ToastHost from './components/ToastHost.vue'
import { activeTab, closeActiveTab, openNewTab, projectById, restoreTabs, tabs } from './stores/tabs'
import { showToast } from './stores/toasts'
import OpenProjectPage from './views/OpenProjectPage.vue'
import ProjectPage from './views/ProjectPage.vue'

/** The "back" button found on many mice. */
const MOUSE_BACK_BUTTON = 3

const unlisteners: (() => void)[] = []

function onMouseUp(event: MouseEvent): void {
  if (event.button === MOUSE_BACK_BUTTON) {
    activeTab.value?.project?.goBack()
  }
}

// Ctrl+T and Ctrl+W open and close tabs as in a browser, even while typing in
// a field.
function onKeyDown(event: KeyboardEvent): void {
  if (!event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {
    return
  }
  const key = event.key.toLowerCase()
  if (key !== 't' && key !== 'w') {
    return
  }
  event.preventDefault()
  // A held key repeats, which would open or close one tab after another.
  if (event.repeat) {
    return
  }
  if (key === 't') {
    openNewTab()
  } else {
    closeActiveTab()
  }
}

// Only the project on screen is polled for outside changes, and the window
// title names it.
watch(
  () => activeTab.value?.project?.summary.id ?? null,
  (id) => {
    appApi.showProject(id).catch((error: unknown) => {
      showToast('error', `Couldn't switch the project: ${errorMessage(error)}`)
    })
  },
)

onMounted(async () => {
  window.addEventListener('mouseup', onMouseUp)
  window.addEventListener('keydown', onKeyDown)
  // Listening starts before any project loads, so none misses an update;
  // a failure here still leaves the projects usable.
  try {
    unlisteners.push(
      await events.onRepoChanged((id) => {
        const project = projectById(id)
        if (project) {
          project.repo.reloadPromptVisible = true
        }
      }),
      await events.onWorkingTreeChanged((id, entries) => projectById(id)?.applyWorkingTreeStatus(entries)),
      await events.onPendingChanged((id, summary) => projectById(id)?.applyPendingSummary(summary)),
      await events.onStashesChanged((id) => {
        const project = projectById(id)
        if (project) {
          project.repo.stashRevision += 1
        }
      }),
      await events.onFetchStatus((id, status) => projectById(id)?.applyFetchStatus(status)),
    )
  } catch (error) {
    showToast('error', `Live updates are unavailable: ${errorMessage(error)}`)
  }
  await restoreTabs()
})

onBeforeUnmount(() => {
  window.removeEventListener('mouseup', onMouseUp)
  window.removeEventListener('keydown', onKeyDown)
  unlisteners.forEach((unlisten) => unlisten())
})
</script>

<template>
  <div class="app">
    <ProjectTabs />
    <main class="pages">
      <!-- Every tab stays mounted and only hides, so each keeps its views,
           scroll positions and loaded data while another is shown. -->
      <div
        v-for="tab in tabs.list"
        :key="tab.key"
        class="page"
        :class="{ hidden: tab.key !== tabs.activeKey }"
      >
        <ProjectPage v-if="tab.project" :project="tab.project" :active="tab.key === tabs.activeKey" />
        <OpenProjectPage v-else :tab-key="tab.key" :active="tab.key === tabs.activeKey" />
      </div>
    </main>
  </div>
  <ToastHost />
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.pages {
  position: relative;
  flex: 1;
  min-height: 0;
}

.page {
  position: absolute;
  inset: 0;
}

.page.hidden {
  visibility: hidden;
  pointer-events: none;
}
</style>
