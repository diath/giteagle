<script setup lang="ts">
import { ICON_PATHS, type IconName } from '../icons/paths'

defineProps<{ name: IconName; large?: boolean }>()
</script>

<template>
  <svg
    class="app-icon"
    :class="{ large }"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.4"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="ICON_PATHS[name]" />
  </svg>
</template>

<style scoped>
.app-icon {
  flex: none;
  width: var(--icon-size);
  height: var(--icon-size);
}

.app-icon.large {
  width: var(--icon-size-large);
  height: var(--icon-size-large);
}
</style>
