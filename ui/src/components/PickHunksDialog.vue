<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, provide, ref, shallowRef } from 'vue'

import { errorMessage, type FileDiff, type FilePick, type PendingChanges } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import ConfirmDialog from './ConfirmDialog.vue'
import DiffFile, { DIFF_SCROLLER } from './DiffFile.vue'

/** One question of the walk through the changes: a hunk, or a file that is picked whole. */
interface Choice {
  fileIndex: number
  /** `null` for a whole file. */
  hunk: { index: number; fingerprint: string } | null
}

const emit = defineEmits<{ close: [] }>()

const { api, refreshEverything, repo } = useProject()

const changes = shallowRef<PendingChanges | null>(null)
const loadError = ref<string | null>(null)
const step = ref<'picking' | 'message'>('picking')
/** The position in `choices` of the one asked about. */
const position = ref(0)
/** The positions in `choices` of the picked ones. */
const picked = ref(new Set<number>())
const message = ref('')
const working = ref(false)
const scroller = ref<HTMLElement | null>(null)
const pickButton = ref<HTMLButtonElement | null>(null)
const editor = ref<HTMLTextAreaElement | null>(null)

provide(DIFF_SCROLLER, scroller)

const files = computed(() => changes.value?.files ?? [])

// In diff order, which is path order, as `git add --patch` goes through them.
const choices = computed<Choice[]>(() =>
  files.value.flatMap((file, fileIndex): Choice[] => {
    const fingerprint = file.fingerprint
    if (fingerprint === null) {
      return [{ fileIndex, hunk: null }]
    }
    return file.hunks.map((_, index) => ({ fileIndex, hunk: { index, fingerprint } }))
  }),
)

const current = computed(() => choices.value[position.value] ?? null)
const currentFile = computed(() => (current.value ? files.value[current.value.fileIndex] : null))

const branch = computed(() => repo.overview?.head.name ?? 'the current branch')

/** Where the current hunk is among its file's, e.g. "hunk 2 of 5 in this file". */
const hunkPosition = computed(() => {
  const choice = current.value
  const file = currentFile.value
  if (!choice?.hunk || !file) {
    return null
  }
  return file.hunks.length === 1 ? 'the only hunk in this file' : `hunk ${choice.hunk.index + 1} of ${file.hunks.length} in this file`
})

/** Why a file can only be picked whole. */
function wholeFileReason(file: FileDiff): string {
  if (file.kind === 'added') {
    return 'A new file is picked whole.'
  }
  if (file.kind === 'deleted') {
    return 'A deleted file is picked whole.'
  }
  if (file.kind === 'renamed' || file.kind === 'copied') {
    return `A ${file.kind} file is picked whole.`
  }
  if (file.kind === 'typeChange') {
    return 'A file whose type changed is picked whole.'
  }
  if (file.isBinary) {
    return 'A binary file is picked whole.'
  }
  if (file.tooLarge) {
    return 'This diff is too large to pick hunks from, so the file is picked whole.'
  }
  if (file.oldMode !== file.newMode) {
    return 'A file whose mode changed is picked whole.'
  }
  return 'This file is picked whole.'
}

/** The files with something picked, and what, in the order they were asked about. */
const pickedFiles = computed(() => {
  const byFile = new Map<number, Choice[]>()
  choices.value.forEach((choice, index) => {
    if (picked.value.has(index)) {
      byFile.set(choice.fileIndex, [...(byFile.get(choice.fileIndex) ?? []), choice])
    }
  })
  return [...byFile].map(([fileIndex, chosen]) => {
    const file = files.value[fileIndex]
    const hunks = chosen.flatMap((choice) => (choice.hunk ? [choice.hunk] : []))
    const pick: FilePick = {
      path: file.path,
      oldPath: file.oldPath,
      hunks: hunks.length > 0 ? { fingerprint: hunks[0].fingerprint, indices: hunks.map((hunk) => hunk.index) } : null,
    }
    const what = pick.hunks ? `${hunks.length} of ${file.hunks.length} ${file.hunks.length === 1 ? 'hunk' : 'hunks'}` : 'whole file'
    return { file, pick, what }
  })
})

const canCommit = computed(() => pickedFiles.value.length > 0 && message.value.trim() !== '')

async function showChoice(): Promise<void> {
  await nextTick()
  scroller.value?.scrollTo({ top: 0 })
}

/** Answers the current question, like `y` or `n` in `git add --patch`. */
function answer(pick: boolean): void {
  if (step.value !== 'picking' || current.value === null) {
    return
  }
  if (pick) {
    picked.value = new Set([...picked.value, position.value])
  }
  position.value += 1
  if (position.value < choices.value.length) {
    void showChoice()
  } else {
    finishPicking()
  }
}

/** Leaves the rest out, like `q` in `git add --patch`, and goes on with what was picked so far. */
function skipRest(): void {
  if (step.value !== 'picking' || current.value === null) {
    return
  }
  finishPicking()
}

function finishPicking(): void {
  if (picked.value.size > 0) {
    void startMessage()
  } else {
    showToast('info', 'Nothing was picked, so nothing was committed.')
    emit('close')
  }
}

async function startMessage(): Promise<void> {
  step.value = 'message'
  await nextTick()
  editor.value?.focus()
}

async function confirm(): Promise<void> {
  if (step.value !== 'message' || !canCommit.value || working.value) {
    return
  }
  working.value = true
  repo.busy = 'commit'
  let shortId: string
  try {
    shortId = (await api.commitChanges(message.value, pickedFiles.value.map(({ pick }) => pick))).shortId
  } catch (error) {
    // Nothing changed, so the message stays open for another try.
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', `Committed ${shortId}.`)
  emit('close')
  await refreshEverything()
}

function onKeydown(event: KeyboardEvent): void {
  if (step.value !== 'picking' || current.value === null) {
    return
  }
  // Held keys would race through the hunks unseen.
  if (event.ctrlKey || event.metaKey || event.altKey || event.repeat) {
    return
  }
  const key = event.key.toLowerCase()
  if (key === 'y' || key === 'n' || key === 'q') {
    event.preventDefault()
    if (key === 'q') {
      skipRest()
    } else {
      answer(key === 'y')
    }
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  try {
    changes.value = await api.pendingChanges()
  } catch (error) {
    loadError.value = errorMessage(error)
    return
  }
  // The dialog had nothing to focus before the changes arrived.
  await nextTick()
  pickButton.value?.focus()
})

onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <ConfirmDialog
    title="Commit (Pick Hunks)"
    icon="pickHunks"
    confirm-label="Commit"
    working-label="Committing…"
    :working="working"
    :disabled="!canCommit"
    :large="step === 'picking'"
    :wide="step === 'message'"
    @confirm="confirm"
    @close="emit('close')"
  >
    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="!changes" class="faint">Loading the changes…</p>
    <p v-else-if="choices.length === 0" class="faint">There are no uncommitted changes.</p>

    <template v-else-if="step === 'picking' && current && currentFile">
      <div class="progress">
        <span class="counter">{{ position + 1 }} / {{ choices.length }}</span>
        <span class="muted">{{ hunkPosition ?? wholeFileReason(currentFile) }}</span>
        <span class="picked faint">{{ picked.size }} picked</span>
      </div>
      <div ref="scroller" class="hunk-scroller">
        <DiffFile
          :key="current.fileIndex"
          class="hunk"
          :file="currentFile"
          :highlighted="false"
          :commit="null"
          :base="null"
          :hunk-index="current.hunk?.index ?? null"
          :file-actions="false"
        />
      </div>
    </template>

    <template v-else-if="step === 'message'">
      <p class="muted">
        Commits what was picked onto {{ branch }}. The working tree stays as it is, and what was left out stays
        uncommitted.
      </p>
      <ul class="picked-files" aria-label="Picked changes">
        <li v-for="{ file, what } in pickedFiles" :key="file.path" class="picked-file">
          <span class="path ellipsis">
            <template v-if="file.oldPath">
              <span class="muted">{{ file.oldPath }}</span>
              <span class="faint"> → </span>
            </template>
            {{ file.path }}
          </span>
          <span class="what faint">{{ what }}</span>
        </li>
      </ul>
      <label class="field">
        <span class="muted">Message</span>
        <textarea
          ref="editor"
          v-model="message"
          class="text-input"
          rows="6"
          spellcheck="false"
          :disabled="working"
          @keydown.ctrl.enter.prevent="confirm"
        ></textarea>
      </label>
    </template>

    <template v-if="step === 'picking'" #actions>
      <button class="button cancel" type="button" title="Stop without committing anything (Esc)" @click="emit('close')">
        Cancel
        <span class="key">Esc</span>
      </button>
      <button
        class="button"
        type="button"
        :disabled="!current"
        title="Leave out this and the rest, and commit what was picked so far (Q)"
        @click="skipRest"
      >
        Skip Rest
        <span class="key">Q</span>
      </button>
      <button class="button" type="button" :disabled="!current" title="Leave this out of the commit (N)" @click="answer(false)">
        Skip
        <span class="key">N</span>
      </button>
      <button
        ref="pickButton"
        class="button"
        type="button"
        :disabled="!current"
        title="Take this into the commit (Y)"
        @click="answer(true)"
      >
        Pick
        <span class="key">Y</span>
      </button>
    </template>
  </ConfirmDialog>
</template>

<style scoped>
.error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.progress {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  min-width: 0;
}

.counter {
  flex: none;
  padding: 0 var(--space-2);
  border-radius: var(--radius);
  background: var(--color-bg-active);
  color: var(--color-fg-strong);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.picked {
  flex: none;
  margin-left: auto;
  font-size: var(--font-size-small);
}

/* The same height for every hunk, so the buttons stay under the mouse. */
.hunk-scroller {
  height: min(calc(560 * var(--px)), 60vh);
  overflow-y: auto;
  border-radius: var(--radius-large);
}

/* Over the margin that sets files apart in a list of diffs. */
.hunk-scroller > .hunk {
  margin-bottom: 0;
}

/* Set apart from the answers, so a stray click doesn't throw the picks away. */
.cancel {
  margin-right: auto;
}

/* The key that answers the same way, as in `git add --patch`. */
.key {
  padding: 0 calc(4 * var(--px));
  border: 1px solid var(--color-border);
  border-radius: calc(3 * var(--px));
  color: var(--color-fg-muted);
  font-size: var(--font-size-tiny);
  line-height: 1.3;
}

.button:disabled .key {
  color: var(--color-fg-disabled);
}

.picked-files {
  max-height: calc(200 * var(--px));
  margin: 0;
  padding: 0;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  list-style: none;
}

.picked-file {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
}

.picked-file + .picked-file {
  border-top: 1px solid var(--color-border-subtle);
}

.path {
  flex: 1;
  min-width: 0;
}

.what {
  flex: none;
  font-size: var(--font-size-small);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
</style>
