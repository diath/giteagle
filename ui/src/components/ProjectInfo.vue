<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage } from '../api'
import { repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import AppIcon from './AppIcon.vue'

/** The working directory split before its last component. */
const pathParts = computed(() => {
  const path = (repo.overview?.workdir ?? '').replace(/[\\/]+$/, '')
  const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return cut < 0 ? { start: '', end: path } : { start: path.slice(0, cut), end: path.slice(cut) }
})

async function openFolder(): Promise<void> {
  try {
    await api.openWorkdir()
  } catch (error) {
    showToast('error', errorMessage(error))
  }
}
</script>

<template>
  <button
    class="project"
    type="button"
    :title="`${repo.overview?.workdir ?? ''}\n\nClick to open the folder in the file manager.`"
    @click="openFolder"
  >
    <AppIcon name="folder" />
    <span class="text">
      <span class="name ellipsis">{{ repo.overview?.repoName }}</span>
      <span class="path">
        <span class="path-start ellipsis">{{ pathParts.start }}</span>
        <span class="path-end ellipsis">{{ pathParts.end }}</span>
      </span>
    </span>
  </button>
</template>

<style scoped>
.project {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  color: var(--color-accent-bright);
  text-align: left;
  cursor: pointer;
  transition:
    background-color var(--transition),
    border-color var(--transition);
}

.project:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-hover);
}

.project:active {
  background: var(--color-bg-active);
}

/* The path gets a line of its own, so it has the widget's full width. */
.text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.2;
}

.name {
  color: var(--color-fg-strong);
  font-weight: 600;
}

.path {
  display: flex;
  min-width: 0;
  color: var(--color-fg-muted);
  font-size: calc(12 * var(--px));
}

/* Only the start shrinks, and the folder name gives way only once the start
   is gone, so a long path is cut in the middle and both of its ends stay. */
.path-start {
  min-width: 0;
}

.path-end {
  flex: none;
  max-width: 100%;
}
</style>
