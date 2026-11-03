<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type StashDetails } from '../api'
import DiffBrowser from '../components/DiffBrowser.vue'
import StashActions from '../components/StashActions.vue'
import ViewBar from '../components/ViewBar.vue'
import { goBack, openCommit } from '../stores/navigation'
import { repo } from '../stores/repo'
import { formatAbsolute, formatRelative } from '../utils/dates'
import { useEscape } from '../utils/escape'
import { pendingCounts, statesOf, type PendingState } from '../utils/pending'
import { stashName } from '../utils/stashes'

const props = defineProps<{ id: string }>()

const details = shallowRef<StashDetails | null>(null)
const error = ref<string | null>(null)
let generation = 0

useEscape(goBack)

const counts = computed(() => (details.value ? pendingCounts(details.value.changes) : []))

const statesByPath = computed(
  () => new Map((details.value?.states ?? []).map((file) => [file.path, statesOf(file)])),
)

function statesFor(path: string): PendingState[] {
  return statesByPath.value.get(path) ?? []
}

async function load(): Promise<void> {
  generation += 1
  const started = generation
  try {
    const loaded = await api.stash(props.id)
    if (started === generation) {
      details.value = loaded
      error.value = null
    }
  } catch (caught) {
    if (started === generation) {
      error.value = errorMessage(caught)
    }
  }
}

/** Popping or dropping the stash leaves nothing to show here. */
function onActionDone(action: 'apply' | 'pop' | 'drop'): void {
  if (action !== 'apply') {
    goBack()
  }
}

onMounted(load)
// The stash keeps its id but moves down as newer ones are added, and is gone
// once dropped outside GitEagle.
watch(() => [repo.stashRevision, repo.revision], load)
</script>

<template>
  <div class="stash-view">
    <ViewBar icon="stash">
      <template #title>
        Stashed changes <span v-if="details" class="hash">{{ stashName(details.index) }}</span>
      </template>
      <template #end>
        <StashActions v-if="details && !error" :stash="details" @done="onActionDone" />
      </template>
    </ViewBar>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="!details" class="loading faint">Loading stash…</p>

    <DiffBrowser v-else :files="details.files" :stats="details.stats" empty-text="This stash changes no files.">
      <template #header>
        <header class="stash-header">
          <dl class="detail-list">
            <dt>Stash</dt>
            <dd>
              <span class="hash">{{ stashName(details.index) }}</span>
              <span class="faint stash-id" :title="details.id">{{ details.shortId }}</span>
            </dd>
            <dt>Branch</dt>
            <dd>
              <span v-if="details.branch">{{ details.branch }}</span>
              <span v-else class="faint">detached HEAD</span>
            </dd>
            <dt>Based on</dt>
            <dd>
              <button class="base hash" type="button" title="Open this commit" @click="openCommit(details.base.id)">
                {{ details.base.shortId }}
              </button>
              <span class="muted">{{ details.base.subject }}</span>
            </dd>
            <dt>Stashed</dt>
            <dd>
              {{ formatAbsolute(details.authorTime) }}
              <span class="faint">({{ formatRelative(details.authorTime) }})</span>
              <span class="muted">by {{ details.author.name }}</span>
            </dd>
            <dt>Files</dt>
            <dd>
              <span v-if="counts.length === 0" class="faint">none</span>
              <!-- One span, so the commas aren't flex items set apart by the gap. -->
              <span v-else>
                <template v-for="({ state, count }, index) in counts" :key="state">
                  {{ index > 0 ? ', ' : '' }}{{ count }} <span :class="`state-${state}`">{{ state }}</span>
                </template>
              </span>
            </dd>
            <dt>Changes</dt>
            <dd>
              <span>
                {{ details.stats.filesChanged }} {{ details.stats.filesChanged === 1 ? 'file' : 'files' }},
                <span class="additions">+{{ details.stats.additions }}</span>,
                <span class="deletions">−{{ details.stats.deletions }}</span>
              </span>
            </dd>
          </dl>

          <h1 v-if="details.message" class="subject">{{ details.message }}</h1>
          <!-- One line, so the spaces between the spans are kept. -->
          <h1 v-else class="subject subject--wip">WIP on <span class="hash">{{ details.base.shortId }}</span> {{ details.base.subject }}</h1>
        </header>
      </template>

      <template #file-label="{ file }">
        <span class="file-states">
          <span v-for="state in statesFor(file.path)" :key="state" :class="`state-${state}`">{{ state }}</span>
        </span>
      </template>
    </DiffBrowser>
  </div>
</template>

<style scoped>
.stash-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.error {
  margin: 0;
  padding: var(--space-4);
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.loading {
  margin: 0;
  padding: var(--space-4);
}

.stash-header {
  padding: var(--space-4) 0;
}

.stash-id {
  user-select: text;
}

/* The margin cancels the padding, so the hash lines up with the other
   values while its hover background keeps some room. */
.base {
  margin-inline: calc(-1 * var(--space-1));
  padding: 0 var(--space-1);
  border: none;
  border-radius: var(--radius);
  background: transparent;
  cursor: pointer;
  transition: background-color var(--transition);
}

.base:hover {
  background: var(--color-bg-hover);
}

.subject {
  margin: 0;
  color: var(--color-fg-strong);
  font-size: var(--font-size-title);
  font-weight: 600;
  line-height: 1.35;
  overflow-wrap: anywhere;
  user-select: text;
}

.subject--wip {
  color: var(--color-fg-muted);
  font-weight: 400;
}

.file-states {
  display: flex;
  flex: none;
  gap: var(--space-1);
  font-size: var(--font-size-tiny);
}

.additions {
  color: var(--color-diff-added-sign);
}

.deletions {
  color: var(--color-diff-removed-sign);
}
</style>
