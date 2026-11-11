<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

import PageSearchWidget from './PageSearchWidget.vue'

const props = defineProps<{
  /** The view whose text is searched. */
  root: HTMLElement | null
  /** Whether the view takes keyboard input. */
  active: boolean
}>()

const open = ref(false)
/** Kept while the widget is closed, so opening it again picks up where it left off. */
const term = ref('')

function onKeyDown(event: KeyboardEvent): void {
  if (!props.active || !event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {
    return
  }
  // A dialog over the view has the keyboard to itself.
  if (event.key.toLowerCase() !== 'f' || document.querySelector('[role="dialog"]')) {
    return
  }
  event.preventDefault()
  // A held key repeats, which would open and close the widget over and over.
  if (!event.repeat) {
    open.value = !open.value
  }
}

onMounted(() => window.addEventListener('keydown', onKeyDown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
</script>

<template>
  <PageSearchWidget v-if="open && root" v-model:term="term" :root="root" :active="active" @close="open = false" />
</template>
