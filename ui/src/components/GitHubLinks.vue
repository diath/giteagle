<script setup lang="ts">
import { computed } from 'vue'

import { api, errorMessage, type GitHubPage } from '../api'
import type { IconName } from '../icons/paths'
import { repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import AppIcon from './AppIcon.vue'

const PAGES: { id: GitHubPage; label: string; icon: IconName; description: string }[] = [
  { id: 'issues', label: 'Issues', icon: 'issue', description: 'issues' },
  { id: 'pullRequests', label: 'Pull Requests', icon: 'pullRequest', description: 'pull requests' },
]

const github = computed(() => repo.overview?.github ?? null)

async function openPage(page: GitHubPage): Promise<void> {
  try {
    await api.openGitHubPage(page)
  } catch (error) {
    showToast('error', errorMessage(error))
  }
}
</script>

<template>
  <div v-if="github" class="github-links">
    <button
      v-for="page in PAGES"
      :key="page.id"
      class="button button--quiet link"
      type="button"
      :title="`Open the ${page.description} of ${github.owner}/${github.name} on GitHub`"
      @click="openPage(page.id)"
    >
      <AppIcon :name="page.icon" />
      {{ page.label }}
    </button>
  </div>
</template>

<style scoped>
.github-links {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-1);
}

.link {
  color: var(--color-fg-muted);
}

.link:hover {
  color: var(--color-fg);
}
</style>
