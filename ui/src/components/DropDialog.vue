<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import { api, errorMessage, type CommitSummary } from '../api'
import { refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import ConfirmDialog from './ConfirmDialog.vue'
import TypeToConfirm from './TypeToConfirm.vue'

const props = defineProps<{ commit: CommitSummary }>()
const emit = defineEmits<{ close: [] }>()

/** How many commits come after this one and are recreated without it, once checked. */
const later = ref<number | null>(null)
const loadError = ref<string | null>(null)
const confirmation = ref('')
const working = ref(false)

const expected = computed(() => `delete ${props.commit.shortId}`)
const confirmed = computed(() => confirmation.value === expected.value)

const branch = computed(() => repo.overview?.head.name ?? 'the branch')

const description = computed(() => {
  const removes = `Removes the commit and its changes from ${branch.value}.`
  if (!later.value) {
    return removes
  }
  const recreated =
    later.value === 1 ? 'The commit after it is' : `The ${later.value} commits after it are`
  return `${removes} ${recreated} recreated without them.`
})

async function confirm(): Promise<void> {
  if (later.value === null || !confirmed.value || working.value) {
    return
  }
  working.value = true
  repo.busy = 'drop'
  try {
    await api.dropCommit(props.commit.id)
  } catch (error) {
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', `Dropped ${props.commit.shortId}.`)
  emit('close')
  await refreshEverything()
}

// Checks up front that the commit can be dropped, so a commit that can't
// shows why instead of offering a Drop button that would fail.
onMounted(async () => {
  try {
    const commits = await api.rewritableCommits(props.commit.id)
    later.value = commits.findIndex((candidate) => candidate.id === props.commit.id)
  } catch (error) {
    loadError.value = errorMessage(error)
  }
})
</script>

<template>
  <ConfirmDialog
    :title="`Drop ${commit.shortId}`"
    icon="trash"
    confirm-label="Drop"
    working-label="Dropping…"
    :working="working"
    :disabled="later === null || !confirmed"
    danger
    :warning="later === null ? null : 'The commit and its changes will be lost for good.'"
    @confirm="confirm"
    @close="emit('close')"
  >
    <p class="commit ellipsis"><span class="hash">{{ commit.shortId }}</span> {{ commit.subject }}</p>
    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="later === null" class="faint">Checking the commit…</p>
    <p v-else class="muted">{{ description }}</p>

    <template v-if="!loadError" #confirmation>
      <TypeToConfirm v-model="confirmation" :expected="expected" :disabled="working" />
    </template>
  </ConfirmDialog>
</template>

<style scoped>
.commit {
  color: var(--color-fg-strong);
}

.error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}
</style>
