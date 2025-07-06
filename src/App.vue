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

const handleMenuSelect = (key: string) => {
  activeMenu.value = key
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
      <el-aside width="240px" class="sidebar">
        <div class="logo">
          <h2>📊 MoneyNote</h2>
        </div>
        
        <el-menu
          :default-active="activeMenu"
          class="sidebar-menu"
          @select="handleMenuSelect"
          background-color="#2a2a2a"
          text-color="#b0b0b0"
          active-text-color="#404040"
        >
          <el-menu-item index="dashboard">
            <el-icon><HomeFilled /></el-icon>
            <span>仪表盘</span>
          </el-menu-item>
          
          <el-menu-item index="transactions">
            <el-icon><List /></el-icon>
            <span>记账</span>
          </el-menu-item>
          
          <el-menu-item index="categories">
            <el-icon><Grid /></el-icon>
            <span>分类管理</span>
          </el-menu-item>
          
          <el-menu-item index="budget">
            <el-icon><TrendCharts /></el-icon>
            <span>预算设置</span>
          </el-menu-item>
          
          <el-menu-item index="statistics">
            <el-icon><DataAnalysis /></el-icon>
            <span>数据分析</span>
          </el-menu-item>
          
          <el-menu-item index="import-export">
            <el-icon><Upload /></el-icon>
            <span>导入导出</span>
          </el-menu-item>
        </el-menu>
      </el-aside>
      
      <!-- 主内容区 -->
      <el-main class="main-content">
        <!-- 顶部操作栏 -->
        <div class="header-bar">
          <div class="header-left">
            <h3>{{ getPageTitle() }}</h3>
          </div>
          <div class="header-right">
            <el-button type="primary" @click="showAddTransaction = true">
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
}

.logo {
  padding: 20px;
  text-align: center;
  border-bottom: 1px solid #404040;
}

.logo h2 {
  color: #ffffff;
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.sidebar-menu {
  border: none;
  background: #2a2a2a;
}

.sidebar-menu .el-menu-item {
  height: 56px;
  line-height: 56px;
  border-radius: 0;
  margin: 0;
  color: #b0b0b0;
  transition: all 0.3s ease;
}

.sidebar-menu .el-menu-item:hover {
  background-color: #3a3a3a !important;
  color: #ffffff !important;
}

.sidebar-menu .el-menu-item.is-active {
  background-color: #404040 !important;
  color: #ffffff !important;
  border-right: 3px solid #409eff;
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

.header-left h3 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #ffffff;
}

.page-content {
  padding: 24px;
  min-height: calc(100vh - 88px);
  background: #1a1a1a;
}

/* 全局样式重置 */
:deep(.el-menu-item) {
  font-size: 14px;
}

:deep(.el-menu-item .el-icon) {
  margin-right: 8px;
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