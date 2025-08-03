<template>
  <div class="budget-view">
    <!-- 顶部搜索栏 -->
    <div class="header-section">
      <div class="search-container">
        <el-input
          v-model="searchQuery"
          placeholder="搜索预算..."
          prefix-icon="Search"
          class="search-input"
          clearable
        />
      </div>
      <el-button 
        type="primary" 
        @click="showAddBudgetDialog = true"
        class="add-budget-btn"
      >
        <el-icon><Plus /></el-icon>
        添加预算
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <div class="stats-cards">
      <div class="stat-card">
        <div class="stat-header">
          <h3>总预算</h3>
          <el-icon class="stat-icon"><Money /></el-icon>
        </div>
        <div class="stat-amount">¥{{ formatAmount(totalBudget) }}</div>
        <div class="stat-desc">本月预算</div>
      </div>

      <div class="stat-card">
        <div class="stat-header">
          <h3>已使用</h3>
          <el-icon class="stat-icon"><TrendCharts /></el-icon>
        </div>
        <div class="stat-amount used">¥{{ formatAmount(totalSpent) }}</div>
        <div class="stat-desc">
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${spentPercentage}%` }"></div>
          </div>
          <span>{{ spentPercentage.toFixed(1) }}%</span>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-header">
          <h3>预算类别</h3>
          <el-icon class="stat-icon"><Grid /></el-icon>
        </div>
        <div class="stat-amount">{{ budgetCount }}</div>
        <div class="stat-desc">
          <span class="category-split">
            {{ timeBudgetCount }}个时间预算，{{ eventBudgetCount }}个事件预算
          </span>
        </div>
      </div>
    </div>

    <!-- 预算列表 -->
    <div class="budget-list-section">
      <h3 class="section-title">预算列表</h3>
      
      <el-table
        :data="filteredBudgets"
        style="width: 100%"
        class="budget-table"
      >
        <el-table-column label="预算名称" min-width="150">
          <template #default="{ row }">
            <div class="budget-name">
              <span 
                class="budget-type-badge"
                :class="row.budget_type"
              >
                {{ row.budget_type === 'time' ? '时间' : '事件' }}
              </span>
              {{ row.name }}
            </div>
          </template>
        </el-table-column>

        <el-table-column label="类型" width="120">
          <template #default="{ row }">
            <div class="category-display">
              <span 
                class="category-icon" 
                :style="{ color: row.category_color }"
              >
                {{ row.category_icon || '💰' }}
              </span>
              <span class="category-name">{{ row.category_name }}</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="预算金额" width="120">
          <template #default="{ row }">
            <span class="budget-amount">¥{{ formatAmount(row.amount) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="已用金额" width="120">
          <template #default="{ row }">
            <span class="spent-amount">¥{{ formatAmount(row.spent) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="剩余金额" width="120">
          <template #default="{ row }">
            <span 
              class="remaining-amount"
              :class="{ 'over-budget': row.remaining < 0 }"
            >
              ¥{{ formatAmount(row.remaining) }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="时间范围" width="200">
          <template #default="{ row }">
            <div class="date-range">
              <template v-if="row.budget_type === 'time'">
                {{ formatDate(row.start_date) }} 至 {{ formatDate(row.end_date) }}
              </template>
              <template v-else>
                <span class="event-badge">-</span>
              </template>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="进度" width="150">
          <template #default="{ row }">
            <div class="progress-cell">
              <div 
                class="progress-bar-small"
                :class="{ 'over-budget': row.percentage > 100 }"
              >
                <div 
                  class="progress-fill-small" 
                  :style="{ width: `${Math.min(row.percentage, 100)}%` }"
                ></div>
              </div>
              <span class="progress-text">{{ row.percentage.toFixed(1) }}%</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <div class="action-buttons">
              <el-button
                size="small"
                text
                @click="editBudget(row)"
                class="edit-btn"
              >
                <el-icon><Edit /></el-icon>
              </el-button>
              <el-button
                size="small"
                text
                type="danger"
                @click="deleteBudget(row.id)"
                class="delete-btn"
              >
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>

      <!-- 空状态 -->
      <div v-if="budgets.length === 0" class="empty-state">
        <el-empty description="暂无预算记录">
          <el-button type="primary" @click="showAddBudgetDialog = true">
            添加第一个预算
          </el-button>
        </el-empty>
      </div>
    </div>

    <!-- 添加预算对话框 -->
    <AddBudgetDialog
      v-model="showAddBudgetDialog"
      :budget="editingBudget"
      @success="handleBudgetSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { 
  Plus, 
  Money, 
  TrendCharts, 
  Grid,
  Edit,
  Delete
} from '@element-plus/icons-vue'
import { useAppStore, type BudgetProgress } from '../stores'
import AddBudgetDialog from '../components/AddBudgetDialog.vue'
import dayjs from 'dayjs'

const store = useAppStore()

// 响应式数据
const searchQuery = ref('')
const showAddBudgetDialog = ref(false)
const editingBudget = ref<BudgetProgress | null>(null)

// 计算属性
const budgets = computed(() => store.budgets)

const filteredBudgets = computed(() => {
  if (!searchQuery.value) return budgets.value
  return budgets.value.filter(budget => 
    budget.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
    budget.category_name.toLowerCase().includes(searchQuery.value.toLowerCase())
  )
})

const totalBudget = computed(() => {
  return budgets.value.reduce((sum, budget) => sum + budget.amount, 0)
})

const totalSpent = computed(() => {
  return budgets.value.reduce((sum, budget) => sum + budget.spent, 0)
})

const spentPercentage = computed(() => {
  return totalBudget.value > 0 ? (totalSpent.value / totalBudget.value) * 100 : 0
})

const budgetCount = computed(() => budgets.value.length)

const timeBudgetCount = computed(() => {
  return budgets.value.filter(budget => budget.budget_type === 'time').length
})

const eventBudgetCount = computed(() => {
  return budgets.value.filter(budget => budget.budget_type === 'event').length
})

// 方法
const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  })
}

const formatDate = (date: string) => {
  return dayjs(date).format('YYYY-MM-DD')
}

const editBudget = (budget: BudgetProgress) => {
  editingBudget.value = budget
  showAddBudgetDialog.value = true
}

const deleteBudget = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这个预算吗？', '确认删除', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    await store.deleteBudget(id)
    ElMessage.success('预算删除成功')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除预算失败')
    }
  }
}

const handleBudgetSuccess = () => {
  showAddBudgetDialog.value = false
  editingBudget.value = null
}

// 生命周期
onMounted(() => {
  store.fetchBudgets()
  store.fetchCategories()
})
</script>

<style scoped>
.budget-view {
  padding: 24px;
  background: #1a1a1a;
  min-height: 100vh;
  color: #ffffff;
}

.header-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.search-container {
  flex: 1;
  max-width: 400px;
}

.search-input {
  width: 100%;
}

.add-budget-btn {
  margin-left: 16px;
  height: 40px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.stats-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  margin-bottom: 32px;
}

.stat-card {
  background: #2a2a2a;
  border-radius: 12px;
  padding: 24px;
  border: 1px solid #404040;
  transition: all 0.3s ease;
}

.stat-card:hover {
  border-color: #606060;
  transform: translateY(-2px);
}

.stat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.stat-header h3 {
  margin: 0;
  font-size: 16px;
  color: #b0b0b0;
}

.stat-icon {
  font-size: 24px;
  color: #409eff;
}

.stat-amount {
  font-size: 32px;
  font-weight: 700;
  color: #ffffff;
  margin-bottom: 8px;
}

.stat-amount.used {
  color: #f56c6c;
}

.stat-desc {
  font-size: 14px;
  color: #b0b0b0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.progress-bar {
  width: 120px;
  height: 4px;
  background: #404040;
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #409eff;
  transition: width 0.3s ease;
}

.category-split {
  font-size: 12px;
}

.budget-list-section {
  background: #2a2a2a;
  border-radius: 12px;
  padding: 24px;
  border: 1px solid #404040;
}

.section-title {
  margin: 0 0 20px 0;
  font-size: 18px;
  color: #ffffff;
}

.budget-table {
  background: transparent;
}

.budget-name {
  display: flex;
  align-items: center;
  gap: 8px;
}

.budget-type-badge {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
}

.budget-type-badge.time {
  background: #409eff20;
  color: #409eff;
}

.budget-type-badge.event {
  background: #67c23a20;
  color: #67c23a;
}

.category-display {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
}

.category-name {
  font-size: 14px;
  color: #ffffff;
}

.budget-amount {
  font-weight: 600;
  color: #67c23a;
}

.spent-amount {
  font-weight: 600;
  color: #f56c6c;
}

.remaining-amount {
  font-weight: 600;
  color: #409eff;
}

.remaining-amount.over-budget {
  color: #f56c6c;
}

.date-range {
  font-size: 14px;
  color: #b0b0b0;
}

.event-badge {
  color: #67c23a;
  font-weight: 500;
}

.progress-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.progress-bar-small {
  width: 60px;
  height: 6px;
  background: #404040;
  border-radius: 3px;
  overflow: hidden;
}

.progress-bar-small.over-budget {
  background: #f56c6c20;
}

.progress-fill-small {
  height: 100%;
  background: #409eff;
  transition: width 0.3s ease;
}

.progress-bar-small.over-budget .progress-fill-small {
  background: #f56c6c;
}

.progress-text {
  font-size: 12px;
  color: #b0b0b0;
  min-width: 40px;
}

.action-buttons {
  display: flex;
  gap: 8px;
}

.edit-btn:hover {
  color: #409eff;
}

.delete-btn:hover {
  color: #f56c6c;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
}

/* 深色主题适配 */
:deep(.el-table) {
  background: transparent;
  color: #ffffff;
}

:deep(.el-table__header) {
  background: #1a1a1a;
}

:deep(.el-table th) {
  background: #1a1a1a;
  color: #b0b0b0;
  border-color: #404040;
}

:deep(.el-table td) {
  background: transparent;
  border-color: #404040;
  color: #ffffff;
}

:deep(.el-table__row:hover) {
  background: #404040;
}

:deep(.el-input__inner) {
  background: #2a2a2a;
  border-color: #404040;
  color: #ffffff;
}

:deep(.el-input__inner:focus) {
  border-color: #409eff;
}

:deep(.el-empty) {
  background: transparent;
}

:deep(.el-empty__description) {
  color: #b0b0b0;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .budget-view {
    padding: 16px;
  }
  
  .stats-cards {
    grid-template-columns: 1fr;
  }
  
  .header-section {
    flex-direction: column;
    gap: 16px;
  }
  
  .search-container {
    max-width: 100%;
  }
  
  .add-budget-btn {
    margin-left: 0;
    width: 100%;
  }
}
</style> 