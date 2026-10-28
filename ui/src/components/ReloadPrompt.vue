<script setup lang="ts">
import { reloadRepository, repo } from '../stores/repo'
import AppIcon from './AppIcon.vue'
</script>

<template>
  <div class="reload-prompt" role="status">
    <AppIcon name="refresh" />
    <span>The repository has changed outside GitEagle. Reload?</span>
    <button class="button" type="button" :disabled="repo.busy !== null" @click="reloadRepository">Reload</button>
    <button class="button button--quiet" type="button" @click="repo.reloadPromptVisible = false">Dismiss</button>
  </div>
</template>

<style scoped>
.reload-prompt {
  position: fixed;
  bottom: var(--space-4);
  left: 50%;
  z-index: 30;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius-large);
  background: var(--color-bg-raised);
  color: var(--color-fg-strong);
  box-shadow: 0 4px 14px var(--color-shadow);
  transform: translateX(-50%);
  white-space: nowrap;
}

.reload-prompt > :first-child {
  color: var(--color-accent-bright);
}
</style>
