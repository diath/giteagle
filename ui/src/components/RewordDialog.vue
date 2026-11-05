<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'

import { errorMessage, type CommitSummary } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import ConfirmDialog from './ConfirmDialog.vue'

const props = defineProps<{ commit: CommitSummary }>()
const emit = defineEmits<{ close: [] }>()

const { api, refreshEverything, repo } = useProject()

const original = ref<string | null>(null)
const message = ref('')
/** How many commits come after this one and are recreated on top. */
const later = ref(0)
const loadError = ref<string | null>(null)
const working = ref(false)
const editor = ref<HTMLTextAreaElement | null>(null)

// Trailing whitespace is tidied away when the commit is made, so it
// doesn't count as a change.
const changed = computed(
  () =>
    original.value !== null &&
    message.value.trim() !== '' &&
    message.value.trimEnd() !== original.value.trimEnd(),
)

const description = computed(() => {
  const files = 'Only the message changes; the files stay as they are.'
  if (later.value === 0) {
    return files
  }
  const recreated =
    later.value === 1 ? 'The commit after it is' : `The ${later.value} commits after it are`
  return `${files} ${recreated} recreated on top.`
})

async function confirm(): Promise<void> {
  if (!changed.value || working.value) {
    return
  }
  working.value = true
  repo.busy = 'reword'
  try {
    await api.rewordCommit(props.commit.id, message.value)
  } catch (error) {
    // Nothing changed, so the message stays open for another try.
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', `Reworded ${props.commit.shortId}.`)
  emit('close')
  await refreshEverything()
}

onMounted(async () => {
  try {
    const commits = await api.rewritableCommits(props.commit.id)
    later.value = commits.findIndex((candidate) => candidate.id === props.commit.id)
    original.value = commits[later.value]?.message ?? ''
    message.value = original.value
  } catch (error) {
    loadError.value = errorMessage(error)
    return
  }
  await nextTick()
  editor.value?.focus()
})
</script>

<template>
  <ConfirmDialog
    :title="`Reword ${commit.shortId}`"
    icon="pencil"
    confirm-label="Reword"
    working-label="Rewording…"
    :working="working"
    :disabled="!changed"
    wide
    @confirm="confirm"
    @close="emit('close')"
  >
    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="original === null" class="faint">Loading the message…</p>
    <template v-else>
      <p class="muted">{{ description }}</p>
      <label class="field">
        <span class="muted">Message</span>
        <textarea
          ref="editor"
          v-model="message"
          class="text-input"
          rows="8"
          spellcheck="false"
          :disabled="working"
          @keydown.ctrl.enter.prevent="confirm"
        ></textarea>
      </label>
    </template>
  </ConfirmDialog>
</template>

<style scoped>
.error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
</style>
