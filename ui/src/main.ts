import '@fontsource/ibm-plex-mono/400.css'
import '@fontsource/ibm-plex-mono/400-italic.css'
import '@fontsource/ibm-plex-mono/500.css'
import '@fontsource/ibm-plex-mono/600.css'
import './theme/base.css'

import { createApp } from 'vue'

import { errorMessage } from './api'
import App from './App.vue'
import { loadSettings } from './stores/settings'
import { showToast } from './stores/toasts'
import { applyPalette } from './theme/palette'
import { applyScale } from './theme/scale'

// The settings load before anything is drawn, so the window never shows the
// defaults first.
async function start(): Promise<void> {
  applyPalette(document.documentElement)
  applyScale(document.documentElement)
  try {
    await loadSettings()
  } catch (error) {
    showToast('error', `Couldn't load the settings: ${errorMessage(error)}`)
  }
  createApp(App).mount('#app')
}

void start()
