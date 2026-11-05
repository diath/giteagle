<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type FileAtCommit, type FileVersions } from '../api'
import AppIcon from '../components/AppIcon.vue'
import ViewBar from '../components/ViewBar.vue'
import { useProject } from '../stores/project'
import { showToast } from '../stores/toasts'
import { buildSegments, splitLines } from '../utils/diff'
import { splitBeforeLast } from '../utils/paths'
import { useEscape } from '../utils/escape'
import { languageFor, tokenizeFile, type Token } from '../utils/highlight'

const props = defineProps<{ commit: string; shortId: string; path: string }>()

const { api, goBack, navigation, openFileHistory, replaceFileVersion } = useProject()

const file = shallowRef<FileAtCommit | null>(null)
const error = ref<string | null>(null)
const tokens = shallowRef<Token[][] | null>(null)
const versions = shallowRef<FileVersions | null>(null)
const versionsError = ref<string | null>(null)

useEscape(goBack)

const lines = computed(() => splitLines(file.value?.text ?? null) ?? [])

// Plain until the tokens arrive, so the text shows at once.
const rows = computed(() => lines.value.map((text, index) => buildSegments(text, tokens.value?.[index], undefined)))

/** One number per line, as a single text, which lays out far faster than a node per line. */
const lineNumbers = computed(() => lines.value.map((_, index) => index + 1).join('\n'))

// The view is only seen on top of the stack, so Back leads to the one below.
const backTitle = computed(() => {
  const below = navigation.stack[navigation.stack.length - 2]?.kind
  if (below === 'main') {
    return 'Back to the files (Esc)'
  }
  return below === 'commit' ? 'Back to the commit (Esc)' : 'Back (Esc)'
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

function showBlame(): void {
  showToast('info', 'Blame is not implemented yet.')
}

function formatSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} ${bytes === 1 ? 'byte' : 'bytes'}`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function loadVersions(): Promise<void> {
  try {
    versions.value = await api.fileVersions(props.commit, props.path)
  } catch (caught) {
    versionsError.value = errorMessage(caught)
  }
}

onMounted(async () => {
  void loadVersions()
  try {
    const loaded = await api.fileAt(props.commit, props.path)
    file.value = loaded
    if (loaded.text !== null) {
      tokens.value = await tokenizeFile(loaded.text, languageFor(props.path))
    }
  } catch (caught) {
    error.value = errorMessage(caught)
  }
})
</script>

<template>
  <div class="file-view">
    <ViewBar icon="file" :back-title="backTitle" keep-end>
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
        <button
          class="button"
          type="button"
          title="Show the commits on this branch that changed this file"
          @click="openFileHistory(path)"
        >
          <AppIcon name="history" />
          History
        </button>
        <button class="button" type="button" title="Show who last changed each line" @click="showBlame">
          <AppIcon name="blame" />
          Blame
        </button>
      </template>
    </ViewBar>

    <p v-if="error" class="message error">{{ error }}</p>
    <p v-else-if="!file" class="message faint">Loading the file…</p>
    <p v-else-if="file.isBinary" class="message muted">This is a binary file of {{ formatSize(file.size) }}.</p>
    <p v-else-if="file.text === null" class="message muted">
      This file is too large to show ({{ formatSize(file.size) }}).
    </p>
    <p v-else-if="lines.length === 0" class="message muted">This file is empty.</p>
    <div v-else class="code-scroll">
      <div class="code">
        <pre class="line-numbers" aria-hidden="true">{{ lineNumbers }}</pre>
        <div class="text">
          <div v-for="(segments, index) in rows" :key="index" class="line"><span
            v-for="(segment, segmentIndex) in segments"
            :key="segmentIndex"
            :class="{ italic: segment.italic, bold: segment.bold }"
            :style="segment.color ? { color: segment.color } : undefined"
          >{{ segment.text }}</span></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-view {
  display: flex;
  flex-direction: column;
  height: 100%;
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

.message {
  margin: 0;
  padding: var(--space-8);
  text-align: center;
}

.error {
  color: var(--color-danger-bright);
}

.code-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--color-bg-sunken);
}

/* The numbers and the lines share one line height, so they stay level. */
.code {
  display: grid;
  grid-template-columns: max-content 1fr;
  min-width: max-content;
  min-height: 100%;
  font-size: var(--font-size-small);
  line-height: 1.55;
}

/* Stays in view while long lines scroll sideways. */
.line-numbers {
  position: sticky;
  left: 0;
  margin: 0;
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
  border-right: 1px solid var(--color-border-subtle);
  background: var(--color-bg-sunken);
  color: var(--color-fg-faint);
  font: inherit;
  text-align: right;
  user-select: none;
}

.text {
  padding: var(--space-2) var(--space-4);
  color: var(--color-syntax-variable);
  white-space: pre;
  tab-size: 4;
}

/* An empty line still takes its height. */
.line {
  min-height: 1lh;
}

.italic {
  font-style: italic;
}

.bold {
  font-weight: 600;
}
</style>
