import { createRouter, createWebHistory } from 'vue-router'

import { useSessionStore } from '@/entities/session'
import { RouteName } from '@/shared/config/routes'

declare module 'vue-router' {
  interface RouteMeta {
    /** Route is only for signed-out users (login, register). */
    guestOnly?: boolean
    /** Route is reachable without a session. */
    public?: boolean
    /** Marketing page: signed-in users go straight to their files. */
    landing?: boolean
  }
}

export function createAppRouter() {
  const router = createRouter({
    history: createWebHistory(),
    routes: [
      {
        path: '/',
        name: RouteName.Landing,
        component: () => import('@/pages/landing'),
        meta: { public: true, landing: true },
      },
      {
        path: '/login',
        name: RouteName.Login,
        component: () => import('@/pages/login'),
        meta: { public: true, guestOnly: true },
      },
      {
        path: '/register',
        name: RouteName.Register,
        component: () => import('@/pages/register'),
        meta: { public: true, guestOnly: true },
      },
      {
        path: '/documents',
        name: RouteName.Documents,
        component: () => import('@/pages/documents'),
      },
      {
        path: '/documents/:id',
        name: RouteName.Document,
        component: () => import('@/pages/document'),
        props: true,
      },
      {
        path: '/spreadsheets/:id',
        name: RouteName.Spreadsheet,
        component: () => import('@/pages/spreadsheet'),
        props: true,
      },
      {
        path: '/presentations/:id',
        name: RouteName.Presentation,
        component: () => import('@/pages/presentation'),
        props: true,
      },
      {
        path: '/pdfs/:id',
        name: RouteName.Pdf,
        component: () => import('@/pages/pdf'),
        props: true,
      },
      {
        path: '/profile',
        name: RouteName.Profile,
        component: () => import('@/pages/profile'),
      },
      {
        path: '/:pathMatch(.*)*',
        name: RouteName.NotFound,
        component: () => import('@/pages/not-found'),
        meta: { public: true },
      },
    ],
  })

  router.beforeEach(async (to) => {
    const session = useSessionStore()
    await session.whenReady()
    if ((to.meta.guestOnly === true || to.meta.landing === true) && session.isAuthenticated) {
      return { name: RouteName.Documents }
    }
    if (to.meta.public !== true && session.token() === null) {
      return { name: RouteName.Login, query: { redirect: to.fullPath } }
    }
    return true
  })

  return router
}
