<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'

import { api, errorMessage, type DeleteInfo } from '../api'
import { repo } from '../stores/repo'
import { showToast } from '../stores/toasts'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ name: string }>()
const emit = defineEmits<{ close: []; deleted: [] }>()

const info = ref<DeleteInfo | null>(null)
const loadError = ref<string | null>(null)
const alsoRemote = ref(false)
const confirmation = ref('')
const working = ref(false)
const input = ref<HTMLInputElement | null>(null)

const expected = computed(() => `delete ${props.name}`)
const confirmed = computed(() => confirmation.value === expected.value)
const lostCommits = computed(() => {
  if (!info.value) {
    return 0
  }
  return alsoRemote.value ? info.value.lostCommitsWithRemote : info.value.lostCommits
})

useEscape(() => {
  if (!working.value) {
    emit('close')
  }
})

async function confirm(): Promise<void> {
  if (!confirmed.value || working.value) {
    return
  }
  working.value = true
  repo.busy = 'delete'
  try {
    await api.deleteBranch(props.name, alsoRemote.value)
    showToast('success', alsoRemote.value ? `Deleted ${props.name} here and on the remote.` : `Deleted ${props.name}.`)
    emit('deleted')
  } catch (error) {
    showToast('error', errorMessage(error))
  } finally {
    working.value = false
    repo.busy = null
  }
}

onMounted(async () => {
  await nextTick()
  input.value?.focus()
  try {
    info.value = await api.deleteInfo(props.name)
  } catch (error) {
    loadError.value = errorMessage(error)
  }
})
</script>

<template>
  <div class="dialog-layer" @mousedown.self="!working && emit('close')">
    <form class="dialog" role="dialog" aria-modal="true" :aria-label="`Delete ${name}`" @submit.prevent="confirm">
      <h2 class="title">
        <AppIcon name="trash" large />
        Delete branch <span class="branch">{{ name }}</span>
      </h2>

      <p v-if="loadError" class="warning">{{ loadError }}</p>
      <p v-else-if="lostCommits > 0" class="warning">
        <AppIcon name="warning" />
        <span>
          {{ lostCommits }} {{ lostCommits === 1 ? 'commit is' : 'commits are' }} not on any other branch
          and will be lost.
        </span>
      </p>

      <label v-if="info?.remoteBranch" class="checkbox">
        <input v-model="alsoRemote" type="checkbox" :disabled="working" />
        <span>Also delete <strong>{{ info.remoteBranch.displayName }}</strong> on the remote</span>
      </label>

      <label class="confirm">
        <span class="muted">
          Type <code>{{ expected }}</code> to confirm
        </span>
        <input
          ref="input"
          v-model="confirmation"
          class="text-input"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :disabled="working"
        />
      </label>

      <div class="actions">
        <button class="button" type="button" :disabled="working" @click="emit('close')">Cancel</button>
        <button class="button button--danger" type="submit" :disabled="!confirmed || working">
          {{ working ? 'Deleting…' : 'Delete' }}
        </button>
      </div>
    </form>
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
  width: min(calc(650 * var(--px)), calc(100% - 2 * var(--space-6)));
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

.branch {
  color: var(--color-danger-bright);
}

.warning {
  display: flex;
  gap: var(--space-2);
  margin: 0;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-danger);
  border-radius: var(--radius);
  background: var(--color-danger-muted);
  color: var(--color-danger-bright);
  white-space: pre-wrap;
}

.checkbox {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
}

.checkbox input {
  accent-color: var(--color-danger);
}

.confirm {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

code {
  padding: 0 var(--space-1);
  border-radius: var(--radius);
  background: var(--color-bg-active);
  color: var(--color-fg-strong);
  font: inherit;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}
</style>
