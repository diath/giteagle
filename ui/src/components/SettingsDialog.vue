<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'

import type { FontSize, ThemeName } from '../api'
import { changeSettings, settings } from '../stores/settings'
import { FONT_SIZE_SCALES } from '../theme/scale'
import { useEscape } from '../utils/escape'
import AppIcon from './AppIcon.vue'
import NumberField from './NumberField.vue'
import SelectField from './SelectField.vue'

const emit = defineEmits<{ close: [] }>()

const THEMES: { value: ThemeName; label: string }[] = [
  { value: 'darkBlack', label: 'Dark (Black)' },
  { value: 'darkBrown', label: 'Dark (Brown)' },
  { value: 'lightBeige', label: 'Light (Beige)' },
  { value: 'lightWhite', label: 'Light (White)' },
]

/** Names a font size along with how large it draws the interface, e.g. "Large (125%)". */
function fontSizeOption(value: FontSize, name: string): { value: FontSize; label: string } {
  return { value, label: `${name} (${Math.round(FONT_SIZE_SCALES[value] * 100)}%)` }
}

const FONT_SIZES = [
  fontSizeOption('small', 'Small'),
  fontSizeOption('normal', 'Normal'),
  fontSizeOption('large', 'Large'),
  fontSizeOption('extraLarge', 'Extra large'),
]

/** The most projects the Recent menu remembers; more would be hard to scan. */
const MAX_RECENT_LIMIT = 50

const themeField = ref<{ focus: () => void } | null>(null)

// Every change applies and is saved at once, so there is nothing to confirm.
const theme = computed({
  get: () => settings.theme,
  set: (theme: ThemeName) => changeSettings({ theme }),
})

const fontSize = computed({
  get: () => settings.fontSize,
  set: (fontSize: FontSize) => changeSettings({ fontSize }),
})

const recentLimit = computed({
  get: () => settings.recentLimit,
  set: (recentLimit: number) => changeSettings({ recentLimit }),
})

useEscape(() => emit('close'))

// The first setting takes focus, so the keyboard reaches it at once.
onMounted(async () => {
  await nextTick()
  themeField.value?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="dialog-layer" @mousedown.self="emit('close')">
      <section class="dialog" role="dialog" aria-modal="true" aria-label="Settings">
        <h2 class="title">
          <AppIcon name="settings" large />
          Settings
        </h2>

        <fieldset class="group-box">
          <legend>Interface</legend>
          <div class="setting">
            <span class="setting-label">Theme</span>
            <SelectField ref="themeField" v-model="theme" class="select" :options="THEMES" label="Theme" />
          </div>
          <div class="setting">
            <span class="setting-label">Font size</span>
            <SelectField v-model="fontSize" class="select" :options="FONT_SIZES" label="Font size" />
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

/* Wide enough for the longest option, "Extra large (150%)", so each field
   keeps its size as its choice changes, and both line up. */
.select {
  width: calc(250 * var(--px));
}

.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
