<script setup lang="ts">
import type { IconName } from '../icons/paths'
import AppIcon from './AppIcon.vue'

defineProps<{ icon: IconName }>()

defineSlots<{
  /** What the prompt asks. */
  default: () => unknown
  /** The buttons that answer it. */
  actions: () => unknown
}>()
</script>

<template>
  <!-- Floats over the bottom of the view, which stays usable around it. -->
  <div class="floating-prompt" role="status">
    <AppIcon :name="icon" class="icon" />
    <span>
      <slot />
    </span>
    <slot name="actions" />
  </div>
</template>

<style scoped>
.floating-prompt {
  position: fixed;
  bottom: var(--space-4);
  left: 50%;
  z-index: 30;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
  border: 1px solid var(--color-accent);
  border-radius: var(--radius-large);
  background: var(--color-bg-raised);
  color: var(--color-fg-strong);
  box-shadow: 0 calc(5 * var(--px)) calc(18 * var(--px)) var(--color-shadow);
  transform: translateX(-50%);
  white-space: nowrap;
}

.icon {
  color: var(--color-accent-bright);
}
</style>
