<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue'

import { errorMessage, type BranchList, type CommitBrief } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import ConfirmDialog from './ConfirmDialog.vue'

interface BaseChoice {
  key: string
  label: string
  tip: CommitBrief
  isCurrent: boolean
}

const emit = defineEmits<{ close: [] }>()

const { api, refreshEverything, repo, returnToMain } = useProject()

const branches = shallowRef<BranchList | null>(null)
const loadError = ref<string | null>(null)
const name = ref('')
const chosenKey = ref<string | null>(null)
const working = ref(false)

const trimmedName = computed(() => name.value.trim())

const nameTaken = computed(
  () => branches.value?.local.some((row) => row.name === trimmedName.value) ?? false,
)

/** What is checked out comes first; the backend lists the current branch first among the local ones. */
const choices = computed<BaseChoice[]>(() => {
  if (!branches.value) {
    return []
  }
  const rows = [...branches.value.local, ...branches.value.remote].map((row) => ({
    key: `${row.isRemote ? 'remote' : 'local'}/${row.name}`,
    label: row.name,
    tip: row.tip,
    isCurrent: row.isCurrent,
  }))
  const head = repo.overview?.head
  if (head?.kind === 'detached' && head.tip) {
    return [{ key: 'detached', label: 'detached HEAD', tip: head.tip, isCurrent: true }, ...rows]
  }
  return rows
})

const chosen = computed(() => choices.value.find((choice) => choice.key === chosenKey.value) ?? null)

const canCreate = computed(() => trimmedName.value !== '' && !nameTaken.value && chosen.value !== null)

// A fetch can remove the branch that was chosen.
watch(choices, (updated) => {
  if (!updated.some((choice) => choice.key === chosenKey.value)) {
    chosenKey.value = updated[0]?.key ?? null
  }
})

async function load(): Promise<void> {
  try {
    branches.value = await api.branches()
    loadError.value = null
  } catch (error) {
    loadError.value = errorMessage(error)
  }
}

async function confirm(): Promise<void> {
  const base = chosen.value
  if (!canCreate.value || !base || working.value) {
    return
  }
  const newName = trimmedName.value
  working.value = true
  repo.busy = 'createBranch'
  try {
    await api.createBranch(newName, base.tip.id)
  } catch (error) {
    // Nothing changed, so the dialog stays open for another try.
    showToast('error', errorMessage(error))
    return
  } finally {
    working.value = false
    repo.busy = null
  }
  showToast('success', `Created ${newName} from ${base.label} and switched to it.`)
  returnToMain()
  emit('close')
  await refreshEverything()
}

watch(() => repo.metadataRevision, load)

onMounted(load)
</script>

<template>
  <ConfirmDialog
    title="New Branch"
    icon="branch"
    confirm-label="Create"
    working-label="Creating…"
    :working="working"
    :disabled="!canCreate"
    @confirm="confirm"
    @close="emit('close')"
  >
    <label class="field">
      <span class="muted">Name</span>
      <input v-model="name" class="text-input" type="text" spellcheck="false" autocomplete="off" :disabled="working" />
    </label>
    <p v-if="nameTaken" class="error">A branch named {{ trimmedName }} already exists.</p>

    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="!branches" class="faint">Loading branches…</p>
    <div v-else class="field">
      <span class="muted">Based on</span>
      <ul class="branches" aria-label="Branch to base the new branch on">
        <li v-for="choice in choices" :key="choice.key">
          <label class="branch">
            <input v-model="chosenKey" type="radio" name="base" :value="choice.key" :disabled="working" />
            <span class="name ellipsis" :class="{ current: choice.isCurrent }">{{ choice.label }}</span>
            <span v-if="choice.isCurrent" class="note faint">current</span>
            <span class="hash">{{ choice.tip.shortId }}</span>
          </label>
        </li>
      </ul>
    </div>
  </ConfirmDialog>
</template>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.branches {
  max-height: calc(320 * var(--px));
  margin: 0;
  padding: 0;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg-sunken);
  list-style: none;
}

.branches li + li {
  border-top: 1px solid var(--color-border-subtle);
}

.branch {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  cursor: pointer;
  transition: background-color var(--transition);
}

.branch:hover {
  background: var(--color-bg-hover);
}

.name {
  flex: 1;
  min-width: 0;
}

.name.current {
  color: var(--color-accent-bright);
  font-weight: 600;
}

.note {
  flex: none;
  font-size: var(--font-size-small);
}

.hash {
  flex: none;
}
</style>
