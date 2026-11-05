<script setup lang="ts">
import { useProject } from '../stores/project'
import FloatingPrompt from './FloatingPrompt.vue'

const { reloadRepository, repo } = useProject()
</script>

<template>
  <FloatingPrompt icon="refresh">
    The repository has changed outside GitEagle. Reload?
    <template #actions>
      <button class="button" type="button" :disabled="repo.busy !== null" @click="reloadRepository">Reload</button>
      <button class="button" type="button" @click="repo.reloadPromptVisible = false">Dismiss</button>
    </template>
  </FloatingPrompt>
</template>
