<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from './stores'
import DashboardView from './views/DashboardView.vue'
import LedgerView from './views/LedgerView.vue'
import AnalysisView from './views/AnalysisView.vue'
import SystemView from './views/SystemView.vue'
import QuickBookingDialog from './components/QuickBookingDialog.vue'
import ThemeToggle from './components/ui/ThemeToggle.vue'
import MoneyButton from './components/ui/MoneyButton.vue'

const store = useAppStore()
const activeMenu = ref('dashboard')
const showQuickBooking = ref(false)
const isInitializing = ref(true)

const menus = [
  { key: 'dashboard', label: '仪表盘', icon: 'TrendCharts' },
  { key: 'ledger', label: '账本', icon: 'DocumentAdd' },
  { key: 'smart-analysis', label: '智能分析', icon: 'ChatDotRound' },
  { key: 'system', label: '系统', icon: 'Setting' },
]

const handleMenuSelect = (key: string) => {
  activeMenu.value = key
}

const getPageTitle = () => {
  const item = menus.find(m => m.key === activeMenu.value)
  return item?.label || '记账本'
}

const handleQuickBookingSuccess = () => {
  showQuickBooking.value = false
  store.fetchTransactions()
  store.fetchMonthlyStats()
  store.fetchBudgets()
}

onMounted(async () => {
  while (true) {
    try {
      const ready = await invoke<boolean>('is_app_ready')
      if (ready) {
        isInitializing.value = false
        break
      }
    } catch (e) {
      // 后端可能还没注册完 command，继续等待
    }
    await new Promise(r => setTimeout(r, 300))
  }
  store.initializeData()
})
</script>

<template>
  <div class="ms-app">
    <!-- 初始化遮罩 -->
    <div v-if="isInitializing" class="ms-init-overlay">
      <div class="ms-init-content">
        <el-icon :size="40" class="ms-init-icon"><Loading /></el-icon>
        <p class="ms-init-text">系统初始化中</p>
      </div>
    </div>

    <div class="ms-layout">
      <!-- 侧边栏 -->
      <aside class="ms-sidebar">
        <div class="ms-sidebar-header">
          <div class="ms-logo">
            <svg class="ms-logo-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
            <span class="ms-logo-text">MoneySage</span>
          </div>
        </div>

        <nav class="ms-sidebar-nav">
          <div
            v-for="menu in menus"
            :key="menu.key"
            class="ms-nav-item"
            :class="{ active: activeMenu === menu.key }"
            @click="handleMenuSelect(menu.key)"
          >
            <el-icon :size="18">
              <component :is="menu.icon" />
            </el-icon>
            <span>{{ menu.label }}</span>
          </div>
        </nav>

      </aside>

      <!-- 主内容 -->
      <main class="ms-main">
        <header class="ms-header">
          <div class="ms-header-left">
            <h1 class="ms-page-title">{{ getPageTitle() }}</h1>
            <p class="ms-page-date">{{ new Date().toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' }) }}</p>
          </div>
          <div class="ms-header-right">
            <ThemeToggle />
            <MoneyButton @click="showQuickBooking = true">
              <el-icon><Plus /></el-icon>
              <span>记一笔</span>
            </MoneyButton>
          </div>
        </header>

        <div class="ms-content">
          <DashboardView v-if="activeMenu === 'dashboard'" />
          <LedgerView v-else-if="activeMenu === 'ledger'" />
          <AnalysisView v-else-if="activeMenu === 'smart-analysis'" />
          <SystemView v-else-if="activeMenu === 'system'" />
        </div>
      </main>
    </div>

    <!-- 弹窗 -->
    <QuickBookingDialog v-model="showQuickBooking" @success="handleQuickBookingSuccess" />
  </div>
</template>

<style scoped>
.ms-app {
  height: 100vh;
  background-color: var(--ms-bg-primary);
  color: var(--ms-text-primary);
}

.ms-layout {
  display: flex;
  height: 100%;
}

/* Sidebar */
.ms-sidebar {
  width: var(--ms-sidebar-width);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background-color: var(--ms-bg-secondary);
  border-right: 1px solid var(--ms-border-subtle);
}

.ms-sidebar-header {
  height: var(--ms-header-height);
  display: flex;
  align-items: center;
  padding: 0 16px;
  border-bottom: 1px solid var(--ms-border-subtle);
}

.ms-logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ms-logo-icon {
  width: 32px;
  height: 32px;
  padding: 6px;
  border-radius: var(--ms-radius-md);
  background: var(--ms-gradient-primary);
  color: white;
  flex-shrink: 0;
}

.ms-logo-text {
  font-size: var(--ms-text-lg);
  font-weight: var(--ms-font-bold);
  background: var(--ms-gradient-primary);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.ms-sidebar-nav {
  flex: 1;
  padding: 12px 10px;
  overflow-y: auto;
}

.ms-nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: var(--ms-radius-lg);
  color: var(--ms-text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
  margin-bottom: 4px;
}

.ms-nav-item:hover {
  background-color: var(--ms-surface-hover);
  color: var(--ms-text-primary);
}

.ms-nav-item.active {
  background: var(--ms-gradient-primary);
  color: white;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

/* Main */
.ms-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background-color: var(--ms-bg-primary);
}

.ms-header {
  height: var(--ms-header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 32px;
  border-bottom: 1px solid var(--ms-border-subtle);
  background-color: var(--ms-bg-primary);
  flex-shrink: 0;
}

.ms-header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.ms-page-title {
  margin: 0;
  font-size: var(--ms-text-xl);
  font-weight: var(--ms-font-bold);
  color: var(--ms-text-primary);
}

.ms-page-date {
  margin: 0;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.ms-header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ms-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 32px;
}

/* Init overlay */
.ms-init-overlay {
  position: fixed;
  inset: 0;
  background-color: var(--ms-bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.ms-init-content {
  text-align: center;
  color: var(--ms-text-secondary);
}

.ms-init-icon {
  animation: spin 1.5s linear infinite;
  color: var(--ms-primary-500);
}

.ms-init-text {
  margin-top: 16px;
  font-size: var(--ms-text-base);
  font-weight: var(--ms-font-medium);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Responsive */
@media (max-width: 768px) {
  .ms-sidebar {
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    z-index: 100;
    transform: translateX(-100%);
    transition: transform 0.3s ease;
  }

  .ms-sidebar.is-open {
    transform: translateX(0);
  }

  .ms-content {
    padding: 16px;
  }
}
</style>
