<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type FileVersions } from '../api'
import type { IconName } from '../icons/paths'
import { useProject } from '../stores/project'
import { splitBeforeLast } from '../utils/paths'
import AppIcon from './AppIcon.vue'
import ViewBar from './ViewBar.vue'

/**
 * The bar of a view of one version of a file: its path and commit, between
 * steps to the file's previous and next versions, which the view on top of
 * the stack switches to.
 */
const props = defineProps<{ icon: IconName; commit: string; shortId: string; path: string }>()

defineSlots<{
  /** At the right end of the bar. */
  end?: () => unknown
}>()

const { api, navigation, replaceFileVersion } = useProject()

const versions = shallowRef<FileVersions | null>(null)
const versionsError = ref<string | null>(null)

// The view is only seen on top of the stack, so Back leads to the one below.
const backTitle = computed(() => {
  switch (navigation.stack[navigation.stack.length - 2]?.kind) {
    case 'main':
      return 'Back to the files (Esc)'
    case 'commit':
      return 'Back to the commit (Esc)'
    case 'file':
      return 'Back to the file (Esc)'
    default:
      return 'Back (Esc)'
  }
})

const olderTitle = computed(() => {
  if (versionsError.value) {
    return versionsError.value
  }
  if (!versions.value) {
    return 'Looking for other versions…'
  }
  const older = versions.value.older
  return older ? `Previous version: ${older.shortId} ${older.subject}` : 'This is the oldest version'
})

const newerTitle = computed(() => {
  if (versionsError.value) {
    return versionsError.value
  }
  if (!versions.value) {
    return 'Looking for other versions…'
  }
  const { newer, onBranch } = versions.value
  if (newer) {
    return `Next version: ${newer.shortId} ${newer.subject}`
  }
  return onBranch
    ? 'This is the newest version on the current branch'
    : "Newer versions are only looked for on the current branch, which doesn't have this commit"
})

function showVersion(commit: { id: string; shortId: string } | null | undefined): void {
  if (commit) {
    replaceFileVersion(props.path, props.commit, commit)
  }
}

onMounted(async () => {
  try {
    versions.value = await api.fileVersions(props.commit, props.path)
  } catch (caught) {
    versionsError.value = errorMessage(caught)
  }
})
</script>

<template>
  <ViewBar class="file-version-bar" :icon="icon" :back-title="backTitle" keep-end>
    <template #before-title>
      <button
        class="button step"
        type="button"
        :disabled="!versions?.older"
        :title="olderTitle"
        aria-label="Previous version"
        @click="showVersion(versions?.older)"
      >
        <AppIcon name="chevronLeft" />
        <span class="step-label">Previous</span>
      </button>
    </template>
    <template #title>
      <span class="path" :title="path">
        <span class="path-start ellipsis">{{ splitBeforeLast(path).start }}</span>
        <span class="path-end ellipsis">{{ splitBeforeLast(path).end }}</span>
      </span>
      <span class="at faint">@</span>
      <span class="hash" :title="commit">{{ shortId }}</span>
    </template>
    <template #after-title>
      <button
        class="button step"
        type="button"
        :disabled="!versions?.newer"
        :title="newerTitle"
        aria-label="Next version"
        @click="showVersion(versions?.newer)"
      >
        <span class="step-label">Next</span>
        <AppIcon name="chevronRight" />
      </button>
    </template>
    <template #end>
      <slot name="end" />
    </template>
  </ViewBar>
</template>

<style scoped>
.file-version-bar {
  container-type: inline-size;
}

/* In a narrow window the steps keep only their arrows, leaving the path
   more room. The width is in em, which here follows the font size setting. */
@container (max-width: 72em) {
  .step {
    justify-content: center;
    width: var(--control-height);
    padding: 0;
  }

  .step-label {
    display: none;
  }
}

/* Only the start of a long path gives way, and the file name only once the
   start is gone, so both ends of the path stay in view. */
.path {
  display: flex;
  min-width: 0;
}

.path-start {
  min-width: 0;
}

.path-end {
  flex: none;
  max-width: 100%;
}

/* Only the path gives way when the title is too long. */
.at,
.hash {
  flex: none;
}
</style>
