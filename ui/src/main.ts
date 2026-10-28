import '@fontsource/ibm-plex-mono/400.css'
import '@fontsource/ibm-plex-mono/400-italic.css'
import '@fontsource/ibm-plex-mono/500.css'
import '@fontsource/ibm-plex-mono/600.css'
import './theme/base.css'

import { createApp } from 'vue'

import App from './App.vue'
import { applyPalette } from './theme/palette'

applyPalette(document.documentElement)
createApp(App).mount('#app')
