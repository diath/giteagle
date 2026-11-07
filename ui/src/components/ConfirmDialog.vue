<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'

import type { IconName } from '../icons/paths'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'

const props = withDefaults(
  defineProps<{
    title: string
    icon: IconName
    confirmLabel: string
    /** Replaces the confirm label while `working`. */
    workingLabel: string
    working: boolean
    /** Shown in a red box, for what can't be undone. */
    warning?: string | null
    danger?: boolean
    disabled?: boolean
    /** Room for a commit message's 72 columns. */
    wide?: boolean
    /** Room for a diff, as much as the window has. */
    large?: boolean
  }>(),
  { warning: null, danger: false, disabled: false, wide: false, large: false },
)

const emit = defineEmits<{ confirm: []; close: [] }>()

defineSlots<{
  default?: () => unknown
  /** Comes after the warning, e.g. a field to type a confirmation into. */
  confirmation?: () => unknown
  /** Buttons in place of Cancel and the confirm button. */
  actions?: () => unknown
}>()

const form = ref<HTMLFormElement | null>(null)

useEscape(() => {
  if (!props.working) {
    emit('close')
  }
})

function confirm(): void {
  if (!props.working && !props.disabled) {
    emit('confirm')
  }
}

// The first field gets focus, or else the confirm button, so Enter confirms.
onMounted(async () => {
  await nextTick()
  form.value?.querySelector<HTMLElement>('input, button[type="submit"]')?.focus()
})
</script>

<template>
  <!-- Moved to the body, so clicks in the dialog don't reach a clickable
       row it was opened from. -->
  <Teleport to="body">
    <div class="dialog-layer" @mousedown.self="!working && emit('close')">
      <form
        ref="form"
        class="dialog"
        :class="{ 'dialog--wide': wide, 'dialog--large': large }"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        @submit.prevent="confirm"
      >
        <h2 class="title">
          <AppIcon :name="icon" large />
          {{ title }}
        </h2>

        <slot />

        <p v-if="warning" class="warning">
          <AppIcon name="warning" />
          <span>{{ warning }}</span>
        </p>

        <slot name="confirmation" />

        <div class="actions">
          <slot name="actions">
            <button class="button" type="button" :disabled="working" @click="emit('close')">Cancel</button>
            <button class="button" :class="{ 'button--danger': danger }" type="submit" :disabled="disabled || working">
              {{ working ? workingLabel : confirmLabel }}
            </button>
          </slot>
        </div>
      </form>
    </div>
  </Teleport>
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

.dialog--wide {
  width: min(calc(800 * var(--px)), calc(100% - 2 * var(--space-6)));
}

.dialog--large {
  width: min(calc(1200 * var(--px)), calc(100% - 2 * var(--space-6)));
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

.dialog :slotted(p) {
  margin: 0;
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
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}
</style>
