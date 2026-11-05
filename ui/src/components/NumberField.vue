<script setup lang="ts">
import { ref, watch } from 'vue'

import AppIcon from './AppIcon.vue'

// A text field with buttons of its own rather than a number input, whose
// spin buttons look different on every platform and step on wheel scrolls.
const props = defineProps<{ min: number; max: number; label: string }>()

const value = defineModel<number>({ required: true })

/** What the field shows, which can be a number still being typed. */
const text = ref(String(value.value))

watch(value, (current) => {
  text.value = String(current)
})

function clamp(number: number): number {
  return Math.min(props.max, Math.max(props.min, number))
}

// A whole number in range applies as it is typed. Anything else waits, and
// leaving the field shows the value in effect again.
function onInput(event: Event): void {
  text.value = (event.target as HTMLInputElement).value
  const trimmed = text.value.trim()
  const typed = Number(trimmed)
  if (trimmed !== '' && Number.isInteger(typed) && typed >= props.min && typed <= props.max) {
    value.value = typed
  }
}

function step(by: number): void {
  value.value = clamp(value.value + by)
  text.value = String(value.value)
}

function onKeyDown(event: KeyboardEvent): void {
  if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
    event.preventDefault()
    step(event.key === 'ArrowUp' ? 1 : -1)
  }
}
</script>

<template>
  <div class="number-field">
    <button
      class="button button--icon"
      type="button"
      :disabled="value <= min"
      :aria-label="`Decrease ${label}`"
      title="Decrease"
      @click="step(-1)"
    >
      <AppIcon name="minus" />
    </button>
    <input
      class="text-input"
      type="text"
      inputmode="numeric"
      role="spinbutton"
      :value="text"
      :aria-label="label"
      :aria-valuemin="min"
      :aria-valuemax="max"
      :aria-valuenow="value"
      @input="onInput"
      @blur="text = String(value)"
      @keydown="onKeyDown"
    />
    <button
      class="button button--icon"
      type="button"
      :disabled="value >= max"
      :aria-label="`Increase ${label}`"
      title="Increase"
      @click="step(1)"
    >
      <AppIcon name="plus" />
    </button>
  </div>
</template>

<style scoped>
.number-field {
  display: inline-flex;
  gap: var(--space-1);
}

.text-input {
  width: 7ch;
  text-align: center;
}
</style>
