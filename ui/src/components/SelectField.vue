<script setup lang="ts" generic="T extends string">
import { computed, nextTick, ref } from 'vue'

import AppIcon from './AppIcon.vue'

// Drawn here rather than as a <select>, whose list some platforms draw
// themselves, in the system font and colors.
const props = defineProps<{ options: { value: T; label: string }[]; label: string }>()

const value = defineModel<T>({ required: true })

const open = ref(false)
const root = ref<HTMLElement | null>(null)
const button = ref<HTMLButtonElement | null>(null)
const list = ref<HTMLElement | null>(null)

const chosenLabel = computed(() => props.options.find((option) => option.value === value.value)?.label ?? '')

function optionButtons(): HTMLElement[] {
  return Array.from(list.value?.querySelectorAll<HTMLElement>('.option') ?? [])
}

// The chosen option takes focus, so the arrow keys move on from it.
async function show(): Promise<void> {
  open.value = true
  await nextTick()
  const options = optionButtons()
  const chosen = props.options.findIndex((option) => option.value === value.value)
  options[Math.max(chosen, 0)]?.focus()
}

function hide(): void {
  open.value = false
  button.value?.focus()
}

function choose(option: T): void {
  value.value = option
  hide()
}

function onButtonKeyDown(event: KeyboardEvent): void {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    void show()
  }
}

function onListKeyDown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    // Handled here, Escape no longer reaches the dialog the field is in.
    event.preventDefault()
    hide()
    return
  }
  const options = optionButtons()
  const current = options.indexOf(document.activeElement as HTMLElement)
  const targets: Record<string, number> = {
    ArrowDown: Math.min(current + 1, options.length - 1),
    ArrowUp: Math.max(current - 1, 0),
    Home: 0,
    End: options.length - 1,
  }
  if (event.key in targets) {
    event.preventDefault()
    options[targets[event.key]]?.focus()
  }
}

// Closes the list once focus leaves the field, e.g. on a click elsewhere or
// on Tab.
function onFocusOut(event: FocusEvent): void {
  if (!root.value?.contains(event.relatedTarget as Node | null)) {
    open.value = false
  }
}

defineExpose({ focus: () => button.value?.focus() })
</script>

<template>
  <div ref="root" class="select-field" @focusout="onFocusOut">
    <button
      ref="button"
      class="button field-button"
      type="button"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-label="`${label}: ${chosenLabel}`"
      @click="open ? hide() : show()"
      @keydown="onButtonKeyDown"
    >
      <span class="ellipsis">{{ chosenLabel }}</span>
      <AppIcon name="chevronDown" class="chevron" />
    </button>
    <div v-if="open" ref="list" class="list" role="listbox" :aria-label="label" @keydown="onListKeyDown">
      <button
        v-for="option in options"
        :key="option.value"
        class="option"
        :class="{ chosen: option.value === value }"
        type="button"
        role="option"
        :aria-selected="option.value === value"
        @click="choose(option.value)"
      >
        <AppIcon name="check" class="check" />
        {{ option.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.select-field {
  position: relative;
  display: inline-block;
}

.field-button {
  justify-content: space-between;
  width: 100%;
}

.chevron {
  color: var(--color-fg-faint);
}

.list {
  position: absolute;
  top: calc(100% + var(--space-1));
  left: 0;
  z-index: 1;
  display: flex;
  flex-direction: column;
  min-width: 100%;
  padding: var(--space-1) 0;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius);
  background: var(--color-bg-panel);
  box-shadow: 0 calc(8 * var(--px)) calc(23 * var(--px)) var(--color-shadow);
}

.option {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  border: none;
  background: transparent;
  color: var(--color-fg);
  white-space: nowrap;
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition);
}

.option:hover {
  background: var(--color-bg-hover);
}

.option:focus-visible {
  outline-offset: -2px;
}

/* Every option keeps room for the check, so the labels line up. */
.check {
  visibility: hidden;
  color: var(--color-accent);
}

.option.chosen {
  color: var(--color-fg-strong);
}

.option.chosen .check {
  visibility: visible;
}
</style>
