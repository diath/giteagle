<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'

import { errorMessage } from '../api'
import { changeSettings, settings } from '../stores/settings'
import { openProject } from '../stores/tabs'
import { showToast } from '../stores/toasts'
import { useEscape } from '../utils/escape'
import { folderName, splitBeforeLast } from '../utils/paths'
import AppIcon from './AppIcon.vue'

const emit = defineEmits<{ close: [] }>()

const menu = ref<HTMLElement | null>(null)

// The setting can be lowered after the list grew; the next project opened
// shortens the list itself.
const recent = computed(() => settings.recentProjects.slice(0, settings.recentLimit))

useEscape(() => emit('close'))

function enabledItems(): HTMLElement[] {
  return Array.from(menu.value?.querySelectorAll<HTMLElement>('.item:not(:disabled)') ?? [])
}

function moveFocus(event: KeyboardEvent): void {
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') {
    return
  }
  const items = enabledItems()
  if (items.length === 0) {
    return
  }
  event.preventDefault()
  const current = items.indexOf(document.activeElement as HTMLElement)
  const step = event.key === 'ArrowDown' ? 1 : -1
  const target = current === -1 ? (step === 1 ? 0 : items.length - 1) : (current + step + items.length) % items.length
  items[target].focus()
}

// A folder that can't be opened any more stays out of the menu, so it
// doesn't fail the same way again.
async function open(folder: string): Promise<void> {
  emit('close')
  try {
    await openProject(folder)
  } catch (error) {
    changeSettings({ recentProjects: settings.recentProjects.filter((recentFolder) => recentFolder !== folder) })
    showToast(
      'error',
      `Couldn't open ${folderName(folder)}: ${errorMessage(error)} It was removed from the recent projects.`,
    )
  }
}

function clear(): void {
  changeSettings({ recentProjects: [] })
  emit('close')
}

// The first project takes focus, so the arrow keys move through the menu
// straight away.
onMounted(async () => {
  await nextTick()
  enabledItems()[0]?.focus()
})
</script>

<template>
  <!-- Catches clicks outside the menu, which close it. -->
  <div class="menu-layer" @mousedown="emit('close')"></div>
  <div ref="menu" class="menu" role="menu" aria-label="Recent projects" @keydown="moveFocus">
    <div class="items">
      <p v-if="recent.length === 0" class="empty">No recent projects</p>
      <button
        v-for="folder in recent"
        :key="folder"
        class="item"
        type="button"
        role="menuitem"
        :title="folder"
        @click="open(folder)"
      >
        <AppIcon name="folder" class="item-icon" />
        <span class="text">
          <span class="name ellipsis">{{ folderName(folder) }}</span>
          <span class="path">
            <span class="path-start ellipsis">{{ splitBeforeLast(folder).start }}</span>
            <span class="path-end ellipsis">{{ splitBeforeLast(folder).end }}</span>
          </span>
        </span>
      </button>
    </div>
    <div class="separator" role="separator"></div>
    <button class="item" type="button" role="menuitem" :disabled="recent.length === 0" @click="clear">
      <AppIcon name="trash" class="item-icon" />
      Clear recents
    </button>
  </div>
</template>

<style scoped>
.menu-layer {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.menu {
  position: absolute;
  top: calc(100% + var(--space-1));
  left: 0;
  z-index: 31;
  display: flex;
  flex-direction: column;
  width: calc(420 * var(--px));
  max-width: calc(100vw - 2 * var(--space-2));
  padding: var(--space-1) 0;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius);
  background: var(--color-bg-panel);
  box-shadow: 0 calc(8 * var(--px)) calc(23 * var(--px)) var(--color-shadow);
}

/* Only the projects scroll; clearing them stays in reach. */
.items {
  display: flex;
  flex-direction: column;
  max-height: 60vh;
  overflow-y: auto;
}

.empty {
  margin: 0;
  padding: var(--space-1) var(--space-3);
  color: var(--color-fg-faint);
}

.item {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-1) var(--space-3);
  border: none;
  background: transparent;
  color: var(--color-fg);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.item:hover:not(:disabled) {
  background: var(--color-bg-hover);
}

.item:disabled {
  color: var(--color-fg-disabled);
  cursor: default;
}

.item:focus-visible {
  outline-offset: -2px;
}

.item-icon {
  color: var(--color-fg-faint);
}

.text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.2;
}

.name {
  color: var(--color-fg-strong);
  font-weight: 600;
}

.path {
  display: flex;
  min-width: 0;
  color: var(--color-fg-faint);
  font-size: calc(12 * var(--px));
}

/* As in the tabs, a long path is cut in the middle and both ends stay. */
.path-start {
  min-width: 0;
}

.path-end {
  flex: none;
  max-width: 100%;
}

.separator {
  height: 1px;
  margin: var(--space-1) 0;
  background: var(--color-border);
}
</style>
