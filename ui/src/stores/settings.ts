import { reactive } from 'vue'

import { errorMessage, events, settingsApi, type Settings } from '../api'
import { showToast } from './toasts'

/**
 * The settings, which the backend stores. They start at the backend's
 * defaults, which hold only until `loadSettings` reads the stored ones.
 */
export const settings = reactive<Settings>({
  fontSize: 'normal',
  recentLimit: 10,
  recentProjects: [],
  openProjects: [],
  activeProject: '',
})

/** Reads the stored settings, and follows the ones the backend changes from then on. */
export async function loadSettings(): Promise<void> {
  await events.onSettingsChanged((changes) => Object.assign(settings, changes))
  Object.assign(settings, await settingsApi.all())
}

/** Changes the settings at once and saves them in the background. */
export function changeSettings(changes: Partial<Settings>): void {
  Object.assign(settings, changes)
  settingsApi.set(changes).catch((error: unknown) => {
    showToast('error', `Couldn't save the settings: ${errorMessage(error)}`)
  })
}
