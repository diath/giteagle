<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'

import { activateTab, canCloseTab, closeTab, openNewTab, tabs, type Tab } from '../stores/tabs'
import { splitBeforeLast } from '../utils/paths'
import AppIcon from './AppIcon.vue'
import RecentMenu from './RecentMenu.vue'

/** The middle button, which closes a tab as in a browser. */
const MIDDLE_BUTTON = 1

const strip = ref<HTMLElement | null>(null)
const recentButton = ref<HTMLButtonElement | null>(null)
const recentOpen = ref(false)

function tabName(tab: Tab): string {
  return tab.project?.summary.name ?? 'New tab'
}

// Ctrl+W closes only the shown tab, so only its button names the shortcut.
function closeTitle(tab: Tab): string {
  const title = `Close ${tabName(tab)}`
  return tab.key === tabs.activeKey ? `${title} (Ctrl+W)` : title
}

function closeRecent(): void {
  recentOpen.value = false
  recentButton.value?.focus()
}

function onAuxClick(event: MouseEvent, tab: Tab): void {
  if (event.button === MIDDLE_BUTTON && canCloseTab(tab)) {
    closeTab(tab.key)
  }
}

// The scrollbar is hidden to keep the bar slim, so the wheel scrolls
// sideways through tabs that don't fit.
function onWheel(event: WheelEvent): void {
  const element = strip.value
  if (element && event.deltaY !== 0 && element.scrollWidth > element.clientWidth) {
    element.scrollLeft += event.deltaY
    event.preventDefault()
  }
}

watch(
  () => tabs.activeKey,
  async () => {
    await nextTick()
    strip.value?.querySelector('.tab.active')?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  },
)
</script>

<template>
  <header class="project-tabs">
    <div class="recent">
      <button
        ref="recentButton"
        class="button"
        type="button"
        aria-haspopup="menu"
        :aria-expanded="recentOpen"
        title="Recently opened projects"
        @click="recentOpen = !recentOpen"
      >
        <AppIcon name="history" />
        Recent
        <AppIcon name="chevronDown" />
      </button>
      <RecentMenu v-if="recentOpen" @close="closeRecent" />
    </div>

    <div ref="strip" class="strip" role="tablist" aria-label="Projects" @wheel="onWheel">
      <div
        v-for="tab in tabs.list"
        :key="tab.key"
        class="tab"
        :class="{ active: tab.key === tabs.activeKey }"
        @mousedown.middle.prevent
        @auxclick="onAuxClick($event, tab)"
      >
        <button
          class="tab-select"
          type="button"
          role="tab"
          :aria-selected="tab.key === tabs.activeKey"
          :title="tab.project ? tab.project.summary.workdir : 'Open a project in this tab'"
          @click="activateTab(tab.key)"
        >
          <AppIcon :name="tab.project ? 'folder' : 'plus'" class="tab-icon" />
          <span class="text">
            <span class="name ellipsis">{{ tabName(tab) }}</span>
            <span v-if="tab.project" class="path">
              <span class="path-start ellipsis">{{ splitBeforeLast(tab.project.summary.workdir).start }}</span>
              <span class="path-end ellipsis">{{ splitBeforeLast(tab.project.summary.workdir).end }}</span>
            </span>
            <span v-else class="path">No project open</span>
          </span>
        </button>
        <button
          v-if="canCloseTab(tab)"
          class="icon-button tab-close"
          type="button"
          :title="closeTitle(tab)"
          @click="closeTab(tab.key)"
        >
          <AppIcon name="close" />
        </button>
      </div>
    </div>

    <button
      class="button button--icon new-tab"
      type="button"
      title="Open a project in a new tab (Ctrl+T)"
      @click="openNewTab"
    >
      <AppIcon name="plus" />
    </button>
  </header>
</template>

<style scoped>
/* Editor-style tabs. The shown one takes the color of the bar under it and
   covers the line between them, so the two read as one surface. */
.project-tabs {
  display: flex;
  flex: none;
  align-items: flex-end;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2) 0;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-sunken);
}

/* Reaches over the bar's bottom line, which a scrolling box would otherwise
   clip the shown tab short of. */
/* The menu hangs from the button. */
.recent {
  position: relative;
  flex: none;
  align-self: center;
}

.strip {
  display: flex;
  gap: calc(2 * var(--px));
  min-width: 0;
  margin-bottom: -1px;
  overflow-x: auto;
  scrollbar-width: none;
}

.strip::-webkit-scrollbar {
  display: none;
}

.tab {
  position: relative;
  display: flex;
  flex: 0 1 calc(320 * var(--px));
  align-items: center;
  min-width: calc(165 * var(--px));
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: var(--radius) var(--radius) 0 0;
  transition: background-color var(--transition);
}

.tab:hover {
  background: var(--color-bg-app);
}

.tab.active {
  border-color: var(--color-border);
  background: var(--color-bg-panel);
}

.tab.active::before {
  content: '';
  position: absolute;
  top: -1px;
  right: -1px;
  left: -1px;
  height: calc(2 * var(--px));
  border-radius: var(--radius) var(--radius) 0 0;
  background: var(--color-accent);
}

.tab-select {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
  border: none;
  border-radius: var(--radius) 0 0 0;
  background: transparent;
  color: var(--color-fg-muted);
  text-align: left;
  cursor: pointer;
}

.tab-select:focus-visible {
  outline-offset: -2px;
}

.tab-icon {
  color: var(--color-fg-faint);
}

.tab.active .tab-icon {
  color: var(--color-accent);
}

/* The path gets a line of its own, so it has the tab's full width. */
.text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.2;
}

.name {
  font-weight: 600;
}

.tab.active .name {
  color: var(--color-fg-strong);
}

.path {
  display: flex;
  min-width: 0;
  color: var(--color-fg-faint);
  font-size: calc(12 * var(--px));
}

/* Only the start shrinks, and the folder name gives way only once the start
   is gone, so a long path is cut in the middle and both of its ends stay. */
.path-start {
  min-width: 0;
}

.path-end {
  flex: none;
  max-width: 100%;
}

.tab-close {
  flex: none;
  margin-right: var(--space-1);
}

.new-tab {
  flex: none;
  align-self: center;
}
</style>
