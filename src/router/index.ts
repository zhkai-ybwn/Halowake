import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'
import GitAssistantView from '@/views/git-assistant/GitAssistantView.vue'
import DevDockView from '@/views/devdock/DevDockView.vue'
import CodexReportView from '@/views/codex-report/CodexReportView.vue'
import AiQuotaView from '@/views/quota/AiQuotaView.vue'
import SettingsView from '@/views/settings/SettingsView.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: MainLayout,
    children: [
      {
        path: '',
        redirect: { name: 'devdock' }
      },
      {
        path: 'git',
        name: 'git-assistant',
        component: GitAssistantView,
      },
      {
        path: 'devdock',
        name: 'devdock',
        component: DevDockView,
      },
      {
        path: 'codex-report',
        name: 'codex-report',
        component: CodexReportView,
      },
      {
        path: 'ai-quota',
        name: 'ai-quota',
        component: AiQuotaView,
      },
      {
        path: 'settings',
        name: 'settings',
        component: SettingsView,
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
