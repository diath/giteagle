<script setup lang="ts">
import { dismissToast, toasts } from '../stores/toasts'
import AppIcon from './AppIcon.vue'
</script>

<template>
  <div class="toast-host" aria-live="polite">
    <div v-for="toast in toasts" :key="toast.id" class="toast" :class="`toast--${toast.kind}`">
      <AppIcon :name="toast.kind === 'error' ? 'warning' : 'check'" class="icon" />
      <p class="message">{{ toast.message }}</p>
      <button class="icon-button" type="button" title="Dismiss" @click="dismissToast(toast.id)">
        <AppIcon name="close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  right: var(--space-4);
  bottom: var(--space-4);
  z-index: 50;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--space-2);
  max-width: min(700px, calc(100% - 2 * var(--space-4)));
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-large);
  background: var(--color-bg-raised);
  box-shadow: 0 5px 18px var(--color-shadow);
  pointer-events: auto;
}

.toast--error {
  border-color: var(--color-danger);
}

.toast--success .icon {
  color: var(--color-success);
}

.toast--info .icon {
  color: var(--color-info);
}

.toast--error .icon {
  color: var(--color-danger-bright);
}

.icon {
  margin-top: 4px;
}

.message {
  margin: 1px 0 0;
  max-height: 40vh;
  overflow-y: auto;
  font-size: var(--font-size-small);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}
</style>
