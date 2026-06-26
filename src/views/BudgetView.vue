<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import { useAppStore, type BudgetProgress } from '../stores'
import AddBudgetDialog from '../components/AddBudgetDialog.vue'
import dayjs from 'dayjs'

const store = useAppStore()

const searchQuery = ref('')
const showAddBudgetDialog = ref(false)
const editingBudget = ref<BudgetProgress | null>(null)
const showDetail = ref(false)
const currentBudget = ref<BudgetProgress | null>(null)
const budgetTransactions = ref<any[]>([])

const budgets = computed(() => store.budgets)

const filteredBudgets = computed(() => {
  if (!searchQuery.value) return budgets.value
  const q = searchQuery.value.toLowerCase()
  return budgets.value.filter(b =>
    b.name.toLowerCase().includes(q) || b.category_name.toLowerCase().includes(q)
  )
})

const totalBudget = computed(() => budgets.value.reduce((sum, b) => sum + b.amount, 0))
const totalSpent = computed(() => budgets.value.reduce((sum, b) => sum + b.spent, 0))
const totalRemaining = computed(() => totalBudget.value - totalSpent.value)
const usagePercentage = computed(() => {
  if (totalBudget.value === 0) return 0
  return (totalSpent.value / totalBudget.value) * 100
})

const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', { minimumFractionDigits: 0, maximumFractionDigits: 2 })
}

const getStatusInfo = (percentage: number) => {
  if (percentage >= 100) return { text: '超支', type: 'danger', color: 'var(--ms-expense)' }
  if (percentage >= 80) return { text: '预警', type: 'warning', color: 'var(--ms-warning)' }
  if (percentage >= 50) return { text: '正常', type: 'primary', color: 'var(--ms-primary-500)' }
  return { text: '充足', type: 'success', color: 'var(--ms-income)' }
}

const getProgressStyle = (percentage: number) => {
  if (percentage >= 100) return { background: 'var(--ms-gradient-expense)' }
  if (percentage >= 80) return { backgroundColor: 'var(--ms-warning)' }
  return { background: 'var(--ms-gradient-primary)' }
}

const editBudget = (budget: BudgetProgress) => {
  editingBudget.value = budget
  showAddBudgetDialog.value = true
}

const deleteBudget = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这个预算吗？', '确认删除', {
      confirmButtonText: '确定', cancelButtonText: '取消', type: 'warning'
    })
    await store.deleteBudget(id)
    ElMessage.success('预算删除成功')
  } catch (error) {
    if (error !== 'cancel') ElMessage.error('删除预算失败')
  }
}

const handleBudgetSuccess = () => {
  showAddBudgetDialog.value = false
  editingBudget.value = null
}

const openBudgetDetail = async (budget: BudgetProgress) => {
  currentBudget.value = budget
  showDetail.value = true
  const start = dayjs(budget.start_date).format('YYYY-MM-DD')
  const end = dayjs(budget.end_date || dayjs().format('YYYY-MM-DD')).format('YYYY-MM-DD')
  try {
    const list = await store.fetchTransactionsByDateRange(start, end)
    budgetTransactions.value = (list || []).filter(t => t.budget_id === budget.id)
  } catch (e) {
    budgetTransactions.value = []
  }
}

onMounted(() => {
  store.fetchBudgets()
  store.fetchCategories()
})
</script>

<template>
  <div class="budget-view">
    <!-- Stats -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-label">总预算</div>
        <div class="stat-value">¥ {{ formatAmount(totalBudget) }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">已支出</div>
        <div class="stat-value" style="color: var(--ms-expense);">¥ {{ formatAmount(totalSpent) }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">剩余</div>
        <div class="stat-value" :style="{ color: totalRemaining < 0 ? 'var(--ms-expense)' : 'var(--ms-income)' }">¥ {{ formatAmount(totalRemaining) }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">预算执行率</div>
        <div class="stat-value">{{ usagePercentage.toFixed(1) }}%</div>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <el-input v-model="searchQuery" placeholder="搜索预算..." prefix-icon="Search" clearable class="search-input" />
      <button class="add-btn" @click="showAddBudgetDialog = true">
        <el-icon><Plus /></el-icon>
        <span>新增预算</span>
      </button>
    </div>

    <!-- Budget Grid -->
    <div v-if="filteredBudgets.length > 0" class="budget-grid">
      <div v-for="budget in filteredBudgets" :key="budget.id" class="budget-card">
        <div class="budget-card-header">
          <div class="budget-category">
            <div class="category-icon" :style="{ backgroundColor: `${budget.category_color || '#6366f1'}1a` }">
              {{ budget.category_icon || '💰' }}
            </div>
            <div>
              <div class="budget-name">{{ budget.name }}</div>
              <div class="budget-type">{{ budget.category_name }} · {{ budget.budget_type === 'time' ? '每月周期' : '事件预算' }}</div>
            </div>
          </div>
          <span class="status-badge" :style="{ backgroundColor: `${getStatusInfo(budget.percentage).color}1a`, color: getStatusInfo(budget.percentage).color }">
            {{ getStatusInfo(budget.percentage).text }}
          </span>
        </div>
        <div class="budget-amounts">
          <span class="spent-amount" :style="{ color: budget.percentage >= 100 ? 'var(--ms-expense)' : 'var(--ms-text-primary)' }">¥{{ formatAmount(budget.spent) }}</span>
          <span class="total-amount">/ ¥{{ formatAmount(budget.amount) }}</span>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: `${Math.min(budget.percentage, 100)}%`, ...getProgressStyle(budget.percentage) }"></div>
        </div>
        <div class="budget-card-footer">
          <span v-if="budget.percentage >= 100" class="footer-text" style="color: var(--ms-expense);">已超支 ¥{{ formatAmount(Math.abs(budget.remaining)) }}</span>
          <span v-else class="footer-text">剩余 ¥{{ formatAmount(budget.remaining) }}</span>
          <span class="footer-text">{{ budget.percentage.toFixed(1) }}%</span>
        </div>
        <div class="budget-card-actions">
          <button class="action-btn" @click="openBudgetDetail(budget)">详情</button>
          <button class="action-btn" @click="editBudget(budget)"><el-icon><Edit /></el-icon></button>
          <button class="action-btn danger" @click="deleteBudget(budget.id)"><el-icon><Delete /></el-icon></button>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <el-empty description="暂无预算记录">
        <button class="add-btn" @click="showAddBudgetDialog = true">添加第一个预算</button>
      </el-empty>
    </div>

    <!-- Add/Edit Dialog -->
    <AddBudgetDialog v-model="showAddBudgetDialog" :budget="editingBudget" @success="handleBudgetSuccess" />

    <!-- Detail Drawer -->
    <el-drawer v-model="showDetail" title="预算详情" size="50%">
      <div v-if="currentBudget" class="budget-detail">
        <div class="detail-header">
          <div class="budget-category">
            <div class="category-icon" :style="{ backgroundColor: `${currentBudget.category_color || '#6366f1'}1a` }">
              {{ currentBudget.category_icon || '💰' }}
            </div>
            <span class="category-name">{{ currentBudget.category_name }}</span>
          </div>
          <div class="amounts">
            <div>预算：¥{{ formatAmount(currentBudget.amount) }}</div>
            <div>已用：¥{{ formatAmount(currentBudget.spent) }}</div>
            <div>剩余：¥{{ formatAmount(currentBudget.remaining) }}</div>
          </div>
        </div>
        <el-divider />
        <h4>关联交易</h4>
        <el-table :data="budgetTransactions" size="small">
          <el-table-column label="日期" width="120" prop="date" />
          <el-table-column label="金额" width="120">
            <template #default="{ row }">
              <span :class="row.type">{{ row.type === 'income' ? '+' : '-' }}¥{{ formatAmount(row.amount) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="描述" min-width="200" prop="description" />
        </el-table>
      </div>
      <div v-else>未选择预算</div>
    </el-drawer>
  </div>
</template>

<style scoped>
.budget-view {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-5);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--ms-space-5);
}

.stat-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) var(--ms-space-5);
  box-shadow: var(--ms-shadow-sm);
}

.stat-label {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-bottom: var(--ms-space-1);
}

.stat-value {
  font-size: 24px;
  font-weight: var(--ms-font-bold);
  color: var(--ms-text-primary);
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ms-space-4);
}

.search-input {
  max-width: 360px;
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: var(--ms-radius-lg);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: white;
  background: var(--ms-gradient-primary);
  border: none;
  cursor: pointer;
  transition: opacity 0.2s ease;
}

.add-btn:hover {
  opacity: 0.9;
}

.budget-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--ms-space-5);
}

.budget-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-5);
  box-shadow: var(--ms-shadow-sm);
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-3);
}

.budget-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}

.budget-category {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
}

.category-icon {
  width: 40px;
  height: 40px;
  border-radius: var(--ms-radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.budget-name {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.budget-type {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-top: 2px;
}

.status-badge {
  font-size: 11px;
  font-weight: var(--ms-font-medium);
  padding: 2px 8px;
  border-radius: 10px;
  flex-shrink: 0;
}

.budget-amounts {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.spent-amount {
  font-size: 24px;
  font-weight: var(--ms-font-bold);
}

.total-amount {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
}

.progress-bar {
  height: 6px;
  background-color: var(--ms-bg-tertiary);
  border-radius: 999px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  transition: width 0.3s ease;
}

.budget-card-footer {
  display: flex;
  justify-content: space-between;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.budget-card-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--ms-space-1);
  padding-top: var(--ms-space-2);
  border-top: 1px solid var(--ms-border-subtle);
}

.action-btn {
  width: 28px;
  height: 28px;
  border-radius: var(--ms-radius-md);
  border: none;
  background-color: transparent;
  color: var(--ms-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.action-btn:hover {
  background-color: var(--ms-bg-tertiary);
  color: var(--ms-text-primary);
}

.action-btn.danger:hover {
  background-color: rgba(244, 63, 94, 0.1);
  color: var(--ms-expense);
}

.empty-state {
  padding: var(--ms-space-10) 0;
}

.budget-detail {
  color: var(--ms-text-primary);
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ms-space-4);
  flex-wrap: wrap;
}

.detail-header .category-name {
  font-size: var(--ms-text-lg);
  font-weight: var(--ms-font-semibold);
}

.detail-header .amounts {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
  text-align: right;
}

.detail-header .amounts div {
  margin-bottom: 4px;
}

.income { color: var(--ms-income); }
.expense { color: var(--ms-expense); }

@media (max-width: 1280px) {
  .budget-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 1024px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }

  .budget-grid {
    grid-template-columns: 1fr;
  }

  .toolbar {
    flex-direction: column;
    align-items: stretch;
  }

  .search-input {
    max-width: 100%;
  }
}
</style>
