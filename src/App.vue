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
    dashboard: '仪表盘',
    transactions: '交易记录',
    categories: '分类管理',
    budget: '预算管理',
    statistics: '统计分析',
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
          <h2>💰 记账本</h2>
        </div>
        
        <el-menu
          :default-active="activeMenu"
          class="sidebar-menu"
          @select="handleMenuSelect"
          background-color="#2c3e50"
          text-color="#ecf0f1"
          active-text-color="#3498db"
        >
          <el-menu-item index="dashboard">
            <el-icon><HomeFilled /></el-icon>
            <span>仪表盘</span>
          </el-menu-item>
          
          <el-menu-item index="transactions">
            <el-icon><List /></el-icon>
            <span>交易记录</span>
          </el-menu-item>
          
          <el-menu-item index="categories">
            <el-icon><Grid /></el-icon>
            <span>分类管理</span>
          </el-menu-item>
          
          <el-menu-item index="budget">
            <el-icon><TrendCharts /></el-icon>
            <span>预算管理</span>
          </el-menu-item>
          
          <el-menu-item index="statistics">
            <el-icon><DataAnalysis /></el-icon>
            <span>统计分析</span>
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
  background: #f5f7fa;
}

.sidebar {
  background: #2c3e50;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.logo {
  padding: 20px;
  text-align: center;
  border-bottom: 1px solid #34495e;
}

.logo h2 {
  color: #ecf0f1;
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.sidebar-menu {
  border: none;
}

.sidebar-menu .el-menu-item {
  height: 56px;
  line-height: 56px;
  border-radius: 0;
  margin: 0;
}

.sidebar-menu .el-menu-item:hover {
  background-color: #34495e !important;
}

.sidebar-menu .el-menu-item.is-active {
  background-color: #3498db !important;
  color: #fff !important;
}

.main-content {
  padding: 0;
  background: #f5f7fa;
}

.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  background: #fff;
  border-bottom: 1px solid #e6e8eb;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

.header-left h3 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #2c3e50;
}

.page-content {
  padding: 24px;
  min-height: calc(100vh - 88px);
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
  background: #f5f7fa;
}

.el-button {
  font-weight: 500;
}

.el-card {
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.el-table {
  border-radius: 8px;
  overflow: hidden;
}

.el-dialog {
  border-radius: 8px;
}

.el-form-item__label {
  font-weight: 500;
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