import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createPinia } from 'pinia'
import { createApp } from 'vue'

import { useSessionStore } from '@/entities/session'
import { ApiError, configureAuth } from '@/shared/api'
import { RouteName } from '@/shared/config/routes'
import { createAppI18n } from '@/shared/i18n'
import { startTheme } from '@/shared/lib'

import App from './App.vue'
import { createAppRouter } from './router'
import './styles.css'

startTheme()

const app = createApp(App)
const pinia = createPinia()
const router = createAppRouter()
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: (failures, error) =>
        failures < 2 && !(error instanceof ApiError && error.status >= 400 && error.status < 500),
    },
  },
})

app.use(pinia)

const session = useSessionStore(pinia)
configureAuth({
  getToken: () => session.token(),
  onUnauthorized: () => {
    session.signOut()
    queryClient.clear()
    void router.replace({
      name: RouteName.Login,
      query: { redirect: router.currentRoute.value.fullPath },
    })
  },
})

void session.restore()
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && session.expiresSoon()) {
    void session.refresh()
  }
})

app.use(router)
app.use(VueQueryPlugin, { queryClient })
app.use(createAppI18n('en'))
app.mount('#app')
