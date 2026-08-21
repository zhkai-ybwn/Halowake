import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import pinia from './store'
import naive from './plugins/naive'
import { i18n } from './i18n'
import { useAiSettingsStore } from './stores/ai-settings'
import { usePreferencesStore } from './stores/preferences'
import { useStorageStore } from './stores/storage'
import { Icon } from '@iconify/vue';
import "@/styles/base/index.scss";
import "@/styles/themes/index.scss";
import "@/styles/utilities/index.scss";
import "@/styles/workbench/index.scss";

async function bootstrap() {
  const app = createApp(App)
  app.use(pinia)
  usePreferencesStore(pinia).initPreferences()
  await useAiSettingsStore(pinia).initSettings()
  void useStorageStore(pinia).initStorage()
  app.use(i18n)
  app.use(router)
  app.use(naive)
  app.component('Icon', Icon);
  app.mount('#app')
}

void bootstrap()
