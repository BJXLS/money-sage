<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart, PieChart } from 'echarts/charts'
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  DatasetComponent,
} from 'echarts/components'
import VChart from 'vue-echarts'
import { useAppStore } from '../stores'
import MoneyCard from '../components/ui/MoneyCard.vue'
import MoneyStat from '../components/ui/MoneyStat.vue'
import MoneyProgress from '../components/ui/MoneyProgress.vue'
import { useTheme } from '../composables/useTheme'
import dayjs from 'dayjs'

use([
  CanvasRenderer,
  LineChart,
  PieChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  DatasetComponent,
])

const store = useAppStore()
const { isDark } = useTheme()

const trendPeriod = ref(6)
const currentMonth = ref(dayjs().format('YYYY-MM'))

const netWorthTrend = computed(() => {
  const current = store.currentMonthBalance
  const last = current * 0.92 // mock for prototype; replace with real data
  return Number((((current - last) / last) * 100).toFixed(1))
})

const recentTransactions = computed(() => {
  return store.transactions.slice(0, 8)
})

const activeBudgets = computed(() => {
  return store.budgets.slice(0, 5)
})

const budgetAlertStats = computed(() => {
  const overBudgetCount = store.budgets.filter(b => b.percentage >= 100).length
  const warningCount = store.budgets.filter(b => b.percentage >= 80 && b.percentage < 100).length
  return { overBudgetCount, warningCount }
})

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const formatDate = (date: string) => {
  return dayjs(date).format('M月D日')
}

const getProgressColor = (percentage: number) => {
  if (percentage >= 100) return 'danger'
  if (percentage >= 80) return 'warning'
  return 'primary'
}

// Trend chart
const trendChartOption = computed(() => {
  const months = store.monthlyStats.slice(-trendPeriod.value)
  const textColor = isDark() ? '#94a3b8' : '#475569'
  const gridColor = isDark() ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)'

  return {
    tooltip: {
      trigger: 'axis',
      backgroundColor: isDark() ? '#1a1a28' : '#ffffff',
      borderColor: isDark() ? 'rgba(255,255,255,0.1)' : '#e2e8f0',
      textStyle: { color: isDark() ? '#f8fafc' : '#0f172a' },
      formatter: (params: any) => {
        let result = `<div style="font-weight:600;margin-bottom:4px">${params[0].axisValue}</div>`
        params.forEach((item: any) => {
          result += `<div style="display:flex;align-items:center;gap:8px">
            <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${item.color}"></span>
            <span>${item.seriesName}: ¥${Number(item.value).toLocaleString()}</span>
          </div>`
        })
        return result
      },
    },
    legend: {
      data: ['收入', '支出'],
      right: 0,
      top: 0,
      textStyle: { color: textColor },
      itemWidth: 8,
      itemHeight: 8,
    },
    grid: { left: 16, right: 16, top: 40, bottom: 16, containLabel: true },
    xAxis: {
      type: 'category',
      data: months.map(m => m.month),
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: textColor },
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: textColor,
        formatter: (value: number) => `¥${(value / 1000).toFixed(0)}k`,
      },
      splitLine: { lineStyle: { color: gridColor } },
    },
    series: [
      {
        name: '收入',
        type: 'line',
        smooth: true,
        data: months.map(m => m.income),
        itemStyle: { color: '#10b981' },
        areaStyle: {
          color: {
            type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(16, 185, 129, 0.25)' },
              { offset: 1, color: 'rgba(16, 185, 129, 0.02)' },
            ],
          },
        },
      },
      {
        name: '支出',
        type: 'line',
        smooth: true,
        data: months.map(m => m.expense),
        itemStyle: { color: '#f43f5e' },
        areaStyle: {
          color: {
            type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(244, 63, 94, 0.25)' },
              { offset: 1, color: 'rgba(244, 63, 94, 0.02)' },
            ],
          },
        },
      },
    ],
  }
})

// Pie chart
const pieData = computed(() => {
  return store.categoryStats
    .filter(item => item.amount > 0)
    .sort((a, b) => b.amount - a.amount)
    .slice(0, 6)
})

const pieColors = ['#f43f5e', '#8b5cf6', '#f59e0b', '#3b82f6', '#10b981', '#94a3b8']

const pieChartOption = computed(() => {
  return {
    tooltip: {
      trigger: 'item',
      backgroundColor: isDark() ? '#1a1a28' : '#ffffff',
      borderColor: isDark() ? 'rgba(255,255,255,0.1)' : '#e2e8f0',
      textStyle: { color: isDark() ? '#f8fafc' : '#0f172a' },
      formatter: (params: any) => `${params.name}: ¥${Number(params.value).toLocaleString()} (${params.percent}%)`,
    },
    legend: { show: false },
    series: [
      {
        type: 'pie',
        radius: ['65%', '85%'],
        center: ['50%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 4, borderColor: isDark() ? '#151520' : '#ffffff', borderWidth: 2 },
        label: { show: false },
        emphasis: { label: { show: false } },
        data: pieData.value.map((item, index) => ({
          value: item.amount,
          name: item.category_name,
          itemStyle: { color: pieColors[index % pieColors.length] },
        })),
      },
    ],
  }
})

const healthScore = computed(() => {
  const balance = store.currentMonthBalance
  const income = store.currentMonthIncome
  if (income <= 0) return 70
  const rate = balance / income
  if (rate >= 0.3) return 90
  if (rate >= 0.2) return 80
  if (rate >= 0.1) return 70
  return 60
})

onMounted(() => {
  store.fetchMonthlyStats(12)
  store.fetchCategoryStats(
    dayjs().startOf('month').format('YYYY-MM-DD'),
    dayjs().endOf('month').format('YYYY-MM-DD'),
    'expense'
  )
})

watch(currentMonth, (month) => {
  const start = dayjs(month).startOf('month').format('YYYY-MM-DD')
  const end = dayjs(month).endOf('month').format('YYYY-MM-DD')
  store.fetchCategoryStats(start, end, 'expense')
})
</script>

<template>
  <div class="dashboard-view">
    <!-- Net Worth Overview -->
    <MoneyCard no-padding>
      <div class="net-worth">
        <div class="net-worth-bg"></div>
        <div class="net-worth-content">
          <div class="text-sm text-secondary mb-1">本月净资产</div>
          <div class="flex items-baseline gap-3 flex-wrap">
            <span class="net-worth-value">¥ {{ formatAmount(store.currentMonthBalance) }}</span>
            <span class="net-worth-trend" :class="netWorthTrend >= 0 ? 'up' : 'down'">
              {{ netWorthTrend >= 0 ? '↑' : '↓' }} {{ Math.abs(netWorthTrend) }}%
            </span>
            <span class="text-sm text-tertiary">较上月</span>
          </div>
          <div class="net-worth-summary">
            <div class="summary-item">
              <span class="dot income"></span>
              <span class="text-secondary">本月收入</span>
              <span class="font-semibold text-primary">¥ {{ formatAmount(store.currentMonthIncome) }}</span>
            </div>
            <div class="summary-item">
              <span class="dot expense"></span>
              <span class="text-secondary">本月支出</span>
              <span class="font-semibold text-primary">¥ {{ formatAmount(store.currentMonthExpense) }}</span>
            </div>
          </div>
        </div>
      </div>
    </MoneyCard>

    <!-- Stats Cards -->
    <div class="stats-grid">
      <MoneyStat
        label="本月收入"
        :value="`¥ ${formatAmount(store.currentMonthIncome)}`"
        :trend="12.3"
        type="income"
      />
      <MoneyStat
        label="本月支出"
        :value="`¥ ${formatAmount(store.currentMonthExpense)}`"
        :trend="-5.7"
        type="expense"
      />
      <MoneyStat
        label="本月结余"
        :value="`¥ ${formatAmount(store.currentMonthBalance)}`"
        :trend="22.1"
        type="primary"
      />
    </div>

    <!-- Main Grid -->
    <div class="main-grid">
      <!-- Left Column -->
      <div class="left-column">
        <!-- Trend Chart -->
        <MoneyCard title="收支趋势">
          <template #header>
            <div class="trend-header">
              <h3>收支趋势</h3>
              <div class="trend-controls">
                <button
                  v-for="p in [3, 6, 12]"
                  :key="p"
                  class="period-btn"
                  :class="{ active: trendPeriod === p }"
                  @click="trendPeriod = p"
                >
                  近{{ p }}个月
                </button>
              </div>
            </div>
          </template>
          <div class="chart-container">
            <v-chart :option="trendChartOption" autoresize />
          </div>
        </MoneyCard>

        <!-- Recent Transactions -->
        <MoneyCard title="最近交易" header-action="查看全部" @action="$emit('navigate', 'transactions')">
          <div class="transactions-list">
            <div
              v-for="tx in recentTransactions"
              :key="tx.id"
              class="transaction-item"
            >
              <div class="transaction-left">
                <div class="category-icon" :style="{ backgroundColor: `${tx.category_color || '#6366f1'}20`, color: tx.category_color || '#6366f1' }">
                  {{ tx.category_icon || '💰' }}
                </div>
                <div class="transaction-info">
                  <div class="transaction-desc">{{ tx.description || tx.category_name }}</div>
                  <div class="transaction-date">{{ formatDate(tx.date) }}</div>
                </div>
              </div>
              <div class="transaction-amount" :class="tx.type">
                {{ tx.type === 'income' ? '+' : '-' }}¥{{ formatAmount(tx.amount) }}
              </div>
            </div>
            <div v-if="recentTransactions.length === 0" class="empty-hint">
              暂无交易记录
            </div>
          </div>
        </MoneyCard>
      </div>

      <!-- Right Column -->
      <div class="right-column">
        <!-- Budget Alerts -->
        <MoneyCard title="预算告警">
          <template #header>
            <div class="budget-header">
              <h3>预算告警</h3>
              <span
                class="alert-badge"
                :class="{
                  warning: budgetAlertStats.overBudgetCount === 0 && budgetAlertStats.warningCount > 0,
                  healthy: budgetAlertStats.overBudgetCount === 0 && budgetAlertStats.warningCount === 0,
                }"
              >
                <template v-if="budgetAlertStats.overBudgetCount > 0">{{ budgetAlertStats.overBudgetCount }} 个超支</template>
                <template v-if="budgetAlertStats.overBudgetCount > 0 && budgetAlertStats.warningCount > 0"> · </template>
                <template v-if="budgetAlertStats.warningCount > 0">{{ budgetAlertStats.warningCount }} 个预警</template>
                <template v-if="budgetAlertStats.overBudgetCount === 0 && budgetAlertStats.warningCount === 0">预算健康</template>
              </span>
            </div>
          </template>
          <div class="budget-list">
            <div v-for="budget in activeBudgets" :key="budget.id" class="budget-item">
              <div class="budget-info">
                <span class="text-secondary">{{ budget.name }}</span>
                <span class="font-medium text-primary">
                  ¥{{ formatAmount(budget.spent) }} / ¥{{ formatAmount(budget.amount) }}
                </span>
              </div>
              <MoneyProgress
                :percentage="budget.percentage"
                :color="getProgressColor(budget.percentage)"
                size="sm"
              />
              <div class="budget-status" :class="getProgressColor(budget.percentage)">
                <span v-if="budget.percentage >= 100">已超支 ¥{{ formatAmount(Math.abs(budget.remaining)) }}</span>
                <span v-else-if="budget.percentage >= 80">接近预算上限</span>
                <span v-else>还可支出 ¥{{ formatAmount(budget.remaining) }}</span>
              </div>
            </div>
            <div v-if="activeBudgets.length === 0" class="empty-hint">
              暂无预算设置
            </div>
          </div>
        </MoneyCard>

        <!-- Expense Distribution -->
        <MoneyCard title="支出分布">
          <div class="pie-chart-container">
            <v-chart :option="pieChartOption" autoresize />
          </div>
          <div class="pie-legend">
            <div v-for="(item, index) in pieData" :key="item.category_id" class="legend-item">
              <div class="legend-dot" :style="{ backgroundColor: pieColors[index % pieColors.length] }"></div>
              <span class="legend-name">{{ item.category_name }}</span>
              <span class="legend-value">{{ item.percentage.toFixed(0) }}%</span>
            </div>
          </div>
        </MoneyCard>

        <!-- Financial Health -->
        <MoneyCard title="财务健康度">
          <div class="health-container">
            <div class="health-ring">
              <div class="health-ring-inner"></div>
              <div class="health-score">{{ healthScore }}</div>
            </div>
            <div class="health-text">
              <div class="font-medium text-primary">财务状况良好</div>
              <div class="text-xs text-tertiary mt-1">储蓄率健康，预算控制需加强</div>
            </div>
          </div>
        </MoneyCard>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard-view {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.net-worth {
  position: relative;
  padding: 24px;
  overflow: hidden;
}

.net-worth-bg {
  position: absolute;
  top: 0;
  right: 0;
  width: 280px;
  height: 280px;
  border-radius: 50%;
  background: var(--ms-gradient-primary);
  opacity: 0.06;
  transform: translate(20%, -40%);
}

.net-worth-content {
  position: relative;
  z-index: 1;
}

.net-worth-value {
  font-size: 36px;
  font-weight: 700;
  color: var(--ms-text-primary);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}

.net-worth-trend {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
}

.net-worth-trend.up {
  background-color: rgba(16, 185, 129, 0.1);
  color: var(--ms-income);
}

.net-worth-trend.down {
  background-color: rgba(244, 63, 94, 0.1);
  color: var(--ms-expense);
}

.net-worth-summary {
  display: flex;
  gap: 24px;
  margin-top: 16px;
  flex-wrap: wrap;
}

.summary-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dot.income { background-color: var(--ms-income); }
.dot.expense { background-color: var(--ms-expense); }

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 24px;
}

.main-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 24px;
}

.left-column,
.right-column {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.trend-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.trend-header h3 {
  margin: 0;
  font-size: var(--ms-text-base);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.trend-controls {
  display: flex;
  gap: 4px;
}

.period-btn {
  padding: 4px 10px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  color: var(--ms-text-tertiary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.period-btn:hover {
  color: var(--ms-text-secondary);
  background-color: var(--ms-surface-hover);
}

.period-btn.active {
  color: white;
  background: var(--ms-gradient-primary);
}

.chart-container {
  height: 280px;
}

.pie-chart-container {
  height: 200px;
}

.transactions-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.transaction-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  border-radius: var(--ms-radius-lg);
  transition: background-color 0.15s ease;
}

.transaction-item:hover {
  background-color: var(--ms-surface-hover);
}

.transaction-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.transaction-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.transaction-desc {
  font-size: 14px;
  font-weight: 500;
  color: var(--ms-text-primary);
}

.transaction-date {
  font-size: 12px;
  color: var(--ms-text-tertiary);
}

.transaction-amount {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.transaction-amount.income {
  color: var(--ms-income);
}

.transaction-amount.expense {
  color: var(--ms-expense);
}

.budget-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.budget-header h3 {
  margin: 0;
  font-size: var(--ms-text-base);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.alert-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 500;
  background-color: rgba(244, 63, 94, 0.1);
  color: var(--ms-expense);
}

.alert-badge.warning {
  background-color: rgba(245, 158, 11, 0.1);
  color: var(--ms-warning);
}

.alert-badge.healthy {
  background-color: rgba(16, 185, 129, 0.1);
  color: var(--ms-income);
}

.budget-list {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.budget-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.budget-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
}

.budget-status {
  font-size: 12px;
}

.budget-status.primary { color: var(--ms-text-tertiary); }
.budget-status.warning { color: var(--ms-warning); }
.budget-status.danger { color: var(--ms-expense); }

.pie-legend {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 16px;
}

.legend-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.legend-name {
  color: var(--ms-text-secondary);
  flex: 1;
  margin-left: 8px;
}

.legend-value {
  font-weight: 600;
  color: var(--ms-text-primary);
}

.health-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 8px 0;
}

.health-ring {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  background: conic-gradient(var(--ms-gradient-primary) 0deg 288deg, var(--ms-bg-tertiary) 288deg 360deg);
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.health-ring-inner {
  position: absolute;
  width: 92px;
  height: 92px;
  border-radius: 50%;
  background-color: var(--ms-surface-primary);
}

.health-score {
  position: relative;
  z-index: 1;
  font-size: 28px;
  font-weight: 700;
  color: var(--ms-text-primary);
}

.health-text {
  text-align: center;
}

.empty-hint {
  text-align: center;
  padding: 24px;
  color: var(--ms-text-tertiary);
  font-size: 14px;
}

@media (max-width: 1024px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }

  .main-grid {
    grid-template-columns: 1fr;
  }
}
</style>
