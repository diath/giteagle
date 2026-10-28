<script setup lang="ts">
import { computed } from 'vue'

import { repo } from '../stores/repo'
import AppIcon from './AppIcon.vue'

/** The working directory split before its last component. */
const pathParts = computed(() => {
  const path = (repo.overview?.workdir ?? '').replace(/[\\/]+$/, '')
  const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return cut < 0 ? { start: '', end: path } : { start: path.slice(0, cut), end: path.slice(cut) }
})
</script>

<template>
  <div class="project" :title="repo.overview?.workdir">
    <AppIcon name="folder" />
    <span class="name ellipsis">{{ repo.overview?.repoName }}</span>
    <span class="path">
      <span class="path-start ellipsis">{{ pathParts.start }}</span>
      <span class="path-end ellipsis">{{ pathParts.end }}</span>
    </span>
  </div>
</template>

<style scoped>
.project {
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

.name {
  flex: none;
  max-width: 50%;
  color: var(--color-fg-strong);
  font-weight: 600;
}

.path {
  display: flex;
  min-width: 0;
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

/* The start gives up nearly all of the space before the folder name loses
   any, so a long path is cut in the middle and both of its ends stay. */
.path-start {
  flex: 0 1000 auto;
  min-width: 0;
}

.path-end {
  flex: 0 1 auto;
  min-width: 0;
}
</style>
