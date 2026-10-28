<script setup lang="ts">
import { ref } from 'vue'

import BranchBar from '../components/BranchBar.vue'
import BranchInfo from '../components/BranchInfo.vue'
import BranchSwitcher from '../components/BranchSwitcher.vue'
import CommitsTab from '../components/CommitsTab.vue'
import FilesTab from '../components/FilesTab.vue'
import { navigation, openCommit, type MainTab } from '../stores/navigation'

defineProps<{ active: boolean }>()

const switcherOpen = ref(false)

const TABS: { id: MainTab; label: string }[] = [
  { id: 'commits', label: 'Commits' },
  { id: 'files', label: 'Files' },
]
</script>

<template>
  <div class="main-view">
    <BranchBar @open-switcher="switcherOpen = true" />
    <BranchInfo @open-commit="openCommit" />

    <nav class="tabs" role="tablist">
      <button
        v-for="tab in TABS"
        :key="tab.id"
        class="tab"
        type="button"
        role="tab"
        :aria-selected="navigation.tab === tab.id"
        :class="{ selected: navigation.tab === tab.id }"
        @click="navigation.tab = tab.id"
      >
        {{ tab.label }}
      </button>
    </nav>

    <div class="tab-panes">
      <!-- Both tabs stay mounted and only hide, so each keeps its scroll
           position and loaded data while the other is shown. -->
      <CommitsTab class="pane" :class="{ hidden: navigation.tab !== 'commits' }" :active="active && navigation.tab === 'commits'" />
      <FilesTab class="pane" :class="{ hidden: navigation.tab !== 'files' }" />
    </div>

    <BranchSwitcher v-if="switcherOpen" @close="switcherOpen = false" />
  </div>
</template>

<style scoped>
.main-view {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.tabs {
  display: flex;
  gap: var(--space-1);
  padding: 0 var(--space-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.tab {
  position: relative;
  padding: var(--space-2) var(--space-3);
  border: none;
  background: transparent;
  color: var(--color-fg-muted);
  cursor: pointer;
  transition: color var(--transition);
}

.tab:hover {
  color: var(--color-fg);
}

.tab.selected {
  color: var(--color-fg-strong);
  font-weight: 600;
}

.tab.selected::after {
  content: '';
  position: absolute;
  right: var(--space-2);
  bottom: -1px;
  left: var(--space-2);
  height: 3px;
  background: var(--color-accent);
}

.tab:focus-visible {
  outline-offset: -2px;
}

.tab-panes {
  position: relative;
  flex: 1;
  min-height: 0;
}

.pane {
  position: absolute;
  inset: 0;
}

.pane.hidden {
  visibility: hidden;
  pointer-events: none;
}
</style>
