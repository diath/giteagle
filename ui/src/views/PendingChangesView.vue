<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue'

import { api, errorMessage, type PendingChanges } from '../api'
import AppIcon from '../components/AppIcon.vue'
import DiffBrowser from '../components/DiffBrowser.vue'
import { goBack } from '../stores/navigation'
import { repo } from '../stores/repo'
import { pendingRevision } from '../stores/workingTree'
import { useEscape } from '../utils/escape'
import { pendingCounts, statesOf, type PendingState } from '../utils/pending'

const changes = shallowRef<PendingChanges | null>(null)
const error = ref<string | null>(null)
let generation = 0

useEscape(goBack)

const head = computed(() => repo.overview?.head.tip ?? null)
const counts = computed(() => (changes.value ? pendingCounts(changes.value.summary) : []))

const statesByPath = computed(
  () => new Map((changes.value?.states ?? []).map((file) => [file.path, statesOf(file)])),
)

function statesFor(path: string): PendingState[] {
  return statesByPath.value.get(path) ?? []
}

async function load(): Promise<void> {
  generation += 1
  const started = generation
  try {
    const loaded = await api.pendingChanges()
    if (started === generation) {
      changes.value = loaded
      error.value = null
    }
  } catch (caught) {
    if (started === generation) {
      error.value = errorMessage(caught)
    }
  }
}

onMounted(load)
// Follows edits as they happen, and HEAD moving after a reload.
watch(() => [pendingRevision.value, repo.revision], load)
</script>

<template>
  <div class="pending-view">
    <div class="top-bar">
      <button class="button button--quiet" type="button" title="Back (Esc)" @click="goBack">
        <AppIcon name="back" />
        Back
      </button>
      <span class="title">Uncommitted changes</span>
    </div>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="!changes" class="loading faint">Loading uncommitted changes…</p>

    <DiffBrowser v-else :files="changes.files" :stats="changes.stats" empty-text="There are no uncommitted changes.">
      <template #header>
        <header class="pending-header">
          <dl class="detail-list">
            <dt>Compared with</dt>
            <dd>
              <template v-if="head">
                <span class="hash">{{ head.shortId }}</span>
                <span class="muted">{{ head.subject }}</span>
              </template>
              <span v-else class="faint">no commits yet</span>
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
                {{ changes.stats.filesChanged }} {{ changes.stats.filesChanged === 1 ? 'file' : 'files' }},
                <span class="additions">+{{ changes.stats.additions }}</span>,
                <span class="deletions">−{{ changes.stats.deletions }}</span>
              </span>
            </dd>
          </dl>
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
.pending-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.top-bar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--bar-height);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.title {
  color: var(--color-pending);
  font-weight: 600;
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

.pending-header {
  padding: var(--space-4) 0 0;
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
