<script setup lang="ts">
import { ref } from 'vue'

import AppIcon from './AppIcon.vue'

defineProps<{ placeholder: string }>()
const model = defineModel<string>({ required: true })
const input = ref<HTMLInputElement | null>(null)

function clear(): void {
  model.value = ''
  input.value?.focus()
}

// Escape first clears the search; on an empty field it falls through to
// whatever layer handles Escape, e.g. closing the branch switcher.
function onEscape(event: KeyboardEvent): void {
  if (model.value) {
    event.preventDefault()
    event.stopPropagation()
    clear()
  }
}

defineExpose({ focus: () => input.value?.focus() })
</script>

<template>
  <div class="search-field">
    <AppIcon name="search" class="search-icon" />
    <input
      ref="input"
      v-model="model"
      class="text-input"
      type="text"
      spellcheck="false"
      autocomplete="off"
      :placeholder="placeholder"
      @keydown.esc="onEscape"
    />
    <button v-if="model" class="icon-button clear" type="button" title="Clear search" @click="clear">
      <AppIcon name="close" />
    </button>
  </div>
</template>

<style scoped>
.search-field {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: var(--space-2);
  color: var(--color-fg-faint);
  pointer-events: none;
}

.text-input {
  padding-left: calc(var(--space-2) * 2 + var(--icon-size));
  padding-right: calc(var(--space-2) + var(--icon-button-size));
}

.clear {
  position: absolute;
  right: calc(3 * var(--px));
}
</style>
