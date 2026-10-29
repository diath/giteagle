<script setup lang="ts">
import { computed, ref } from 'vue'

import { api, errorMessage, type CommitSummary, type RevertMode } from '../api'
import { refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { refreshWorkingTree } from '../stores/workingTree'
import ConfirmDialog from './ConfirmDialog.vue'

interface RevertOption {
  mode: RevertMode
  label: string
  description: string
}

const props = defineProps<{ commit: CommitSummary }>()
const emit = defineEmits<{ close: [] }>()

const mode = ref<RevertMode>('newCommit')
const working = ref(false)

const branch = computed(() => repo.overview?.head.name ?? 'the branch')

const options = computed<RevertOption[]>(() => [
  {
    mode: 'newCommit',
    label: 'Create a revert commit',
    description: `Adds a commit to ${branch.value} that undoes the changes. Both commits stay in the history.`,
  },
  {
    mode: 'workingTree',
    label: 'Revert without committing',
    description: 'Undoes the changes in the working tree and stages them, for you to review and commit.',
  },
])

function doneMessage(chosen: RevertMode): string {
  switch (chosen) {
    case 'newCommit':
      return `Reverted ${props.commit.shortId} in a new commit.`
    case 'workingTree':
      return `Undid the changes from ${props.commit.shortId} in the working tree and staged them.`
  }
}

async function confirm(): Promise<void> {
  if (working.value) {
    return
  }
  const chosen = mode.value
  const done = doneMessage(chosen)
  working.value = true
  repo.busy = 'revert'
  try {
    await api.revertCommit(props.commit.id, chosen)
  } catch (error) {
    // Nothing changed, so the dialog stays open for another choice.
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', done)
  emit('close')
  // Reverting in the working tree leaves the commits as they are.
  await (chosen === 'workingTree' ? refreshWorkingTree() : refreshEverything())
}
</script>

<template>
  <ConfirmDialog
    :title="`Revert ${commit.shortId}`"
    icon="revert"
    confirm-label="Revert"
    working-label="Reverting…"
    :working="working"
    @confirm="confirm"
    @close="emit('close')"
  >
    <p class="commit ellipsis"><span class="hash">{{ commit.shortId }}</span> {{ commit.subject }}</p>

    <div class="options" role="radiogroup" aria-label="How to revert the commit">
      <label
        v-for="option in options"
        :key="option.mode"
        class="option"
        :class="{ selected: mode === option.mode }"
      >
        <input v-model="mode" type="radio" name="revert-mode" :value="option.mode" :disabled="working" />
        <span class="option-text">
          <span class="option-label">{{ option.label }}</span>
          <span class="option-description muted">{{ option.description }}</span>
        </span>
      </label>
    </div>
  </ConfirmDialog>
</template>

<style scoped>
.commit {
  color: var(--color-fg-strong);
}

.options {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.option {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  cursor: pointer;
  transition:
    background-color var(--transition),
    border-color var(--transition);
}

.option:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-hover);
}

.option.selected {
  border-color: var(--color-accent);
}

/* Lines the button up with the first line of the label. */
.option input {
  flex: none;
  margin: calc(4 * var(--px)) 0 0;
  accent-color: var(--color-accent);
}

.option-text {
  display: flex;
  flex-direction: column;
  gap: calc(2 * var(--px));
  min-width: 0;
}

.option-label {
  color: var(--color-fg-strong);
}

.option-description {
  font-size: var(--font-size-small);
}
</style>
