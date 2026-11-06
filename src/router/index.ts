import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'

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
        component: () => import('@/views/git-assistant/GitAssistantView.vue'),
      },
      {
        path: 'devdock',
        name: 'devdock',
        component: () => import('@/views/devdock/DevDockView.vue'),
      },
      {
        path: 'codex-report',
        name: 'codex-report',
        component: () => import('@/views/codex-report/CodexReportView.vue'),
      },
      {
        path: 'markdown',
        name: 'markdown-preview',
        component: () => import('@/views/markdown-preview/MarkdownPreviewView.vue'),
      },
      {
        path: 'settings',
        name: 'settings',
        component: () => import('@/views/settings/SettingsView.vue'),
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
  },
  {
    path: '/preview',
    name: 'markdown-window',
    component: () => import('@/views/markdown-preview/MarkdownWindowView.vue')
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
