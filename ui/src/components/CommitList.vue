<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'

import { errorMessage, type CommitSummary, type HistoryCursor, type HistoryPage } from '../api'
import { scaled } from '../theme/scale'
import AppIcon from './AppIcon.vue'
import CommitEntry from './CommitEntry.vue'

/** How close to the bottom the next page starts loading. */
const PRELOAD_MARGIN_PX = scaled(750)

const props = defineProps<{
  load: (cursor: HistoryCursor | null) => Promise<HistoryPage>
  /** Changing this starts the list over from the newest commit. */
  sourceKey: string
  /** Changing this reloads the loaded commits in place, keeping the scroll position. */
  refreshKey: number
  active: boolean
  upstream: string | null
  emptyText: string
}>()

const emit = defineEmits<{ open: [commit: CommitSummary] }>()

const commits = shallowRef<CommitSummary[]>([])
const next = ref<HistoryCursor | null>(null)
const done = ref(false)
const truncated = ref(false)
const loading = ref(false)
const error = ref<string | null>(null)
const scroller = ref<HTMLElement | null>(null)
const sentinel = ref<HTMLElement | null>(null)
let generation = 0
let observer: IntersectionObserver | null = null
/** The commit the list's history starts from; refreshes stay on it. */
let listHead: string | null = null
let refreshPending = false

type Row =
  | { type: 'commit'; key: string; commit: CommitSummary }
  | { type: 'marker'; key: string; pushed: boolean }

const rows = computed<Row[]>(() => {
  const result: Row[] = []
  commits.value.forEach((commit, index) => {
    const previous = index > 0 ? commits.value[index - 1] : null
    // Markers only separate unpushed from pushed commits, so a list that
    // starts with pushed commits gets no marker at the top.
    if (commit.isPushed !== null) {
      const startsUnpushed = previous === null && commit.isPushed === false
      const flips = previous !== null && previous.isPushed !== null && previous.isPushed !== commit.isPushed
      if (startsUnpushed || flips) {
        result.push({ type: 'marker', key: `marker-${commit.id}`, pushed: commit.isPushed })
      }
    }
    result.push({ type: 'commit', key: commit.id, commit })
  })
  return result
})

async function loadMore(): Promise<void> {
  if (loading.value || done.value || error.value !== null || !props.active) {
    return
  }
  const startedGeneration = generation
  loading.value = true
  try {
    const page = await props.load(next.value)
    if (startedGeneration !== generation) {
      return
    }
    commits.value = [...commits.value, ...page.commits]
    listHead ??= page.head
    next.value = page.next
    done.value = page.next === null
    truncated.value = page.truncated
  } catch (caught) {
    if (startedGeneration === generation) {
      error.value = errorMessage(caught)
    }
  } finally {
    if (startedGeneration === generation) {
      loading.value = false
    }
  }
  if (startedGeneration !== generation) {
    return
  }
  if (refreshPending) {
    refreshPending = false
    void refreshInPlace()
    return
  }
  await continueIfNearBottom(startedGeneration)
}

async function continueIfNearBottom(startedGeneration: number): Promise<void> {
  await nextTick()
  if (startedGeneration === generation && sentinelIsNear()) {
    void loadMore()
  }
}

function sentinelIsNear(): boolean {
  if (!scroller.value || !sentinel.value) {
    return false
  }
  const container = scroller.value.getBoundingClientRect()
  return sentinel.value.getBoundingClientRect().top <= container.bottom + PRELOAD_MARGIN_PX
}

function reset(): void {
  generation += 1
  commits.value = []
  next.value = null
  done.value = false
  truncated.value = false
  loading.value = false
  error.value = null
  listHead = null
  refreshPending = false
  scroller.value?.scrollTo({ top: 0 })
  void loadMore()
}

/**
 * Reloads as many commits as are shown, from the same starting commit, and
 * swaps them in at once. Used after fetches, which change push markers but
 * shouldn't move the list. Page loads wait while it runs.
 */
async function refreshInPlace(): Promise<void> {
  if (loading.value) {
    refreshPending = true
    return
  }
  if (commits.value.length === 0 || listHead === null) {
    return
  }
  const startedGeneration = generation
  const target = commits.value.length
  const collected: CommitSummary[] = []
  let last: HistoryPage | null = null
  loading.value = true
  try {
    do {
      last = await props.load(last ? last.next : { head: listHead, skip: 0 })
      collected.push(...last.commits)
    } while (last.next !== null && collected.length < target)
  } catch {
    // The list stays as it was; a failed refresh isn't worth interrupting for.
    last = null
  } finally {
    if (startedGeneration === generation) {
      loading.value = false
    }
  }
  if (startedGeneration !== generation) {
    return
  }
  if (last) {
    commits.value = collected
    next.value = last.next
    done.value = last.next === null
    truncated.value = last.truncated
  }
  await continueIfNearBottom(startedGeneration)
}

function retry(): void {
  error.value = null
  void loadMore()
}

watch(() => props.sourceKey, reset)
watch(() => props.refreshKey, () => void refreshInPlace())
watch(
  () => props.active,
  async (active) => {
    if (active) {
      await nextTick()
      if (sentinelIsNear()) {
        void loadMore()
      }
    }
  },
)

onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        void loadMore()
      }
    },
    { root: scroller.value, rootMargin: `0px 0px ${PRELOAD_MARGIN_PX}px 0px` },
  )
  if (sentinel.value) {
    observer.observe(sentinel.value)
  }
  void loadMore()
})

onBeforeUnmount(() => observer?.disconnect())
</script>

<template>
  <div ref="scroller" class="commit-list">
    <template v-for="row in rows" :key="row.key">
      <div v-if="row.type === 'marker'" class="marker" :class="row.pushed ? 'marker--pushed' : 'marker--unpushed'">
        <AppIcon :name="row.pushed ? 'check' : 'push'" />
        <span>{{ row.pushed ? `Pushed to ${upstream ?? 'the upstream'}` : 'Not pushed yet' }}</span>
      </div>
      <CommitEntry v-else :commit="row.commit" @open="emit('open', row.commit)" />
    </template>

    <div v-if="error" class="status status--error">
      <span>{{ error }}</span>
      <button class="button" type="button" @click="retry">Retry</button>
    </div>
    <div v-else-if="loading" class="status faint">Loading commits…</div>
    <div v-else-if="done && commits.length === 0" class="empty-state">{{ emptyText }}</div>
    <div v-else-if="done && truncated" class="marker marker--truncated">
      <AppIcon name="warning" />
      <span>History is truncated here: this is a shallow clone.</span>
    </div>

    <div ref="sentinel" class="sentinel" aria-hidden="true"></div>
  </div>
</template>

<style scoped>
.commit-list {
  height: 100%;
  overflow-y: auto;
}

.marker {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-4);
  border-bottom: 1px solid var(--color-border-subtle);
  font-size: var(--font-size-small);
  font-weight: 500;
}

.marker::after {
  content: '';
  flex: 1;
  height: 1px;
  background: currentColor;
  opacity: 0.35;
}

.marker--unpushed {
  background: var(--color-accent-muted);
  color: var(--color-marker-unpushed);
}

.marker--pushed {
  background: var(--color-bg-panel);
  color: var(--color-marker-pushed);
}

.marker--truncated {
  border-bottom: none;
  color: var(--color-warning);
}

.status {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
}

.status--error {
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.sentinel {
  height: 1px;
}
</style>
