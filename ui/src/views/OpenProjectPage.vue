<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'

import { appApi, errorMessage } from '../api'
import AppIcon from '../components/AppIcon.vue'
import { openProjectInTab } from '../stores/tabs'

const props = defineProps<{
  /** The tab the project opens in. */
  tabKey: number
  /** Whether the page's tab is the one shown. */
  active: boolean
}>()

const path = ref('')
const error = ref<string | null>(null)
const opening = ref(false)
const browsing = ref(false)
const input = ref<HTMLInputElement | null>(null)
const loadButton = ref<HTMLButtonElement | null>(null)

async function browse(): Promise<void> {
  browsing.value = true
  try {
    const picked = await appApi.pickFolder(path.value.trim() || null)
    if (picked !== null) {
      path.value = picked
      error.value = null
      await nextTick()
      loadButton.value?.focus()
    }
  } catch (caught) {
    error.value = errorMessage(caught)
  } finally {
    browsing.value = false
  }
}

// On success the tab shows the project instead, and this page goes away.
async function load(): Promise<void> {
  if (path.value.trim() === '' || opening.value) {
    return
  }
  opening.value = true
  error.value = null
  try {
    await openProjectInTab(props.tabKey, path.value)
  } catch (caught) {
    error.value = errorMessage(caught)
    opening.value = false
  }
}

function focusPath(): void {
  input.value?.focus()
}

onMounted(() => {
  if (props.active) {
    focusPath()
  }
})

watch(
  () => props.active,
  async (active) => {
    if (active) {
      await nextTick()
      focusPath()
    }
  },
)
</script>

<template>
  <div class="open-page">
    <form class="open-form" @submit.prevent="load">
      <fieldset class="group-box" :disabled="opening">
        <legend>
          <AppIcon name="folder" />
          Open a project
        </legend>

        <p class="hint">Choose the folder of a Git repository, or any folder inside one.</p>

        <div class="path-row">
          <input
            ref="input"
            v-model="path"
            class="text-input"
            type="text"
            spellcheck="false"
            autocomplete="off"
            placeholder="/path/to/project"
            aria-label="Project folder"
            @input="error = null"
          />
          <button class="button" type="button" :disabled="browsing" @click="browse">
            <AppIcon name="folder" />
            Browse…
          </button>
        </div>

        <p v-if="error" class="error" role="alert">
          <span class="error-icon"><AppIcon name="warning" /></span>
          <span>{{ error }}</span>
        </p>

        <div class="actions">
          <button ref="loadButton" class="button" type="submit" :disabled="path.trim() === '' || opening">
            {{ opening ? 'Loading…' : 'Load project' }}
          </button>
        </div>
      </fieldset>
    </form>
  </div>
</template>

<style scoped>
.open-page {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: var(--space-6);
  overflow-y: auto;
  background: var(--color-bg-app);
}

.open-form {
  width: min(calc(700 * var(--px)), 100%);
  /* Rests a little above the middle, where the eye starts. */
  margin-bottom: 10vh;
}

legend > :first-child {
  color: var(--color-accent);
}

.hint {
  margin: 0;
  color: var(--color-fg-muted);
  font-size: var(--font-size-small);
}

.path-row {
  display: flex;
  gap: var(--space-2);
}

.path-row .text-input {
  flex: 1;
  min-width: 0;
}

.path-row .button {
  flex: none;
}

.error {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  margin: 0;
  color: var(--color-danger-bright);
  font-size: var(--font-size-small);
  overflow-wrap: anywhere;
}

/* As tall as a line, so the icon centers on the first one. */
.error-icon {
  display: flex;
  flex: none;
  align-items: center;
  height: 1lh;
}

.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
