import '@fontsource/ibm-plex-mono/400.css'
import '@fontsource/ibm-plex-mono/400-italic.css'
import '@fontsource/ibm-plex-mono/500.css'
import '@fontsource/ibm-plex-mono/600.css'
import './theme/base.css'

import { createApp, watch } from 'vue'

import { errorMessage } from './api'
import App from './App.vue'
import { loadSettings, settings } from './stores/settings'
import { showToast } from './stores/toasts'
import { applyPalette } from './theme/palette'
import { applyFontSize } from './theme/scale'

// The settings load before anything is drawn, so the window never shows the
// defaults first.
async function start(): Promise<void> {
  applyPalette(document.documentElement)
  try {
    await loadSettings()
  } catch (error) {
    showToast('error', `Couldn't load the settings: ${errorMessage(error)}`)
  }
  watch(
    () => settings.fontSize,
    (fontSize) => applyFontSize(document.documentElement, fontSize),
    { immediate: true },
  )
  createApp(App).mount('#app')
}

void start()
