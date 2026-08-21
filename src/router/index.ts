import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('@/layouts/MainLayout.vue'),
    children: [
      {
        path: '',
        redirect: { name: 'devdock' }
      },
      {
        path: 'git',
        name: 'git-assistant',
        component: () => import('@/views/git-assistant/GitAssistantView.vue')
      },
      {
        path: 'devdock',
        name: 'devdock',
        component: () => import('@/views/devdock/DevDockView.vue')
      },
      {
        path: 'codex-report',
        name: 'codex-report',
        component: () => import('@/views/codex-report/CodexReportView.vue')
      },
      {
        path: 'ai-quota',
        name: 'ai-quota',
        component: () => import('@/views/quota/AiQuotaView.vue')
      },
      {
        path: 'settings',
        name: 'settings',
        component: () => import('@/views/settings/SettingsView.vue')
      },
    ]
  },
  {
    path: '/log',
    name: 'git-log',
    component: () => import('@/views/git-log/GitLogView.vue')
  },
  {
    path: '/diff',
    name: 'git-diff',
    component: () => import('@/views/git-diff/GitDiffView.vue')
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
