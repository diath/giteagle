<script setup lang="ts">
import { ICON_PATHS, type IconName } from '../icons/paths'

withDefaults(defineProps<{ name: IconName; size?: 16 | 20 }>(), { size: 16 })
</script>

<template>
  <svg
    class="app-icon"
    :width="size"
    :height="size"
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
}
</style>
