<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from './stores'
import DashboardView from './views/DashboardView.vue'
import TransactionsView from './views/TransactionsView.vue'
import CategoriesView from './views/CategoriesView.vue'
import BudgetView from './views/BudgetView.vue'
import StatisticsView from './views/StatisticsView.vue'
import ImportExportView from './views/ImportExportView.vue'
import AnalysisView from './views/AnalysisView.vue'
import QuickBookingDialog from './components/QuickBookingDialog.vue'
import LLMConfigDialog from './components/LLMConfigDialog.vue'

const store = useAppStore()
const activeMenu = ref('dashboard')
const showQuickBooking = ref(false)
const showLLMConfig = ref(false)
const isCollapsed = ref(false)

const handleMenuSelect = (key: string) => {
  activeMenu.value = key
}

const toggleSidebar = () => {
  isCollapsed.value = !isCollapsed.value
}

const getPageTitle = () => {
  const titles: Record<string, string> = {
    dashboard: '财务概览',
    transactions: '记账',
    categories: '分类管理',
    budget: '预算设置',
    statistics: '数据分析',
    'smart-analysis': '智能分析',
    'import-export': '导入导出'
  }
  return titles[activeMenu.value] || '记账本'
}

const handleQuickBookingSuccess = (data: any) => {
  showQuickBooking.value = false
  // 重新加载数据
  store.fetchTransactions()
  store.fetchMonthlyStats()
  store.fetchBudgets()
  console.log('快速记账处理结果:', data)
}

const handleLLMConfigSaved = () => {
  showLLMConfig.value = false
}

onMounted(() => {
  // 初始化数据
  store.initializeData()
})
</script>

<template>
  <div class="app-container">
    <el-container>
      <!-- 侧边栏 -->
      <el-aside :width="isCollapsed ? '64px' : '240px'" class="sidebar" :class="{ 'is-collapsed': isCollapsed }">
        <div class="logo">
          <div class="logo-icon">
            <el-icon :size="18"><Files /></el-icon>
          </div>
          <h2 v-show="!isCollapsed" class="logo-text">MoneySage</h2>
        </div>
        
        <!-- 汉堡菜单按钮 -->
        <div class="collapse-btn" @click="toggleSidebar">
          <el-icon><Menu /></el-icon>
        </div>
        
        <el-menu
          :default-active="activeMenu"
          class="sidebar-menu"
          @select="handleMenuSelect"
          background-color="transparent"
          text-color="#b0b0b0"
          active-text-color="#ffffff"
          :collapse="isCollapsed"
          :collapse-transition="false"
        >
          <el-menu-item index="dashboard">
            <el-icon><TrendCharts /></el-icon>
            <template #title>仪表盘</template>
          </el-menu-item>
          
          <el-menu-item index="transactions">
            <el-icon><DocumentAdd /></el-icon>
            <template #title>记账</template>
          </el-menu-item>
          
          <el-menu-item index="categories">
            <el-icon><Grid /></el-icon>
            <template #title>分类管理</template>
          </el-menu-item>
          
          <el-menu-item index="budget">
            <el-icon><Setting /></el-icon>
            <template #title>预算设置</template>
          </el-menu-item>
          
          <el-menu-item index="statistics">
            <el-icon><DataAnalysis /></el-icon>
            <template #title>数据分析</template>
          </el-menu-item>

          <el-menu-item index="smart-analysis">
            <el-icon><ChatDotRound /></el-icon>
            <template #title>智能分析</template>
          </el-menu-item>
          
          <el-menu-item index="import-export">
            <el-icon><Upload /></el-icon>
            <template #title>导入导出</template>
          </el-menu-item>
        </el-menu>
      </el-aside>
      
      <!-- 主内容区 -->
      <el-main class="main-content">
        <!-- 顶部操作栏 -->
        <div class="header-bar">
          <div class="header-left">
            <div class="search-container">
              <el-input 
                placeholder="搜索交易记录..." 
                prefix-icon="Search"
                class="search-input"
                clearable
              />
            </div>
            <h3>{{ getPageTitle() }}</h3>
          </div>
          <div class="header-right">
                          <el-button @click="showQuickBooking = true" class="quick-booking-btn">
                <el-icon><Plus /></el-icon>
                快速记账
              </el-button>
            <el-button @click="showLLMConfig = true" class="llm-config-btn" title="大模型配置">
              <el-icon><Setting /></el-icon>
            </el-button>
          </div>
        </div>
        
        <!-- 页面内容 -->
        <div class="page-content">
          <!-- 仪表盘 -->
          <DashboardView v-if="activeMenu === 'dashboard'" />
          
          <!-- 交易记录 -->
          <TransactionsView v-else-if="activeMenu === 'transactions'" />
          
          <!-- 分类管理 -->
          <CategoriesView v-else-if="activeMenu === 'categories'" />
          
          <!-- 预算管理 -->
          <BudgetView v-else-if="activeMenu === 'budget'" />
          
          <!-- 统计分析 -->
          <StatisticsView v-else-if="activeMenu === 'statistics'" />
          
          <!-- 智能分析 -->
          <AnalysisView v-else-if="activeMenu === 'smart-analysis'" />

          <!-- 导入导出 -->
          <ImportExportView v-else-if="activeMenu === 'import-export'" />
        </div>
      </el-main>
    </el-container>
    
    <!-- 快速记账对话框 -->
    <QuickBookingDialog 
      v-model="showQuickBooking" 
      @success="handleQuickBookingSuccess"
    />
    
    <!-- 大模型配置对话框 -->
    <LLMConfigDialog 
      v-model="showLLMConfig" 
      @success="handleLLMConfigSaved"
    />
  </div>
</template>

<style scoped>
.app-container {
  height: 100vh;
  background: #0d0d14;
}

.sidebar {
  background: linear-gradient(180deg, #12121e 0%, #0f0f1a 100%);
  border-right: 1px solid rgba(255, 255, 255, 0.06);
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  box-shadow: 4px 0 32px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.sidebar::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  width: 1px;
  height: 100%;
  background: linear-gradient(180deg, transparent, rgba(99, 102, 241, 0.3) 30%, rgba(139, 92, 246, 0.3) 70%, transparent);
  pointer-events: none;
}

.sidebar.is-collapsed {
  width: 64px !important;
}

.logo {
  display: flex;
  align-items: center;
  padding: 18px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  min-height: 64px;
  gap: 12px;
}

.logo-icon {
  width: 36px;
  height: 36px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
}

.logo-text {
  color: #e2e8f0;
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.3px;
  transition: opacity 0.3s ease;
  white-space: nowrap;
  background: linear-gradient(135deg, #e2e8f0, #a5b4fc);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.sidebar.is-collapsed .logo-text {
  opacity: 0;
  width: 0;
}

.collapse-btn {
  position: absolute;
  top: 18px;
  right: 12px;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  z-index: 10;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.collapse-btn:hover {
  background: rgba(99, 102, 241, 0.2);
  border-color: rgba(99, 102, 241, 0.4);
}

.collapse-btn .el-icon {
  color: #94a3b8;
  font-size: 15px;
  transition: color 0.2s;
}

.collapse-btn:hover .el-icon {
  color: #a5b4fc;
}

.sidebar-menu {
  border: none;
  background: transparent;
  padding: 12px 0;
}

.sidebar-menu .el-menu-item {
  height: 46px;
  line-height: 46px;
  margin: 2px 10px;
  border-radius: 10px;
  color: #64748b;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  font-size: 14px;
  font-weight: 500;
}

.sidebar-menu .el-menu-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #cbd5e1;
}

.sidebar-menu .el-menu-item.is-active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(139, 92, 246, 0.15));
  color: #a5b4fc;
  border: 1px solid rgba(99, 102, 241, 0.2);
}

.sidebar-menu .el-menu-item.is-active::before {
  content: '';
  position: absolute;
  left: -1px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 22px;
  background: linear-gradient(180deg, #6366f1, #8b5cf6);
  border-radius: 0 3px 3px 0;
  box-shadow: 0 0 8px rgba(99, 102, 241, 0.6);
}

.sidebar-menu .el-menu-item .el-icon {
  margin-right: 10px;
  font-size: 18px;
  transition: all 0.2s;
}

.sidebar.is-collapsed .sidebar-menu .el-menu-item {
  justify-content: center;
  margin: 2px 6px;
}

.sidebar.is-collapsed .sidebar-menu .el-menu-item .el-icon {
  margin-right: 0;
}

.main-content {
  padding: 0;
  background: #0d0d14;
  overflow-y: auto;
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 28px;
  height: 64px;
  background: rgba(13, 13, 20, 0.85);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.search-container {
  width: 260px;
}

.search-input {
  width: 100%;
}

.header-left h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #e2e8f0;
  letter-spacing: -0.2px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.llm-config-btn {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  font-weight: 500;
  padding: 8px 12px;
  border-radius: 10px;
  transition: all 0.2s ease;
  min-width: auto;
}

.llm-config-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.2);
  color: #e2e8f0;
}

.llm-config-btn:focus {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.15);
  color: #e2e8f0;
}

.quick-booking-btn {
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border: none;
  color: #ffffff;
  font-weight: 600;
  padding: 8px 18px;
  border-radius: 10px;
  transition: all 0.2s ease;
  box-shadow: 0 4px 15px rgba(99, 102, 241, 0.35);
  font-size: 14px;
}

.quick-booking-btn:hover {
  background: linear-gradient(135deg, #7c7ff5, #9d6ef8);
  box-shadow: 0 6px 20px rgba(99, 102, 241, 0.5);
  transform: translateY(-1px);
  color: #ffffff;
  border: none;
}

.quick-booking-btn:focus {
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border: none;
  color: #ffffff;
}

.quick-booking-btn .el-icon {
  margin-right: 5px;
}

.page-content {
  padding: 24px 28px;
  min-height: calc(100vh - 64px);
}

:deep(.el-menu-item) {
  font-size: 14px;
  font-weight: 500;
}

:deep(.el-menu-item .el-icon) {
  color: inherit;
}

:deep(.el-menu--collapse .el-menu-item .el-icon) {
  margin-right: 0;
}

:deep(.search-input .el-input__wrapper) {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.08) !important;
  border-radius: 10px !important;
  padding: 0 14px !important;
  box-shadow: none !important;
  transition: all 0.2s;
}

:deep(.search-input .el-input__wrapper:hover),
:deep(.search-input .el-input__wrapper.is-focus) {
  border-color: rgba(99, 102, 241, 0.5) !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1) !important;
}

:deep(.search-input .el-input__inner) {
  color: #e2e8f0 !important;
  font-size: 14px !important;
}

:deep(.search-input .el-input__inner::placeholder) {
  color: #475569 !important;
}

:deep(.search-input .el-input__prefix) {
  color: #475569 !important;
}
</style>

<style>
/* 全局样式 */
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: 'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, sans-serif;
  background: #0d0d14;
  color: #e2e8f0;
}

.el-button {
  font-weight: 500;
  letter-spacing: 0.01em;
}

.el-card {
  border-radius: 14px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
  background: #151520;
  border: 1px solid rgba(255, 255, 255, 0.07);
}

.el-table {
  border-radius: 12px;
  overflow: hidden;
  background: #151520;
}

.el-dialog {
  border-radius: 16px;
  background: #151520;
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(99, 102, 241, 0.1);
}

.el-dialog__header {
  background: #151520;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  padding: 20px 24px 16px;
  border-radius: 16px 16px 0 0;
}

.el-dialog__title {
  color: #e2e8f0;
  font-size: 16px;
  font-weight: 600;
}

.el-dialog__body {
  background: #151520;
  color: #e2e8f0;
  padding: 20px 24px;
}

.el-dialog__footer {
  background: #151520;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding: 16px 24px;
  border-radius: 0 0 16px 16px;
}

.el-form-item__label {
  font-weight: 500;
  color: #94a3b8 !important;
  font-size: 13px;
}

/* Element Plus 深色主题覆盖 */
.el-card__body {
  background: #151520;
  color: #e2e8f0;
}

.el-card__header {
  background: #151520;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  color: #e2e8f0;
  padding: 16px 20px;
}

.el-table {
  background: #151520;
  color: #e2e8f0;
}

.el-table th.el-table__cell {
  background: rgba(255, 255, 255, 0.03) !important;
  color: #64748b !important;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06) !important;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.el-table td.el-table__cell {
  background: transparent !important;
  color: #cbd5e1;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04) !important;
}

.el-table tr:hover td.el-table__cell {
  background: rgba(255, 255, 255, 0.03) !important;
}

.el-table__empty-block {
  background: transparent;
}

.el-input__wrapper {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  box-shadow: none !important;
  border-radius: 8px !important;
  transition: all 0.2s;
}

.el-input__wrapper:hover {
  border-color: rgba(255, 255, 255, 0.18) !important;
}

.el-input__wrapper.is-focus {
  border-color: rgba(99, 102, 241, 0.6) !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.12) !important;
}

.el-input__inner {
  background: transparent !important;
  color: #e2e8f0 !important;
}

.el-input__inner::placeholder {
  color: #475569 !important;
}

.el-select .el-input__wrapper {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
}

.el-select-dropdown {
  background: #1a1a28 !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  border-radius: 10px !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
}

.el-select-dropdown__item {
  color: #94a3b8 !important;
  font-size: 14px;
}

.el-select-dropdown__item.is-hovering {
  background: rgba(99, 102, 241, 0.1) !important;
  color: #a5b4fc !important;
}

.el-select-dropdown__item.is-selected {
  color: #a5b4fc !important;
  font-weight: 600;
}

.el-date-picker__header {
  background: #1a1a28;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.el-picker-panel {
  background: #1a1a28 !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  border-radius: 12px !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
}

.el-date-table td .el-date-table-cell__text {
  color: #94a3b8;
}

.el-date-table td.today .el-date-table-cell__text {
  color: #a5b4fc;
}

.el-date-table td.current .el-date-table-cell__text {
  background: linear-gradient(135deg, #6366f1, #8b5cf6) !important;
}

.el-button--primary {
  background: linear-gradient(135deg, #6366f1, #8b5cf6) !important;
  border-color: transparent !important;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
  font-weight: 600;
}

.el-button--primary:hover {
  background: linear-gradient(135deg, #7c7ff5, #9d6ef8) !important;
  border-color: transparent !important;
  box-shadow: 0 6px 16px rgba(99, 102, 241, 0.4);
  transform: translateY(-1px);
}

.el-button--default {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  color: #94a3b8 !important;
}

.el-button--default:hover {
  background: rgba(255, 255, 255, 0.1) !important;
  border-color: rgba(255, 255, 255, 0.2) !important;
  color: #e2e8f0 !important;
}

.el-button--small {
  background: rgba(255, 255, 255, 0.06) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  color: #94a3b8 !important;
  border-radius: 8px !important;
}

.el-button--small:hover {
  background: rgba(255, 255, 255, 0.12) !important;
  border-color: rgba(255, 255, 255, 0.2) !important;
  color: #e2e8f0 !important;
}

.el-empty__description {
  color: #475569;
}

.el-empty__image svg path {
  fill: rgba(255, 255, 255, 0.06) !important;
}

.el-progress-bar__inner {
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
}

.el-progress-bar__outer {
  background: rgba(255, 255, 255, 0.06) !important;
}

.el-menu-item .el-icon {
  color: inherit;
}

.el-dropdown-menu {
  background: #1a1a28 !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  border-radius: 10px !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
}

.el-dropdown-menu__item {
  color: #94a3b8 !important;
}

.el-dropdown-menu__item:hover {
  background: rgba(99, 102, 241, 0.1) !important;
  color: #a5b4fc !important;
}

.el-popover {
  background: #1a1a28 !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  color: #e2e8f0 !important;
  border-radius: 10px !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
}

.el-tooltip__popper {
  background: #1a1a28 !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  color: #e2e8f0 !important;
}

/* 滚动条全局美化 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.22);
}

/* 响应式设计 */
@media (max-width: 768px) {
  .el-aside {
    position: fixed;
    height: 100vh;
    z-index: 999;
    transform: translateX(-100%);
    transition: transform 0.3s ease;
  }
  
  .el-aside.mobile-show {
    transform: translateX(0);
  }
  
  .el-main {
    margin-left: 0;
  }
  
  .header-bar {
    padding: 0 16px;
  }
  
  .page-content {
    padding: 16px;
  }
}
</style>