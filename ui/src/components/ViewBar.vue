<script setup lang="ts">
import type { IconName } from '../icons/paths'
import { goBack } from '../stores/navigation'
import AppIcon from './AppIcon.vue'

withDefaults(
  defineProps<{
    /** Shown before the title, in the frame's accent color. */
    icon?: IconName
    backTitle?: string
  }>(),
  { icon: undefined, backTitle: 'Back (Esc)' },
)

defineSlots<{
  /** Inside the frame in the middle of the bar. */
  title: () => unknown
  /** Beside the frame, on its left. */
  'before-title'?: () => unknown
  /** Beside the frame, on its right. */
  'after-title'?: () => unknown
  /** At the right end of the bar. */
  end?: () => unknown
}>()
</script>

<template>
  <div class="view-bar">
    <div class="column">
      <button class="button" type="button" :title="backTitle" @click="goBack">
        <AppIcon name="back" />
        Back
      </button>
    </div>

    <div class="column column--center">
      <slot name="before-title" />
      <div class="inset title">
        <AppIcon v-if="icon" :name="icon" />
        <span class="label">
          <slot name="title" />
        </span>
      </div>
      <slot name="after-title" />
    </div>

    <div class="column column--end">
      <slot name="end" />
    </div>
  </div>
</template>

<style scoped>
/* Laid out like the branch bar: the middle column takes what it needs and
   stays centered in the window, and the outer columns always keep a share. */
.view-bar {
  display: grid;
  grid-template-columns: minmax(15%, 1fr) minmax(0, max-content) minmax(15%, 1fr);
  align-items: center;
  gap: var(--space-4);
  min-height: var(--bar-height);
  padding: var(--space-1) var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.column {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.column--center > :not(.title) {
  flex: none;
}

.column--end {
  justify-content: flex-end;
}

.title {
  height: var(--control-height);
}

/* A long title, e.g. a deep path, is cut short by the parts it marks as
   `ellipsis`. */
.label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  color: var(--color-fg-strong);
  font-weight: 600;
  white-space: nowrap;
}
</style>
