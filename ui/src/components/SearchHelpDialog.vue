<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'

import { api, errorMessage, type SearchTag } from '../api'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'

const emit = defineEmits<{ close: [] }>()

const tags = ref<SearchTag[] | null>(null)
const loadError = ref<string | null>(null)
const closeButton = ref<HTMLButtonElement | null>(null)

useEscape(() => emit('close'))

onMounted(async () => {
  await nextTick()
  closeButton.value?.focus()
  try {
    tags.value = await api.searchTags()
  } catch (error) {
    loadError.value = errorMessage(error)
  }
})
</script>

<template>
  <div class="dialog-layer" @mousedown.self="emit('close')">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Search tags">
      <h2 class="title">
        <AppIcon name="help" large />
        Search tags
      </h2>

      <p class="muted">
        Words without a tag match the message or author names. A commit must match every part of the search,
        ignoring case. Quote values that contain spaces.
      </p>

      <p v-if="loadError" class="warning">
        <AppIcon name="warning" />
        <span>{{ loadError }}</span>
      </p>
      <div v-else-if="tags" class="table-scroll">
        <table class="tags">
          <thead>
            <tr>
              <th>Tag</th>
              <th>Usage</th>
              <th>Example</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="tag in tags" :key="tag.name">
              <td class="code-cell"><code class="tag-name">{{ tag.name }}:</code></td>
              <td>{{ tag.description }}</td>
              <td class="code-cell"><code>{{ tag.example }}</code></td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="actions">
        <button ref="closeButton" class="button" type="button" @click="emit('close')">Close</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-layer {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-scrim);
}

.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  width: min(calc(900 * var(--px)), calc(100% - 2 * var(--space-6)));
  max-height: calc(100% - 2 * var(--space-6));
  padding: var(--space-6);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-large);
  background: var(--color-bg-panel);
  box-shadow: 0 calc(10 * var(--px)) calc(30 * var(--px)) var(--color-shadow);
}

.title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  color: var(--color-fg-strong);
  font-size: var(--font-size-large);
  font-weight: 600;
}

p {
  margin: 0;
}

.warning {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius);
  background: var(--color-danger-muted);
  color: var(--color-danger-bright);
}

.table-scroll {
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.tags {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-small);
}

.tags th {
  padding: var(--space-1) var(--space-3);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-raised);
  color: var(--color-fg-faint);
  font-weight: normal;
  text-align: left;
}

.tags td {
  padding: var(--space-2) var(--space-3);
  vertical-align: baseline;
}

.tags tbody tr + tr td {
  border-top: 1px solid var(--color-border-subtle);
}

.code-cell {
  white-space: nowrap;
}

code {
  padding: 0 var(--space-1);
  border-radius: var(--radius);
  background: var(--color-bg-active);
  color: var(--color-fg-strong);
  font: inherit;
}

.tag-name {
  color: var(--color-accent-bright);
}

.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
