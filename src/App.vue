<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from './stores'
import DashboardView from './views/DashboardView.vue'
import TransactionsView from './views/TransactionsView.vue'
import CategoriesView from './views/CategoriesView.vue'
import BudgetView from './views/BudgetView.vue'
import StatisticsView from './views/StatisticsView.vue'
import ImportExportView from './views/ImportExportView.vue'
import AddTransactionDialog from './components/AddTransactionDialog.vue'

const store = useAppStore()
const activeMenu = ref('dashboard')
const showAddTransaction = ref(false)
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
    'import-export': '导入导出'
  }
  return titles[activeMenu.value] || '记账本'
}

const handleTransactionAdded = () => {
  showAddTransaction.value = false
  // 重新加载数据
  store.fetchTransactions()
  store.fetchMonthlyStats()
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
          <el-icon class="logo-icon" :size="24"><Files /></el-icon>
          <h2 v-show="!isCollapsed" class="logo-text">MoneyNote</h2>
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
            <el-button @click="showAddTransaction = true" class="add-record-btn">
              <el-icon><Plus /></el-icon>
              记一笔
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
          
          <!-- 导入导出 -->
          <ImportExportView v-else-if="activeMenu === 'import-export'" />
        </div>
      </el-main>
    </el-container>
    
    <!-- 添加交易对话框 -->
    <AddTransactionDialog 
      v-model="showAddTransaction" 
      @success="handleTransactionAdded"
    />
  </div>
</template>

<style scoped>
.app-container {
  height: 100vh;
  background: #1a1a1a;
}

.sidebar {
  background: #2a2a2a;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.3);
  transition: width 0.3s ease;
  position: relative;
}

.sidebar.is-collapsed {
  width: 64px !important;
}

.logo {
  display: flex;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid #404040;
  min-height: 64px;
}

.logo-icon {
  color: #ffffff;
  margin-right: 12px;
}

.logo-text {
  color: #ffffff;
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  transition: opacity 0.3s ease;
}

.sidebar.is-collapsed .logo-text {
  opacity: 0;
}

.collapse-btn {
  position: absolute;
  top: 20px;
  right: 12px;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #404040;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s ease;
  z-index: 10;
}

.collapse-btn:hover {
  background: #606060;
}

.collapse-btn .el-icon {
  color: #ffffff;
  font-size: 16px;
}

.sidebar-menu {
  border: none;
  background: transparent;
  padding-top: 12px;
}

.sidebar-menu .el-menu-item {
  height: 48px;
  line-height: 48px;
  margin: 0 12px 4px 12px;
  border-radius: 8px;
  color: #b0b0b0;
  transition: all 0.3s ease;
  position: relative;
}

.sidebar-menu .el-menu-item:hover {
  background: #3a3a3a;
  color: #ffffff;
}

.sidebar-menu .el-menu-item.is-active {
  background: #404040;
  color: #ffffff;
}

.sidebar-menu .el-menu-item.is-active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 4px;
  height: 24px;
  background: #409eff;
  border-radius: 0 2px 2px 0;
}

.sidebar-menu .el-menu-item .el-icon {
  margin-right: 12px;
  font-size: 20px;
}

.sidebar.is-collapsed .sidebar-menu .el-menu-item {
  justify-content: center;
  margin: 0 8px 4px 8px;
}

.sidebar.is-collapsed .sidebar-menu .el-menu-item .el-icon {
  margin-right: 0;
}

.main-content {
  padding: 0;
  background: #1a1a1a;
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  background: #2a2a2a;
  border-bottom: 1px solid #404040;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 20px;
}

.search-container {
  width: 300px;
}

.search-input {
  width: 100%;
}

.header-left h3 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #ffffff;
}

.add-record-btn {
  background: #606060;
  border: 1px solid #707070;
  color: #ffffff;
  font-weight: 500;
  padding: 8px 16px;
  border-radius: 6px;
  transition: all 0.3s ease;
}

.add-record-btn:hover {
  background: #707070;
  border-color: #808080;
  color: #ffffff;
}

.add-record-btn:focus {
  background: #707070;
  border-color: #808080;
  color: #ffffff;
}

.add-record-btn .el-icon {
  margin-right: 4px;
}

.page-content {
  padding: 24px;
  min-height: calc(100vh - 88px);
  background: #1a1a1a;
}

/* 全局样式重置 */
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
  background: #404040;
  border: 1px solid #606060;
  border-radius: 24px;
  padding: 0 16px;
}

:deep(.search-input .el-input__inner) {
  color: #ffffff;
  font-size: 14px;
}

:deep(.search-input .el-input__inner::placeholder) {
  color: #b0b0b0;
}

:deep(.search-input .el-input__prefix) {
  color: #b0b0b0;
}
</style>

<style>
/* 全局样式 */
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  background: #1a1a1a;
  color: #ffffff;
}

.el-button {
  font-weight: 500;
}

.el-card {
  border-radius: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  background: #2a2a2a;
  border: 1px solid #404040;
}

.el-table {
  border-radius: 12px;
  overflow: hidden;
  background: #2a2a2a;
}

.el-dialog {
  border-radius: 12px;
  background: #2a2a2a;
}

.el-form-item__label {
  font-weight: 500;
  color: #ffffff;
}

/* Element Plus 深色主题覆盖 */
.el-card__body {
  background: #2a2a2a;
  color: #ffffff;
}

.el-card__header {
  background: #2a2a2a;
  border-bottom: 1px solid #404040;
  color: #ffffff;
}

.el-table {
  background: #2a2a2a;
  color: #ffffff;
}

.el-table th {
  background: #404040;
  color: #ffffff;
}

.el-table td {
  background: #2a2a2a;
  color: #ffffff;
}

.el-input__wrapper {
  background: #404040;
  border: 1px solid #606060;
  box-shadow: 0 0 0 1px #606060 inset;
}

.el-input__inner {
  background: transparent;
  color: #ffffff;
}

.el-select .el-input__wrapper {
  background: #404040;
  border: 1px solid #606060;
}

.el-select .el-input__inner {
  background: transparent;
  color: #ffffff;
}

.el-date-picker__header {
  background: #2a2a2a;
  border-bottom: 1px solid #404040;
}

.el-picker-panel {
  background: #2a2a2a;
  border: 1px solid #404040;
}

.el-button--primary {
  background: #409eff;
  border-color: #409eff;
}

.el-button--primary:hover {
  background: #66b1ff;
  border-color: #66b1ff;
}

.el-button--small {
  background: #404040;
  border-color: #606060;
  color: #ffffff;
}

.el-button--small:hover {
  background: #606060;
  border-color: #808080;
}

/* 记一笔按钮特殊样式 */
:deep(.add-record-btn) {
  background: #606060 !important;
  border: 1px solid #707070 !important;
  color: #ffffff !important;
  font-weight: 500;
}

:deep(.add-record-btn:hover) {
  background: #707070 !important;
  border-color: #808080 !important;
  color: #ffffff !important;
}

:deep(.add-record-btn:focus) {
  background: #707070 !important;
  border-color: #808080 !important;
  color: #ffffff !important;
}

.el-empty__description {
  color: #b0b0b0;
}

.el-progress-bar__inner {
  background: linear-gradient(to right, #409eff, #66b1ff);
}

.el-menu-item .el-icon {
  color: inherit;
}

.el-dropdown-menu {
  background: #2a2a2a;
  border: 1px solid #404040;
}

.el-dropdown-menu__item {
  color: #ffffff;
}

.el-dropdown-menu__item:hover {
  background: #404040;
}

.el-popover {
  background: #2a2a2a;
  border: 1px solid #404040;
  color: #ffffff;
}

.el-tooltip__popper {
  background: #2a2a2a;
  border: 1px solid #404040;
  color: #ffffff;
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
    padding: 16px 20px;
  }
  
  .page-content {
    padding: 16px 20px;
  }
}
</style>