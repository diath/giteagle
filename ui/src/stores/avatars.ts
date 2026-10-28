import { reactive } from 'vue'

import { api } from '../api'

/** Data URL per lowercased email; `null` once known to have no avatar. */
const avatars = reactive(new Map<string, string | null>())
const requested = new Set<string>()

/**
 * Returns the avatar if it has loaded, starting the lookup on first use.
 * Until then callers show the generated placeholder.
 */
export function avatarFor(email: string): string | null {
  const key = email.trim().toLowerCase()
  if (!requested.has(key)) {
    requested.add(key)
    api
      .avatar(key)
      .then((url) => avatars.set(key, url))
      .catch(() => avatars.set(key, null))
  }
  return avatars.get(key) ?? null
}
