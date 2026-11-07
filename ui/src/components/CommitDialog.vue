<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type FilePick, type PendingChanges } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import { statesOf, type PendingState } from '../utils/pending'
import AppIcon from './AppIcon.vue'
import ConfirmDialog from './ConfirmDialog.vue'
import { KIND_LABELS } from './DiffFile.vue'

const emit = defineEmits<{ close: [] }>()

const { api, refreshEverything, repo } = useProject()

const changes = shallowRef<PendingChanges | null>(null)
const loadError = ref<string | null>(null)
const message = ref('')
/** The paths of the checked files. */
const chosen = ref<string[]>([])
const working = ref(false)
const editor = ref<HTMLTextAreaElement | null>(null)

const files = computed(() => changes.value?.files ?? [])
const allChosen = computed(() => files.value.length > 0 && chosen.value.length === files.value.length)
const someChosen = computed(() => chosen.value.length > 0 && !allChosen.value)
const canCommit = computed(() => chosen.value.length > 0 && message.value.trim() !== '')
const branch = computed(() => repo.overview?.head.name ?? 'the current branch')

const statesByPath = computed(
  () => new Map((changes.value?.states ?? []).map((file) => [file.path, statesOf(file)])),
)

function statesFor(path: string): PendingState[] {
  return statesByPath.value.get(path) ?? []
}

function toggleAll(): void {
  chosen.value = allChosen.value ? [] : files.value.map((file) => file.path)
}

async function confirm(): Promise<void> {
  if (!canCommit.value || working.value) {
    return
  }
  const picks: FilePick[] = files.value
    .filter((file) => chosen.value.includes(file.path))
    .map((file) => ({ path: file.path, oldPath: file.oldPath }))
  working.value = true
  repo.busy = 'commit'
  let shortId: string
  try {
    shortId = (await api.commitChanges(message.value, picks)).shortId
  } catch (error) {
    // Nothing changed, so the dialog stays open for another try.
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

onMounted(async () => {
  try {
    changes.value = await api.pendingChanges()
  } catch (error) {
    loadError.value = errorMessage(error)
    return
  }
  chosen.value = changes.value.files.map((file) => file.path)
  await nextTick()
  editor.value?.focus()
})
</script>

<template>
  <ConfirmDialog
    title="Commit"
    icon="commit"
    confirm-label="Commit"
    working-label="Committing…"
    :working="working"
    :disabled="!canCommit"
    wide
    @confirm="confirm"
    @close="emit('close')"
  >
    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="!changes" class="faint">Loading the changes…</p>
    <p v-else-if="files.length === 0" class="faint">There are no uncommitted changes.</p>
    <template v-else>
      <p class="muted">
        Commits the checked files onto {{ branch }}, as they are in the working tree. Staged changes to the
        other files stay staged.
      </p>
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
      <div class="field">
        <label class="all-files">
          <input
            type="checkbox"
            :checked="allChosen"
            :indeterminate="someChosen"
            :disabled="working"
            @change="toggleAll"
          />
          <span class="muted">Files</span>
          <span class="count faint">{{ chosen.length }} of {{ files.length }} checked</span>
        </label>
        <ul class="files" aria-label="Files to commit">
          <li v-for="file in files" :key="file.path">
            <label class="file" :title="`${KIND_LABELS[file.kind]}: ${file.oldPath ? `${file.oldPath} → ` : ''}${file.path}`">
              <input v-model="chosen" type="checkbox" :value="file.path" :disabled="working" />
              <AppIcon name="file" class="file-icon" :class="`file-icon--${file.kind}`" />
              <span class="path ellipsis">
                <template v-if="file.oldPath">
                  <span class="muted">{{ file.oldPath }}</span>
                  <span class="faint"> → </span>
                </template>
                {{ file.path }}
              </span>
              <span class="states">
                <span v-for="state in statesFor(file.path)" :key="state" :class="`state-${state}`">{{ state }}</span>
              </span>
              <span class="stats">
                <span class="additions">+{{ file.additions }}</span>
                <span class="deletions">−{{ file.deletions }}</span>
              </span>
            </label>
          </li>
        </ul>
      </div>
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

.all-files {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding-inline: var(--space-3);
  cursor: pointer;
}

.count {
  margin-left: auto;
  font-size: var(--font-size-small);
}

input[type='checkbox'] {
  flex: none;
  margin: 0;
  accent-color: var(--color-accent);
}

.files {
  max-height: calc(320 * var(--px));
  margin: 0;
  padding: 0;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  list-style: none;
}

.files li + li {
  border-top: 1px solid var(--color-border-subtle);
}

.file {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  cursor: pointer;
  transition: background-color var(--transition);
}

.file:hover {
  background: var(--color-bg-hover);
}

.file-icon {
  flex: none;
}

.file-icon--added {
  color: var(--color-status-added);
}

.file-icon--deleted {
  color: var(--color-status-deleted);
}

.file-icon--modified {
  color: var(--color-status-modified);
}

.file-icon--renamed,
.file-icon--copied {
  color: var(--color-status-renamed);
}

.file-icon--typeChange {
  color: var(--color-status-type-change);
}

.path {
  flex: 1;
  min-width: 0;
}

.states {
  flex: none;
  font-size: var(--font-size-tiny);
}

.states > * + * {
  margin-left: var(--space-1);
}

.stats {
  display: flex;
  flex: none;
  gap: 1ch;
  font-size: var(--font-size-small);
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}
</style>
