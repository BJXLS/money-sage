<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete, Wallet, Flag, Timer } from '@element-plus/icons-vue'
import { useAppStore, type BudgetProgress } from '../stores'
import AddBudgetDialog from '../components/AddBudgetDialog.vue'
import dayjs from 'dayjs'

const store = useAppStore()

const activeTab = ref<'all' | 'total' | 'category' | 'event'>('all')
const searchQuery = ref('')
const showAddBudgetDialog = ref(false)
const editingBudget = ref<BudgetProgress | null>(null)
const showDetailDrawer = ref(false)
const currentBudget = ref<BudgetProgress | null>(null)
const budgetTransactions = ref<any[]>([])

type BudgetUiType = 'total' | 'category' | 'event'

const tabs: { key: BudgetUiType | 'all'; label: string }[] = [
  { key: 'all', label: '全部' },
  { key: 'total', label: '总预算' },
  { key: 'category', label: '分类预算' },
  { key: 'event', label: '事件预算' },
]

const getUiType = (budget: BudgetProgress): BudgetUiType => {
  if (budget.budget_type === 'total') return 'total'
  if (budget.budget_type === 'event') return 'event'
  return 'category'
}

const budgets = computed(() => store.budgets)

const filteredBudgets = computed(() => {
  let list = budgets.value
  if (activeTab.value !== 'all') {
    list = list.filter(b => getUiType(b) === activeTab.value)
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(
      b =>
        b.name.toLowerCase().includes(q) ||
        (b.category_name || '').toLowerCase().includes(q)
    )
  }
  return list
})

const totalBudgetAmount = computed(() =>
  filteredBudgets.value.reduce((sum, b) => sum + b.amount, 0)
)
const totalSpent = computed(() =>
  filteredBudgets.value.reduce((sum, b) => sum + b.spent, 0)
)
const totalRemaining = computed(() => totalBudgetAmount.value - totalSpent.value)
const alertCount = computed(() =>
  filteredBudgets.value.filter(b => b.percentage >= 80).length
)
const overBudgetCount = computed(() =>
  filteredBudgets.value.filter(b => b.percentage >= 100).length
)

const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  })
}

const formatDate = (date?: string | null) => {
  if (!date) return ''
  return dayjs(date).format('YYYY-MM-DD')
}

const getStatusInfo = (percentage: number) => {
  if (percentage >= 100) return { text: '超支', type: 'danger' }
  if (percentage >= 80) return { text: '预警', type: 'warning' }
  if (percentage >= 50) return { text: '正常', type: 'primary' }
  return { text: '充足', type: 'success' }
}

const getProgressStyle = (percentage: number) => {
  if (percentage >= 100) return { background: 'var(--ms-gradient-expense)' }
  if (percentage >= 80) return { backgroundColor: 'var(--ms-warning)' }
  return { background: 'var(--ms-gradient-primary)' }
}

const getBudgetIcon = (budget: BudgetProgress) => {
  if (budget.category_icon) return budget.category_icon
  if (budget.budget_type === 'total') return '🏦'
  if (budget.budget_type === 'event') return '🎯'
  return '🏷️'
}

const getBudgetIconColor = (budget: BudgetProgress) => {
  return budget.category_color || (budget.budget_type === 'event' ? '#8b5cf6' : '#6366f1')
}

const getPeriodLabel = (periodType?: string | null) => {
  const map: Record<string, string> = {
    daily: '每日',
    weekly: '每周',
    monthly: '每月',
    yearly: '每年',
  }
  return map[periodType || ''] || ''
}

const getBudgetTypeLabel = (budget: BudgetProgress) => {
  if (budget.budget_type === 'total') return '总预算'
  if (budget.budget_type === 'event') return '事件预算'
  return '分类预算'
}

const getBudgetSubtitle = (budget: BudgetProgress) => {
  const typeLabel = getBudgetTypeLabel(budget)
  if (budget.budget_type === 'total') {
    return `${typeLabel} · ${getPeriodLabel(budget.period_type)}`
  }
  if (budget.budget_type === 'event') {
    return typeLabel
  }
  return `${typeLabel} · ${budget.category_name || '未分类'} · ${getPeriodLabel(budget.period_type)}`
}

const getBudgetRangeText = (budget: BudgetProgress) => {
  if (budget.budget_type === 'event') {
    if (budget.start_date && budget.end_date) {
      return `${formatDate(budget.start_date)} 至 ${formatDate(budget.end_date)}`
    }
    if (budget.start_date) return `${formatDate(budget.start_date)} 开始`
    if (budget.end_date) return `截止 ${formatDate(budget.end_date)}`
    return '不限制时间范围'
  }
  if (budget.period_start && budget.period_end) {
    return `${formatDate(budget.period_start)} 至 ${formatDate(budget.period_end)}`
  }
  return getPeriodLabel(budget.period_type) || '当前周期'
}

const getEventDaysText = (budget: BudgetProgress) => {
  if (budget.budget_type !== 'event' || !budget.end_date) return null
  const remaining = dayjs(budget.end_date).diff(dayjs(), 'day')
  if (remaining > 0) return `还剩 ${remaining} 天`
  if (remaining === 0) return '今天截止'
  return `已逾期 ${Math.abs(remaining)} 天`
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
      type: 'warning',
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
  showDetailDrawer.value = true
  budgetTransactions.value = []

  try {
    let list: any[] = []
    if (budget.budget_type === 'event') {
      // 事件预算不限制日期，获取全部交易后按 budget_id 过滤
      const start = '1900-01-01'
      const end = '2099-12-31'
      list = (await store.fetchTransactionsByDateRange(start, end)) || []
    } else {
      const start = formatDate(budget.period_start) || dayjs().startOf('month').format('YYYY-MM-DD')
      const end = formatDate(budget.period_end) || dayjs().endOf('month').format('YYYY-MM-DD')
      list = (await store.fetchTransactionsByDateRange(start, end)) || []
    }
    budgetTransactions.value = list.filter(t => t.budget_id === budget.id && t.type === 'expense')
  } catch (e) {
    budgetTransactions.value = []
  }
}

const eventCategoryBreakdown = computed(() => {
  if (!currentBudget.value || currentBudget.value.budget_type !== 'event') return []
  const map = new Map<
    number,
    { name: string; icon: string; color: string; amount: number }
  >()
  budgetTransactions.value.forEach(t => {
    if (t.type !== 'expense') return
    const id = t.category_id
    const existing = map.get(id)
    if (existing) {
      existing.amount += t.amount
    } else {
      map.set(id, {
        name: t.category_name || '未知分类',
        icon: t.category_icon || '📋',
        color: t.category_color || '#94a3b8',
        amount: t.amount,
      })
    }
  })
  return Array.from(map.values()).sort((a, b) => b.amount - a.amount)
})

const eventTotalSpent = computed(() =>
  eventCategoryBreakdown.value.reduce((sum, item) => sum + item.amount, 0)
)

onMounted(() => {
  store.fetchBudgets()
  store.fetchCategories()
})
</script>

<template>
  <div class="budget-view">
    <!-- Summary Cards -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-label">
          <el-icon><Wallet /></el-icon>
          <span>总预算</span>
        </div>
        <div class="stat-value">¥ {{ formatAmount(totalBudgetAmount) }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">
          <el-icon><Timer /></el-icon>
          <span>已支出</span>
        </div>
        <div class="stat-value" style="color: var(--ms-expense);">
          ¥ {{ formatAmount(totalSpent) }}
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">
          <el-icon><Wallet /></el-icon>
          <span>剩余可用</span>
        </div>
        <div
          class="stat-value"
          :style="{ color: totalRemaining < 0 ? 'var(--ms-expense)' : 'var(--ms-income)' }"
        >
          ¥ {{ formatAmount(totalRemaining) }}
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">
          <el-icon><Flag /></el-icon>
          <span>预算状态</span>
        </div>
        <div
          class="stat-value"
          :style="{
            color: overBudgetCount > 0 ? 'var(--ms-expense)' : alertCount > 0 ? 'var(--ms-warning)' : 'var(--ms-income)',
          }"
        >
          {{ alertCount }} 个预警
        </div>
        <div class="stat-sub" v-if="overBudgetCount > 0">
          {{ overBudgetCount }} 个预算已超支
        </div>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <div class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['tab-btn', { active: activeTab === tab.key }]"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </div>
      <div class="toolbar-right">
        <el-input
          v-model="searchQuery"
          placeholder="搜索预算..."
          prefix-icon="Search"
          clearable
          class="search-input"
        />
        <button class="add-btn" @click="showAddBudgetDialog = true">
          <el-icon><Plus /></el-icon>
          <span>新增预算</span>
        </button>
      </div>
    </div>

    <!-- Budget Grid -->
    <div v-if="filteredBudgets.length > 0" class="budget-grid">
      <div
        v-for="budget in filteredBudgets"
        :key="budget.id"
        class="budget-card"
        @click="openBudgetDetail(budget)"
      >
        <div class="budget-card-header">
          <div class="budget-category">
            <div
              class="category-icon"
              :style="{ backgroundColor: `${getBudgetIconColor(budget)}1a` }"
            >
              {{ getBudgetIcon(budget) }}
            </div>
            <div>
              <div class="budget-name">{{ budget.name }}</div>
              <div class="budget-type">{{ getBudgetSubtitle(budget) }}</div>
              <div class="budget-range">{{ getBudgetRangeText(budget) }}</div>
            </div>
          </div>
          <span
            class="status-badge"
            :style="{
              backgroundColor: `var(--el-color-${getStatusInfo(budget.percentage).type}-light-9)`,
              color: `var(--el-color-${getStatusInfo(budget.percentage).type})`,
            }"
          >
            {{ getStatusInfo(budget.percentage).text }}
          </span>
        </div>

        <div class="budget-progress">
          <div class="progress-info">
            <span class="achievement-rate">达成率 {{ budget.percentage.toFixed(1) }}%</span>
            <span
              class="remaining-amount"
              :style="{
                color: budget.percentage >= 100 ? 'var(--ms-expense)' : 'var(--ms-text-secondary)',
              }"
            >
              {{ budget.percentage >= 100 ? '超支' : '剩余' }}
              ¥{{ formatAmount(Math.abs(budget.remaining)) }}
            </span>
          </div>
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{
                width: `${Math.min(budget.percentage, 100)}%`,
                ...getProgressStyle(budget.percentage),
              }"
            ></div>
          </div>
        </div>

        <div class="budget-card-footer">
          <span class="spent-text">
            已支出 <strong>¥{{ formatAmount(budget.spent) }}</strong> / ¥{{ formatAmount(budget.amount) }}
          </span>
          <div class="budget-card-actions" @click.stop>
            <button class="action-btn" @click="editBudget(budget)">
              <el-icon><Edit /></el-icon>
            </button>
            <button class="action-btn danger" @click="deleteBudget(budget.id)">
              <el-icon><Delete /></el-icon>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <el-empty description="暂无预算记录">
        <button class="add-btn" @click="showAddBudgetDialog = true">
          添加第一个预算
        </button>
      </el-empty>
    </div>

    <!-- Add/Edit Dialog -->
    <AddBudgetDialog
      v-model="showAddBudgetDialog"
      :budget="editingBudget"
      @success="handleBudgetSuccess"
    />

    <!-- Detail Drawer -->
    <el-drawer
      v-model="showDetailDrawer"
      :title="currentBudget?.name || '预算详情'"
      size="520px"
      destroy-on-close
    >
      <div v-if="currentBudget" class="budget-detail">
        <!-- Header -->
        <div class="detail-header">
          <div class="budget-category">
            <div
              class="category-icon"
              :style="{ backgroundColor: `${getBudgetIconColor(currentBudget)}1a` }"
            >
              {{ getBudgetIcon(currentBudget) }}
            </div>
            <div>
              <div class="budget-name">{{ currentBudget.name }}</div>
              <div class="budget-type">{{ getBudgetSubtitle(currentBudget) }}</div>
            </div>
          </div>
        </div>

        <!-- Progress Card -->
        <div class="detail-progress-card">
          <div class="detail-amounts">
            <div>
              <div class="detail-amount-label">{{ currentBudget.budget_type === 'event' ? '事件预算' : '本期预算' }}</div>
              <div class="detail-amount-value">¥{{ formatAmount(currentBudget.amount) }}</div>
            </div>
            <div class="text-right">
              <div class="detail-amount-label">已支出</div>
              <div
                class="detail-amount-value-spent"
                :style="{
                  color: currentBudget.percentage >= 100 ? 'var(--ms-expense)' : 'var(--ms-text-primary)',
                }"
              >
                ¥{{ formatAmount(currentBudget.spent) }}
              </div>
            </div>
          </div>
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{
                width: `${Math.min(currentBudget.percentage, 100)}%`,
                ...getProgressStyle(currentBudget.percentage),
              }"
            ></div>
          </div>
          <div class="detail-progress-footer">
            <span>达成率 {{ currentBudget.percentage.toFixed(1) }}%</span>
            <span
              :style="{
                color: currentBudget.percentage >= 100 ? 'var(--ms-expense)' : 'var(--ms-text-secondary)',
              }"
            >
              {{ currentBudget.percentage >= 100 ? '超支' : '剩余' }}
              ¥{{ formatAmount(Math.abs(currentBudget.remaining)) }}
            </span>
          </div>
        </div>

        <!-- Event-only: time range and countdown -->
        <template v-if="currentBudget.budget_type === 'event'">
          <div class="detail-section">
            <div class="detail-section-title">事件时间</div>
            <div class="event-time-card">
              <div class="event-time-range">
                {{ getBudgetRangeText(currentBudget) }}
              </div>
              <div v-if="getEventDaysText(currentBudget)" class="event-countdown">
                {{ getEventDaysText(currentBudget) }}
              </div>
              <el-tag
                v-if="currentBudget.start_date || currentBudget.end_date"
                size="small"
                :type="
                  dayjs().isAfter(dayjs(currentBudget.end_date || '2099-12-31'))
                    ? 'info'
                    : 'primary'
                "
                class="event-status-tag"
              >
                {{
                  dayjs().isAfter(dayjs(currentBudget.end_date || '2099-12-31'))
                    ? '已结束'
                    : '进行中'
                }}
              </el-tag>
            </div>
          </div>

          <!-- Category Breakdown -->
          <div class="detail-section">
            <div class="detail-section-title">按分类汇总</div>
            <div v-if="eventCategoryBreakdown.length > 0" class="category-breakdown">
              <div
                v-for="item in eventCategoryBreakdown"
                :key="item.name"
                class="breakdown-item"
              >
                <div class="breakdown-info">
                  <span class="breakdown-icon">{{ item.icon }}</span>
                  <span class="breakdown-name">{{ item.name }}</span>
                  <span class="breakdown-amount">¥{{ formatAmount(item.amount) }}</span>
                </div>
                <div class="progress-bar">
                  <div
                    class="progress-fill"
                    :style="{
                      width: `${eventTotalSpent > 0 ? (item.amount / eventTotalSpent) * 100 : 0}%`,
                      backgroundColor: item.color,
                    }"
                  ></div>
                </div>
              </div>
            </div>
            <div v-else class="empty-hint">暂无支出记录</div>
          </div>
        </template>

        <!-- Linked Transactions -->
        <div class="detail-section">
          <div class="detail-section-title">
            {{ currentBudget.budget_type === 'event' ? '关联交易时间轴' : '本期关联交易' }}
          </div>
          <div
            v-if="budgetTransactions.length > 0"
            :class="[
              'transaction-list',
              { timeline: currentBudget.budget_type === 'event' },
            ]"
          >
            <div
              v-for="tx in budgetTransactions"
              :key="tx.id"
              class="transaction-item"
            >
              <div class="transaction-left">
                <div
                  class="category-icon small"
                  :style="{ backgroundColor: `${tx.category_color || '#6366f1'}1a` }"
                >
                  {{ tx.category_icon || '📋' }}
                </div>
                <div class="transaction-info">
                  <div class="transaction-desc">{{ tx.description || tx.category_name || '未命名' }}</div>
                  <div class="transaction-date">
                    {{ formatDate(tx.date) }} · {{ tx.category_name || '未知分类' }}
                  </div>
                </div>
              </div>
              <div class="transaction-amount expense">
                -¥{{ formatAmount(tx.amount) }}
              </div>
            </div>
          </div>
          <div v-else class="empty-hint">暂无关联交易</div>
        </div>
      </div>
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
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-bottom: var(--ms-space-1);
}

.stat-value {
  font-size: 24px;
  font-weight: var(--ms-font-bold);
  color: var(--ms-text-primary);
}

.stat-sub {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-top: 2px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ms-space-4);
  flex-wrap: wrap;
}

.tabs {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tab-btn {
  padding: 8px 16px;
  border-radius: var(--ms-radius-md);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-btn:hover {
  color: var(--ms-text-primary);
  background-color: var(--ms-surface-hover);
}

.tab-btn.active {
  color: white;
  background: var(--ms-gradient-primary);
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
}

.search-input {
  width: 240px;
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
  gap: var(--ms-space-4);
  cursor: pointer;
  transition: all 0.2s ease;
}

.budget-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--ms-shadow-md);
  border-color: var(--ms-border-default);
}

.budget-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--ms-space-2);
}

.budget-category {
  display: flex;
  align-items: flex-start;
  gap: var(--ms-space-3);
  min-width: 0;
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

.category-icon.small {
  width: 32px;
  height: 32px;
  font-size: 14px;
}

.budget-name {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.budget-type {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-secondary);
  margin-top: 2px;
}

.budget-range {
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

.budget-progress {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-2);
}

.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: var(--ms-text-xs);
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
  align-items: center;
  justify-content: space-between;
  padding-top: var(--ms-space-2);
  border-top: 1px solid var(--ms-border-subtle);
}

.spent-text {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.spent-text strong {
  color: var(--ms-text-primary);
}

.budget-card-actions {
  display: flex;
  gap: var(--ms-space-1);
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
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-5);
}

.detail-header {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
}

.detail-header .budget-name {
  font-size: var(--ms-text-lg);
}

.detail-progress-card {
  background: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-5);
}

.detail-amounts {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: var(--ms-space-4);
}

.detail-amount-label {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-bottom: 4px;
}

.detail-amount-value {
  font-size: 28px;
  font-weight: var(--ms-font-bold);
  color: var(--ms-text-primary);
}

.detail-amount-value-spent {
  font-size: 20px;
  font-weight: var(--ms-font-bold);
}

.detail-progress-footer {
  display: flex;
  justify-content: space-between;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-secondary);
  margin-top: var(--ms-space-2);
}

.detail-section {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-3);
}

.detail-section-title {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.event-time-card {
  background: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4);
}

.event-time-range {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-primary);
  font-weight: var(--ms-font-medium);
}

.event-countdown {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-top: 4px;
}

.event-status-tag {
  margin-top: var(--ms-space-2);
}

.category-breakdown {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-3);
}

.breakdown-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.breakdown-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--ms-text-sm);
}

.breakdown-icon {
  font-size: 16px;
}

.breakdown-name {
  color: var(--ms-text-secondary);
  flex: 1;
}

.breakdown-amount {
  color: var(--ms-text-primary);
  font-weight: var(--ms-font-medium);
}

.transaction-list {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-2);
}

.transaction-list.timeline {
  position: relative;
  padding-left: var(--ms-space-4);
}

.transaction-list.timeline::before {
  content: '';
  position: absolute;
  left: 7px;
  top: 8px;
  bottom: 8px;
  width: 1px;
  background: var(--ms-border-default);
}

.transaction-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-3);
  border-radius: var(--ms-radius-lg);
  background: var(--ms-surface-secondary);
}

.transaction-list.timeline .transaction-item {
  position: relative;
}

.transaction-list.timeline .transaction-item::before {
  content: '';
  position: absolute;
  left: -21px;
  top: 18px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ms-border-default);
}

.transaction-left {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
}

.transaction-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.transaction-desc {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-primary);
}

.transaction-date {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.transaction-amount {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  font-variant-numeric: tabular-nums;
}

.transaction-amount.expense {
  color: var(--ms-expense);
}

.empty-hint {
  text-align: center;
  padding: var(--ms-space-6);
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-sm);
  background: var(--ms-surface-secondary);
  border-radius: var(--ms-radius-lg);
}

.text-right {
  text-align: right;
}

@media (max-width: 1280px) {
  .budget-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 1024px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .toolbar {
    flex-direction: column;
    align-items: stretch;
  }

  .tabs {
    overflow-x: auto;
    padding-bottom: 4px;
  }

  .toolbar-right {
    justify-content: space-between;
  }
}

@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }

  .budget-grid {
    grid-template-columns: 1fr;
  }

  .search-input {
    width: 100%;
  }
}
</style>
