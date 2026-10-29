<script setup lang="ts">
import { ref } from 'vue'

withDefaults(defineProps<{ expected: string; disabled?: boolean }>(), { disabled: false })

const typed = defineModel<string>({ required: true })
const input = ref<HTMLInputElement | null>(null)

defineExpose({ focus: () => input.value?.focus() })
</script>

<template>
  <!-- For what can't be undone: the action waits until `expected` is typed. -->
  <label class="type-to-confirm">
    <span class="muted">
      Type <code>{{ expected }}</code> to confirm
    </span>
    <input
      ref="input"
      v-model="typed"
      class="text-input"
      type="text"
      spellcheck="false"
      autocomplete="off"
      :disabled="disabled"
    />
  </label>
</template>

<style scoped>
.type-to-confirm {
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
</style>
