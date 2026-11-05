<script setup lang="ts">
import { computed, ref } from 'vue'

import { useProject } from '../stores/project'
import { useEscape } from '../utils/escape'
import CherryPickDialog from './CherryPickDialog.vue'
import FloatingPrompt from './FloatingPrompt.vue'

const { capabilities, endPickSelection, pickSelection } = useProject()

const dialogOpen = ref(false)

const count = computed(() => pickSelection.commits.length)

// The dialog's own Escape layer comes after this one, so Escape closes the
// dialog first and only then leaves the selection.
useEscape(endPickSelection)
</script>

<template>
  <FloatingPrompt icon="cherryPick">
    <template v-if="count === 0">Check the commits to cherry-pick.</template>
    <template v-else>{{ count }} {{ count === 1 ? 'commit' : 'commits' }} selected to cherry-pick.</template>
    <template #actions>
      <button
        class="button"
        type="button"
        :disabled="count === 0 || !capabilities.canCherryPick"
        @click="dialogOpen = true"
      >
        Cherry-pick
      </button>
      <button class="button" type="button" title="Leave the selection (Esc)" @click="endPickSelection">Cancel</button>
    </template>
  </FloatingPrompt>
  <CherryPickDialog
    v-if="dialogOpen"
    :commits="pickSelection.commits"
    @picked="endPickSelection"
    @close="dialogOpen = false"
  />
</template>
