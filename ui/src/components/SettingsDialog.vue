<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'

import type { FontSize } from '../api'
import { changeSettings, settings } from '../stores/settings'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'
import NumberField from './NumberField.vue'

const emit = defineEmits<{ close: [] }>()

const FONT_SIZES: { value: FontSize; label: string }[] = [
  { value: 'small', label: 'Small' },
  { value: 'normal', label: 'Normal' },
  { value: 'large', label: 'Large' },
  { value: 'extraLarge', label: 'Extra large' },
]

/** The most projects the Recent menu remembers; more would be hard to scan. */
const MAX_RECENT_LIMIT = 50

const dialog = ref<HTMLElement | null>(null)

// Every change applies and is saved at once, so there is nothing to confirm.
const fontSize = computed({
  get: () => settings.fontSize,
  set: (fontSize: FontSize) => changeSettings({ fontSize }),
})

const recentLimit = computed({
  get: () => settings.recentLimit,
  set: (recentLimit: number) => changeSettings({ recentLimit }),
})

useEscape(() => emit('close'))

// The chosen font size takes focus, so the arrow keys change it at once.
onMounted(async () => {
  await nextTick()
  dialog.value?.querySelector<HTMLElement>('input:checked')?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="dialog-layer" @mousedown.self="emit('close')">
      <section ref="dialog" class="dialog" role="dialog" aria-modal="true" aria-label="Settings">
        <h2 class="title">
          <AppIcon name="settings" large />
          Settings
        </h2>

        <fieldset class="group-box">
          <legend>Interface</legend>
          <div class="setting">
            <span id="font-size-label" class="setting-label">Font size</span>
            <div class="choices" role="radiogroup" aria-labelledby="font-size-label">
              <label v-for="option in FONT_SIZES" :key="option.value" class="choice">
                <input v-model="fontSize" type="radio" name="font-size" :value="option.value" />
                <span>{{ option.label }}</span>
              </label>
            </div>
          </div>
        </fieldset>

        <fieldset class="group-box">
          <legend>Misc</legend>
          <div class="setting">
            <span class="setting-label">Recent projects to remember</span>
            <NumberField v-model="recentLimit" :min="1" :max="MAX_RECENT_LIMIT" label="Recent projects to remember" />
          </div>
        </fieldset>

        <div class="actions">
          <button class="button" type="button" @click="emit('close')">Close</button>
        </div>
      </section>
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
  max-height: calc(100% - 2 * var(--space-6));
  padding: var(--space-6);
  overflow-y: auto;
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

/* The label on the left and its control on the right, or below it when the
   dialog is too narrow for both. */
.setting {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2) var(--space-4);
}

.choices {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1) var(--space-4);
}

.choice {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
}

.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
