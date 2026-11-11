<script setup lang="ts">
import { computed, ref } from 'vue'

import AppIcon from '../components/AppIcon.vue'
import FileVersionBar from '../components/FileVersionBar.vue'
import PageSearch from '../components/PageSearch.vue'
import { useProject } from '../stores/project'
import { useEscape } from '../utils/escape'
import { useFileAt } from '../utils/fileText'

const props = defineProps<{ commit: string; shortId: string; path: string; active: boolean }>()

const { api, goBack, openBlame, openFileHistory } = useProject()
const { lines, rows, notice } = useFileAt(api, props.commit, props.path)
const root = ref<HTMLElement | null>(null)

useEscape(goBack)

/** One number per line, as a single text, which lays out far faster than a node per line. */
const lineNumbers = computed(() => lines.value.map((_, index) => index + 1).join('\n'))
</script>

<template>
  <div ref="root" class="file-view">
    <FileVersionBar icon="file" :commit="commit" :short-id="shortId" :path="path">
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
        <button
          class="button"
          type="button"
          title="Show who last changed each line"
          @click="openBlame(commit, shortId, path)"
        >
          <AppIcon name="blame" />
          Blame
        </button>
      </template>
    </FileVersionBar>

    <PageSearch :root="root" :active="active" />

    <p v-if="notice" class="message" :class="notice.tone">{{ notice.text }}</p>
    <div v-else class="code-scroll">
      <div class="code">
        <pre class="line-numbers" aria-hidden="true">{{ lineNumbers }}</pre>
        <div class="text">
          <div v-for="(segments, index) in rows" :key="index" class="line" data-searchable><span
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
