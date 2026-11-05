<script setup lang="ts">
import { computed, ref } from 'vue'

import { useProject } from '../stores/project'
import { useEscape } from '../utils/escape'
import CherryPickDialog from './CherryPickDialog.vue'
import FloatingPrompt from './FloatingPrompt.vue'
import SquashDialog from './SquashDialog.vue'

const { capabilities, commitSelection, endSelection } = useProject()

const dialogOpen = ref(false)

const count = computed(() => commitSelection.commits.length)
const commitsText = computed(() => `${count.value} ${count.value === 1 ? 'commit' : 'commits'}`)
const squashTarget = computed(() => (commitSelection.purpose === 'squash' ? commitSelection.target : null))

const canConfirm = computed(() => {
  if (count.value === 0) {
    return false
  }
  return squashTarget.value ? capabilities.value.canRewrite : capabilities.value.canCherryPick
})

// The dialog's own Escape layer comes after this one, so Escape closes the
// dialog first and only then leaves the selection.
useEscape(endSelection)
</script>

<template>
  <FloatingPrompt :icon="squashTarget ? 'squash' : 'cherryPick'">
    <template v-if="squashTarget">
      <template v-if="count === 0">Check the commits to squash into {{ squashTarget.shortId }}.</template>
      <template v-else>{{ commitsText }} selected to squash into {{ squashTarget.shortId }}.</template>
    </template>
    <template v-else>
      <template v-if="count === 0">Check the commits to cherry-pick.</template>
      <template v-else>{{ commitsText }} selected to cherry-pick.</template>
    </template>
    <template #actions>
      <button class="button" type="button" :disabled="!canConfirm" @click="dialogOpen = true">
        {{ squashTarget ? 'Squash' : 'Cherry-pick' }}
      </button>
      <button class="button" type="button" title="Leave the selection (Esc)" @click="endSelection">Cancel</button>
    </template>
  </FloatingPrompt>
  <template v-if="dialogOpen">
    <SquashDialog
      v-if="squashTarget"
      :commit="squashTarget"
      :preselected="commitSelection.commits.map((commit) => commit.id)"
      @squashed="endSelection"
      @close="dialogOpen = false"
    />
    <CherryPickDialog
      v-else
      :commits="commitSelection.commits"
      @picked="endSelection"
      @close="dialogOpen = false"
    />
  </template>
</template>
