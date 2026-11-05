<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage, type GitHubPage } from '../api'
import type { IconName } from '../icons/paths'
import { repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import AppIcon from './AppIcon.vue'

const GITHUB_PAGES: { id: GitHubPage; label: string; icon: IconName; description: string }[] = [
  { id: 'issues', label: 'Issues', icon: 'issue', description: 'issues' },
  { id: 'pullRequests', label: 'Pull Requests', icon: 'pullRequest', description: 'pull requests' },
]

const github = computed(() => repo.overview?.github ?? null)

async function run(action: () => Promise<void>): Promise<void> {
  try {
    await action()
  } catch (error) {
    showToast('error', errorMessage(error))
  }
}

function openFolder(): void {
  void run(api.openWorkdir)
}

function openPage(page: GitHubPage): void {
  void run(() => api.openGitHubPage(page))
}
</script>

<template>
  <div class="project-links">
    <button
      class="button"
      type="button"
      :title="`Open ${repo.overview?.workdir ?? 'the project folder'} in the file manager`"
      @click="openFolder"
    >
      <AppIcon name="folder" />
      Folder
    </button>
    <template v-if="github">
      <button
        v-for="page in GITHUB_PAGES"
        :key="page.id"
        class="button"
        type="button"
        :title="`Open the ${page.description} of ${github.owner}/${github.name} on GitHub`"
        @click="openPage(page.id)"
      >
        <AppIcon :name="page.icon" />
        {{ page.label }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.project-links {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
}
</style>
