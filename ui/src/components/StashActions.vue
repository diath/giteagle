<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { errorMessage, type StashSummary } from '../api'
import type { IconName } from '../icons/paths'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import { pendingCounts } from '../utils/pending'
import { stashName } from '../utils/stashes'
import AppIcon from './AppIcon.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import TypeToConfirm from './TypeToConfirm.vue'

type Action = 'apply' | 'pop' | 'drop'

const { api, capabilities, repo, refreshWorkingTree } = useProject()

const ACTIONS: Record<
  Action,
  {
    verb: string
    icon: IconName
    working: string
    run: (id: string) => Promise<string>
    done: (name: string) => string
  }
> = {
  apply: {
    verb: 'Apply',
    icon: 'stashApply',
    working: 'Applying…',
    run: api.applyStash,
    done: (name) => `Applied ${name}.`,
  },
  pop: {
    verb: 'Pop',
    icon: 'stashPop',
    working: 'Popping…',
    run: api.popStash,
    done: (name) => `Applied and deleted ${name}.`,
  },
  drop: {
    verb: 'Delete',
    icon: 'trash',
    working: 'Deleting…',
    run: api.dropStash,
    done: (name) => `Deleted ${name}.`,
  },
}

const props = defineProps<{ stash: StashSummary }>()
const emit = defineEmits<{ done: [action: Action] }>()

const confirming = ref<Action | null>(null)
const confirmation = ref('')
const working = ref(false)

const name = computed(() => stashName(props.stash.index))

// Deleting can't be undone, so it waits until the stash's name is typed.
const expected = computed(() => `delete ${name.value}`)
const confirmed = computed(() => confirming.value !== 'drop' || confirmation.value === expected.value)

watch(confirming, () => {
  confirmation.value = ''
})

const contents = computed(() => {
  const changes = props.stash.changes
  const files = pendingCounts(changes)
    .map(({ state, count }) => `${count} ${state}`)
    .join(', ')
  return `${files} (+${changes.additions}, −${changes.deletions})`
})

async function confirm(): Promise<void> {
  const action = confirming.value
  if (action === null || !confirmed.value || working.value) {
    return
  }
  working.value = true
  repo.busy = 'stash'
  try {
    // The name it had when this ran, which may differ from the one shown if
    // stashes were added or dropped outside GitEagle.
    const ranOn = await ACTIONS[action].run(props.stash.id)
    showToast('success', ACTIONS[action].done(ranOn))
    confirming.value = null
    emit('done', action)
  } catch (error) {
    showToast('error', errorMessage(error))
  } finally {
    working.value = false
    repo.busy = null
    repo.stashRevision += 1
    await refreshWorkingTree()
  }
}
</script>

<template>
  <div class="stash-actions">
    <button
      class="button"
      type="button"
      :disabled="!capabilities.canStash"
      title="Restore these changes and keep the stash"
      @click.stop="confirming = 'apply'"
      @keydown.enter.stop
    >
      <AppIcon :name="ACTIONS.apply.icon" />
      Apply
    </button>
    <button
      class="button"
      type="button"
      :disabled="!capabilities.canStash"
      title="Restore these changes and delete the stash"
      @click.stop="confirming = 'pop'"
      @keydown.enter.stop
    >
      <AppIcon :name="ACTIONS.pop.icon" />
      Pop
    </button>
    <button
      class="button drop"
      type="button"
      :disabled="!capabilities.canDropStash"
      title="Delete the stash without restoring it"
      @click.stop="confirming = 'drop'"
      @keydown.enter.stop
    >
      <AppIcon :name="ACTIONS.drop.icon" />
      Delete
    </button>

    <ConfirmDialog
      v-if="confirming"
      :title="`${ACTIONS[confirming].verb} ${name}`"
      :icon="ACTIONS[confirming].icon"
      :confirm-label="ACTIONS[confirming].verb"
      :working-label="ACTIONS[confirming].working"
      :working="working"
      :disabled="!confirmed"
      :danger="confirming === 'drop'"
      :warning="confirming === 'drop' ? 'The stashed changes will be lost for good.' : null"
      @confirm="confirm"
      @close="confirming = null"
    >
      <p v-if="confirming === 'apply'">Restores the stashed changes into the working tree. The stash is kept.</p>
      <p v-else-if="confirming === 'pop'">
        Restores the stashed changes into the working tree, then deletes the stash.
      </p>
      <p v-else>Deletes the stash without applying it.</p>

      <p class="muted">
        <span class="hash">{{ name }}</span> holds {{ contents }}.
        <template v-if="confirming !== 'drop' && stash.changes.staged > 0">
          Staged changes come back unstaged, except for new files.
        </template>
      </p>

      <template v-if="confirming === 'drop'" #confirmation>
        <TypeToConfirm v-model="confirmation" :expected="expected" :disabled="working" />
      </template>
    </ConfirmDialog>
  </div>
</template>

<style scoped>
.stash-actions {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
}

.drop:hover:not(:disabled) {
  color: var(--color-danger-bright);
}
</style>
