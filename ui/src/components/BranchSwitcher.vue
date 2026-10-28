<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'

import { api, errorMessage, type BranchList, type BranchRow } from '../api'
import { returnToMain } from '../stores/navigation'
import { capabilities, refreshEverything, repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { formatRelative } from '../utils/dates'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'
import DeleteBranchDialog from './DeleteBranchDialog.vue'
import SearchField from './SearchField.vue'

const emit = defineEmits<{ close: [] }>()

const branches = ref<BranchList | null>(null)
const error = ref<string | null>(null)
const filter = ref('')
const deleting = ref<string | null>(null)
const panel = ref<HTMLElement | null>(null)
const search = ref<InstanceType<typeof SearchField> | null>(null)

useEscape(() => {
  if (deleting.value === null) {
    emit('close')
  }
})

async function load(): Promise<void> {
  try {
    branches.value = await api.branches()
    error.value = null
  } catch (caught) {
    error.value = errorMessage(caught)
  }
}

function matches(row: BranchRow): boolean {
  const needle = filter.value.trim().toLowerCase()
  return needle === '' || row.name.toLowerCase().includes(needle)
}

const local = computed(() => branches.value?.local.filter(matches) ?? [])
const remote = computed(() => branches.value?.remote.filter(matches) ?? [])

async function select(row: BranchRow): Promise<void> {
  if (row.isCurrent) {
    emit('close')
    return
  }
  repo.busy = 'switch'
  try {
    const switchedTo = await api.switchBranch(row.name, row.isRemote)
    showToast('success', `Switched to ${switchedTo}.`)
    returnToMain()
    emit('close')
  } catch (caught) {
    showToast('error', errorMessage(caught))
  } finally {
    repo.busy = null
    await refreshEverything()
  }
}

async function deleted(): Promise<void> {
  deleting.value = null
  await Promise.all([load(), refreshEverything()])
}

/** Arrow keys move between rows, like in a menu. */
function moveFocus(event: KeyboardEvent): void {
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') {
    return
  }
  const rows = Array.from(panel.value?.querySelectorAll<HTMLElement>('.row-main') ?? [])
  if (rows.length === 0) {
    return
  }
  event.preventDefault()
  const current = rows.indexOf(document.activeElement as HTMLElement)
  const step = event.key === 'ArrowDown' ? 1 : -1
  const target = current === -1 ? (step === 1 ? 0 : rows.length - 1) : (current + step + rows.length) % rows.length
  rows[target].focus()
}

watch(() => repo.metadataRevision, load)

onMounted(async () => {
  await nextTick()
  search.value?.focus()
  await load()
})
</script>

<template>
  <div class="switcher-layer" @mousedown.self="emit('close')">
    <section ref="panel" class="switcher" aria-label="Switch branch" @keydown="moveFocus">
      <div class="filter">
        <SearchField ref="search" v-model="filter" placeholder="Filter branches…" />
      </div>

      <div class="list">
        <p v-if="error" class="message error">{{ error }}</p>
        <p v-else-if="!branches" class="message faint">Loading branches…</p>
        <template v-else>
          <h3 class="group">Local</h3>
          <p v-if="local.length === 0" class="message faint">No matching local branches</p>
          <div v-for="row in local" :key="`local-${row.name}`" class="row" :class="{ current: row.isCurrent }">
            <button class="row-main" type="button" :disabled="!capabilities.canSwitch && !row.isCurrent" @click="select(row)">
              <span class="marker">{{ row.isCurrent ? '●' : '' }}</span>
              <span class="name ellipsis">{{ row.name }}</span>
              <span class="hash">{{ row.tip.shortId }}</span>
              <span class="subject ellipsis">{{ row.tip.subject }}</span>
              <span v-if="row.upstream" class="upstream ellipsis">{{ row.upstream }}</span>
              <span v-else class="upstream faint">no upstream</span>
              <span class="counts">
                <template v-if="row.ahead !== null">
                  <span :class="row.ahead ? 'ahead' : 'faint'">↑{{ row.ahead }}</span>
                  <span :class="row.behind ? 'behind' : 'faint'">↓{{ row.behind }}</span>
                </template>
              </span>
              <span class="date faint">{{ formatRelative(row.tip.authorTime) }}</span>
            </button>
            <button
              v-if="!row.isCurrent && capabilities.canDelete"
              class="icon-button delete"
              type="button"
              :title="`Delete ${row.name}`"
              @click="deleting = row.name"
            >
              <AppIcon name="trash" />
            </button>
            <span v-else class="delete-placeholder"></span>
          </div>

          <h3 class="group">Remote</h3>
          <p v-if="remote.length === 0" class="message faint">No matching remote branches</p>
          <div v-for="row in remote" :key="`remote-${row.name}`" class="row">
            <button
              class="row-main"
              type="button"
              :disabled="!capabilities.canSwitch"
              :title="row.trackedBy ? `Switches to the local branch ${row.trackedBy}` : 'Creates a local branch tracking this one'"
              @click="select(row)"
            >
              <span class="marker"></span>
              <span class="name ellipsis">{{ row.name }}</span>
              <span class="hash">{{ row.tip.shortId }}</span>
              <span class="subject ellipsis">{{ row.tip.subject }}</span>
              <span class="upstream faint ellipsis">{{ row.trackedBy ? `→ ${row.trackedBy}` : '' }}</span>
              <span class="counts"></span>
              <span class="date faint">{{ formatRelative(row.tip.authorTime) }}</span>
            </button>
            <span class="delete-placeholder"></span>
          </div>
        </template>
      </div>
    </section>

    <DeleteBranchDialog v-if="deleting" :name="deleting" @close="deleting = null" @deleted="deleted" />
  </div>
</template>

<style scoped>
.switcher-layer {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: var(--color-scrim);
}

.switcher {
  position: absolute;
  top: calc(var(--bar-height) + var(--space-1));
  left: var(--space-4);
  display: flex;
  flex-direction: column;
  width: min(1500px, calc(100% - 2 * var(--space-4)));
  max-height: calc(100% - var(--bar-height) - var(--space-1) - var(--space-6));
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-large);
  background: var(--color-bg-panel);
  box-shadow: 0 9px 27px var(--color-shadow);
}

.filter {
  padding: var(--space-3);
  border-bottom: 1px solid var(--color-border);
}

.list {
  overflow-y: auto;
  padding-bottom: var(--space-2);
}

.group {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  padding: var(--space-3) var(--space-4) var(--space-1);
  color: var(--color-fg-faint);
  font-size: var(--font-size-tiny);
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.group::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--color-border);
}

.message {
  margin: 0;
  padding: var(--space-2) var(--space-4);
}

.error {
  color: var(--color-danger-bright);
}

.row {
  display: flex;
  align-items: center;
  padding-right: var(--space-2);
}

.row-main {
  display: grid;
  flex: 1;
  grid-template-columns: 1.5ch minmax(12ch, 1.2fr) 8ch minmax(10ch, 2fr) minmax(8ch, 1fr) 11ch 14ch;
  gap: var(--space-3);
  align-items: center;
  min-width: 0;
  padding: var(--space-1) var(--space-2) var(--space-1) var(--space-4);
  border: none;
  background: transparent;
  font-size: var(--font-size-small);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.row-main:hover:not(:disabled) {
  background: var(--color-bg-hover);
}

.row-main:disabled {
  cursor: default;
}

.row-main:focus-visible {
  outline-offset: -2px;
}

.marker {
  color: var(--color-accent);
}

.name {
  color: var(--color-fg-strong);
}

.current .name {
  color: var(--color-accent-bright);
  font-weight: 600;
}

.subject {
  color: var(--color-fg-muted);
}

.upstream {
  color: var(--color-fg-muted);
}

.counts {
  display: flex;
  gap: var(--space-2);
}

.ahead {
  color: var(--color-accent-bright);
}

.behind {
  color: var(--color-link);
}

.date {
  text-align: right;
}

.delete,
.delete-placeholder {
  flex: none;
  width: var(--icon-button-size);
}

.delete:hover:not(:disabled) {
  color: var(--color-danger-bright);
}
</style>
