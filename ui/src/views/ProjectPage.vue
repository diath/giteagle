<script setup lang="ts">
import { onMounted } from 'vue'

import CommitSelectionPrompt from '../components/CommitSelectionPrompt.vue'
import ReloadPrompt from '../components/ReloadPrompt.vue'
import { provideProject, type Project } from '../stores/project'
import { provideEscapeScope } from '../utils/escape'
import BlameView from './BlameView.vue'
import CommitView from './CommitView.vue'
import FileHistoryView from './FileHistoryView.vue'
import FileView from './FileView.vue'
import MainView from './MainView.vue'
import PendingChangesView from './PendingChangesView.vue'
import StashView from './StashView.vue'

const props = defineProps<{
  /** Fixed for the page's life: a tab that opens a project gets a new page. */
  project: Project
  /** Whether the page's tab is the one shown. */
  active: boolean
}>()

// Read once, as a page keeps its project for life.
const project = props.project
const { commitSelection, navigation, repo } = project
provideProject(project)
provideEscapeScope(() => props.active)

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
    case 'stash':
      return `${index}:stash:${view.id}`
    case 'file':
      return `${index}:file:${view.commit}:${view.path}`
    case 'blame':
      return `${index}:blame:${view.commit}:${view.path}`
  }
  return String(index)
}

/** Only the top view of the shown tab takes keyboard input and loads more as it scrolls. */
function isActive(index: number): boolean {
  return props.active && index === navigation.stack.length - 1
}

onMounted(() => void project.load())
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
      <MainView v-if="view.kind === 'main'" :active="isActive(index)" />
      <FileHistoryView v-else-if="view.kind === 'fileHistory'" :path="view.path" :active="isActive(index)" />
      <PendingChangesView v-else-if="view.kind === 'pending'" />
      <StashView v-else-if="view.kind === 'stash'" :id="view.id" />
      <FileView
        v-else-if="view.kind === 'file'"
        :commit="view.commit"
        :short-id="view.shortId"
        :path="view.path"
      />
      <BlameView
        v-else-if="view.kind === 'blame'"
        :commit="view.commit"
        :short-id="view.shortId"
        :path="view.path"
        :line="view.line"
      />
      <CommitView v-else :id="view.id" :from-file="view.fromFile" :sequence="view.sequence" />
    </div>
  </div>
  <p v-else class="loading">Loading the repository…</p>
  <!-- Both sit at the bottom, so the reload prompt waits for the selection. -->
  <CommitSelectionPrompt v-if="commitSelection.active" />
  <ReloadPrompt v-else-if="repo.reloadPromptVisible" />
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
