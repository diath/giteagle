<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef, watch } from 'vue'

import { errorMessage, type CommitSummary, type RewritableCommit } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import ConfirmDialog from './ConfirmDialog.vue'

const props = withDefaults(
  defineProps<{
    commit: CommitSummary
    /** The ids of the commits to check from the start, e.g. those checked in the commit list. */
    preselected?: string[]
  }>(),
  { preselected: () => [] },
)
const emit = defineEmits<{ close: []; squashed: [] }>()

const { api, refreshEverything, repo } = useProject()

/** Newest first, as in the commit list. */
const commits = shallowRef<RewritableCommit[] | null>(null)
const loadError = ref<string | null>(null)
const chosen = ref<string[]>([])
const message = ref('')
/** Once the message has been edited, checking commits leaves it alone. */
const messageEdited = ref(false)
const working = ref(false)
const list = ref<HTMLElement | null>(null)

const isTarget = (candidate: RewritableCommit): boolean => candidate.id === props.commit.id

/** The commits that become one, oldest first, which is how their messages are combined. */
const squashed = computed(() =>
  (commits.value ?? [])
    .filter((candidate) => isTarget(candidate) || chosen.value.includes(candidate.id))
    .reverse(),
)

watch(squashed, (combined) => {
  if (!messageEdited.value) {
    message.value = combined.map((candidate) => candidate.message.trimEnd()).join('\n\n')
  }
})

const canSquash = computed(() => chosen.value.length > 0 && message.value.trim() !== '')

async function confirm(): Promise<void> {
  if (!canSquash.value || working.value) {
    return
  }
  const others = [...chosen.value]
  working.value = true
  repo.busy = 'squash'
  try {
    await api.squashCommits(props.commit.id, others, message.value)
  } catch (error) {
    // Nothing changed, so the dialog stays open for another choice.
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', `Squashed ${others.length + 1} commits into one.`)
  emit('squashed')
  emit('close')
  await refreshEverything()
}

onMounted(async () => {
  try {
    commits.value = await api.rewritableCommits(props.commit.id)
  } catch (error) {
    loadError.value = errorMessage(error)
    return
  }
  const loaded = commits.value
  chosen.value = props.preselected.filter(
    (id) => id !== props.commit.id && loaded.some((candidate) => candidate.id === id),
  )
  // The dialog had nothing to focus before the commits arrived.
  await nextTick()
  list.value?.querySelector<HTMLInputElement>('input:not(:disabled)')?.focus()
})
</script>

<template>
  <ConfirmDialog
    :title="`Squash into ${commit.shortId}`"
    icon="squash"
    confirm-label="Squash"
    working-label="Squashing…"
    :working="working"
    :disabled="!canSquash"
    wide
    @confirm="confirm"
    @close="emit('close')"
  >
    <p class="commit ellipsis"><span class="hash">{{ commit.shortId }}</span> {{ commit.subject }}</p>
    <p class="muted">
      Folds the commits you check into this one, which keeps its place in the history and its author.
    </p>

    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="!commits" class="faint">Loading commits…</p>
    <p v-else-if="commits.length === 1" class="faint">No other unpushed commits can be squashed into it.</p>
    <template v-else>
      <ul ref="list" class="commits" aria-label="Commits to squash">
        <li v-for="candidate in commits" :key="candidate.id">
          <label class="row" :class="{ target: isTarget(candidate) }">
            <input v-if="isTarget(candidate)" type="checkbox" checked disabled />
            <input v-else v-model="chosen" type="checkbox" :value="candidate.id" :disabled="working" />
            <span class="hash">{{ candidate.shortId }}</span>
            <span class="subject ellipsis">{{ candidate.subject }}</span>
            <span v-if="isTarget(candidate)" class="note faint">squashing into</span>
          </label>
        </li>
      </ul>
      <label class="field">
        <span class="muted">Message</span>
        <textarea
          v-model="message"
          class="text-input"
          rows="7"
          spellcheck="false"
          :disabled="working"
          @input="messageEdited = true"
          @keydown.ctrl.enter.prevent="confirm"
        ></textarea>
      </label>
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

.commits {
  max-height: calc(240 * var(--px));
  margin: 0;
  padding: 0;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  list-style: none;
}

.commits li + li {
  border-top: 1px solid var(--color-border-subtle);
}

.row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  cursor: pointer;
  transition: background-color var(--transition);
}

.row:hover:not(.target) {
  background: var(--color-bg-hover);
}

.row input {
  flex: none;
  margin: 0;
  accent-color: var(--color-accent);
}

.target {
  cursor: default;
}

.hash {
  flex: none;
}

.subject {
  min-width: 0;
}

.note {
  flex: none;
  margin-left: auto;
  font-size: var(--font-size-small);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
</style>
