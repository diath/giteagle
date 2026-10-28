<script setup lang="ts">
import { computed } from 'vue'

import type { Person } from '../api'
import { avatarFor } from '../stores/avatars'
import { initials, placeholderColors } from '../utils/authors'

const MAX_SHOWN = 3

const props = withDefaults(defineProps<{ authors: Person[]; size?: number }>(), { size: 20 })

const shown = computed(() =>
  props.authors.slice(0, MAX_SHOWN).map((author) => ({
    key: author.email.toLowerCase(),
    title: `${author.name} <${author.email}>`,
    image: avatarFor(author.email),
    initials: initials(author.name),
    colors: placeholderColors(author.email),
  })),
)
</script>

<template>
  <span class="avatar-stack" :style="{ '--avatar-size': `${size}px` }">
    <span
      v-for="avatar in shown"
      :key="avatar.key"
      class="avatar"
      :title="avatar.title"
      :style="{ background: avatar.colors.background, color: avatar.colors.foreground }"
    >
      <img v-if="avatar.image" :src="avatar.image" alt="" draggable="false" />
      <span v-else class="initials">{{ avatar.initials }}</span>
    </span>
  </span>
</template>

<style scoped>
.avatar-stack {
  display: inline-flex;
  flex: none;
  align-items: center;
}

.avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--avatar-size);
  height: var(--avatar-size);
  overflow: hidden;
  border: 1.5px solid var(--color-bg-app);
  border-radius: 50%;
  box-sizing: content-box;
}

.avatar + .avatar {
  margin-left: calc(var(--avatar-size) * -0.3);
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.initials {
  font-size: calc(var(--avatar-size) * 0.42);
  font-weight: 600;
  letter-spacing: 0;
  line-height: 1;
}
</style>
