<script setup lang="ts">
import { computed, nextTick, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type CommitSummary, type PickTarget } from '../api'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import { joinWithAnd } from '../utils/lists'
import ConfirmDialog from './ConfirmDialog.vue'

const props = defineProps<{
  /** One or more, in any order; each branch gets them oldest first. */
  commits: CommitSummary[]
}>()
const emit = defineEmits<{ close: []; picked: [] }>()

const { api, repo } = useProject()

const targets = shallowRef<PickTarget[] | null>(null)
const loadError = ref<string | null>(null)
const chosen = ref<string[]>([])
const working = ref(false)
const list = ref<HTMLElement | null>(null)

const single = computed(() => (props.commits.length === 1 ? props.commits[0] : null))

/** Oldest first, roughly the order the copies are made in. */
const shownCommits = computed(() => [...props.commits].sort((left, right) => left.authorTime - right.authorTime))

const title = computed(() =>
  single.value ? `Cherry-pick ${single.value.shortId}` : `Cherry-pick ${props.commits.length} commits`,
)

/** Why the commit can't go onto a branch, or null when it can. */
function blocker(target: PickTarget): string | null {
  if (target.isCurrent) {
    return 'current branch'
  }
  if (target.hasCommit) {
    return single.value ? 'already has this commit' : 'already has one of these commits'
  }
  if (target.inOtherWorktree) {
    return 'checked out in another worktree'
  }
  return null
}

const hasChoices = computed(() => targets.value?.some((target) => blocker(target) === null) ?? false)

async function confirm(): Promise<void> {
  // In list order, whatever order they were checked in.
  const branches = (targets.value ?? [])
    .map((target) => target.name)
    .filter((name) => chosen.value.includes(name))
  if (branches.length === 0 || working.value) {
    return
  }
  working.value = true
  repo.busy = 'cherryPick'
  try {
    await api.cherryPick(props.commits.map((commit) => commit.id), branches)
    const what = single.value ? single.value.shortId : `${props.commits.length} commits`
    showToast('success', `Cherry-picked ${what} to ${joinWithAnd(branches)}.`)
    emit('picked')
    emit('close')
  } catch (error) {
    showToast('error', errorMessage(error))
  } finally {
    working.value = false
    repo.busy = null
  }
}

onMounted(async () => {
  try {
    targets.value = await api.pickTargets(props.commits.map((commit) => commit.id))
  } catch (error) {
    loadError.value = errorMessage(error)
    return
  }
  // The dialog had nothing to focus before the branches arrived.
  await nextTick()
  list.value?.querySelector<HTMLInputElement>('input:not(:disabled)')?.focus()
})
</script>

<template>
  <ConfirmDialog
    :title="title"
    icon="cherryPick"
    confirm-label="Cherry-pick"
    working-label="Cherry-picking…"
    :working="working"
    :disabled="chosen.length === 0"
    @confirm="confirm"
    @close="emit('close')"
  >
    <p v-if="single" class="commit ellipsis"><span class="hash">{{ single.shortId }}</span> {{ single.subject }}</p>
    <ul v-else class="commits" aria-label="Commits to cherry-pick">
      <li v-for="commit in shownCommits" :key="commit.id" class="commit ellipsis">
        <span class="hash">{{ commit.shortId }}</span> {{ commit.subject }}
      </li>
    </ul>
    <p v-if="single" class="muted">
      Adds a copy of the commit on top of each branch you check. The current branch and the working tree
      stay as they are.
    </p>
    <p v-else class="muted">
      Adds copies of the commits on top of each branch you check, the oldest first. The current branch and
      the working tree stay as they are.
    </p>

    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-else-if="!targets" class="faint">Loading branches…</p>
    <template v-else>
      <ul ref="list" class="branches" aria-label="Branches to cherry-pick to">
        <li v-for="target in targets" :key="target.name">
          <label class="branch" :class="{ blocked: blocker(target) !== null }">
            <input
              v-model="chosen"
              type="checkbox"
              :value="target.name"
              :disabled="working || blocker(target) !== null"
            />
            <span class="name ellipsis">{{ target.name }}</span>
            <span v-if="blocker(target)" class="note faint">{{ blocker(target) }}</span>
          </label>
        </li>
      </ul>
      <p v-if="!hasChoices" class="faint">There are no other branches to cherry-pick to.</p>
    </template>
  </ConfirmDialog>
</template>

<style scoped>
.commit {
  color: var(--color-fg-strong);
}

.commits {
  max-height: calc(160 * var(--px));
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
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

.branch:hover:not(.blocked) {
  background: var(--color-bg-hover);
}

.branch input {
  flex: none;
  margin: 0;
  accent-color: var(--color-accent);
}

.name {
  min-width: 0;
}

.blocked {
  cursor: default;
}

.blocked .name {
  color: var(--color-fg-faint);
}

.note {
  flex: none;
  margin-left: auto;
  font-size: var(--font-size-small);
}
</style>
