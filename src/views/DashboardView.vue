<template>
  <div class="dashboard">
    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-cards">
      <el-col :xs="24" :sm="12" :md="8" :lg="6">
        <el-card class="stat-card income-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月收入</div>
              <div class="stat-value">¥{{ formatAmount(store.currentMonthIncome) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :xs="24" :sm="12" :md="8" :lg="6">
        <el-card class="stat-card expense-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon><Minus /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月支出</div>
              <div class="stat-value">¥{{ formatAmount(store.currentMonthExpense) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :xs="24" :sm="12" :md="8" :lg="6">
        <el-card class="stat-card balance-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon><Wallet /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月结余</div>
              <div class="stat-value" :class="{ 'negative': store.currentMonthBalance < 0 }">
                ¥{{ formatAmount(store.currentMonthBalance) }}
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
      
      <el-col :xs="24" :sm="12" :md="8" :lg="6">
        <el-card class="stat-card transactions-card">
          <div class="stat-content">
            <div class="stat-icon">
              <el-icon><List /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">交易笔数</div>
              <div class="stat-value">{{ store.transactions.length }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
    
    <!-- 图表区域 -->
    <el-row :gutter="20" class="charts-section">
      <el-col :xs="24" :lg="12">
        <el-card class="chart-card">
          <template #header>
            <div class="card-header">
              <span>收支趋势</span>
              <el-select v-model="trendPeriod" size="small" @change="updateTrendChart">
                <el-option label="近3个月" value="3"></el-option>
                <el-option label="近6个月" value="6"></el-option>
                <el-option label="近12个月" value="12"></el-option>
              </el-select>
            </div>
          </template>
          <div class="chart-container">
            <v-chart 
              ref="trendChartRef"
              :option="trendChartOption" 
              class="trend-chart"
              :loading="store.loading"
            />
          </div>
        </el-card>
      </el-col>
      
      <el-col :xs="24" :lg="12">
        <el-card class="chart-card">
          <template #header>
            <div class="card-header">
              <span>支出分布</span>
              <el-date-picker
                v-model="currentMonth"
                type="month"
                size="small"
                format="YYYY-MM"
                value-format="YYYY-MM"
                @change="updateCategoryChart"
              />
            </div>
          </template>
          <div class="chart-container">
            <v-chart 
              ref="pieChartRef"
              :option="pieChartOption" 
              class="pie-chart"
              :loading="store.loading"
            />
          </div>
        </el-card>
      </el-col>
    </el-row>
    
    <!-- 底部区域 -->
    <el-row :gutter="20" class="bottom-section">
      <!-- 最近交易 -->
      <el-col :xs="24" :lg="14">
        <el-card class="recent-transactions">
          <template #header>
            <div class="card-header">
              <span>最近交易</span>
              <el-button size="small" @click="goToTransactions">查看全部</el-button>
            </div>
          </template>
          <div class="transactions-list">
            <div 
              v-for="transaction in recentTransactions" 
              :key="transaction.id"
              class="transaction-item"
            >
              <div class="transaction-left">
                <div class="transaction-icon" :style="{ color: transaction.category_color }">
                  {{ transaction.category_icon || '💰' }}
                </div>
                <div class="transaction-info">
                  <div class="transaction-desc">{{ transaction.description || transaction.category_name }}</div>
                  <div class="transaction-date">{{ formatDate(transaction.date) }}</div>
                </div>
              </div>
              <div class="transaction-amount" :class="transaction.type">
                {{ transaction.type === 'income' ? '+' : '-' }}¥{{ formatAmount(transaction.amount) }}
              </div>
            </div>
            
            <div v-if="recentTransactions.length === 0" class="empty-state">
              <el-empty description="暂无交易记录" />
            </div>
          </div>
        </el-card>
      </el-col>
      
      <!-- 预算进度 -->
      <el-col :xs="24" :lg="10">
        <el-card class="budget-progress">
          <template #header>
            <div class="card-header">
              <span>预算执行</span>
              <el-button size="small" @click="goToBudget">管理预算</el-button>
            </div>
          </template>
          <div class="budget-list">
            <div 
              v-for="budget in activeBudgets" 
              :key="budget.id"
              class="budget-item"
            >
              <div class="budget-header">
                <div class="budget-category">
                  <span class="category-icon" :style="{ color: budget.category_color }">
                    {{ budget.category_icon || '💰' }}
                  </span>
                  {{ budget.name }}
                </div>
                <div class="budget-amount">
                  ¥{{ formatAmount(budget.spent) }} / ¥{{ formatAmount(budget.amount) }}
                </div>
              </div>
              <div class="budget-progress-bar">
                <el-progress 
                  :percentage="Math.min(budget.percentage, 100)" 
                  :color="getProgressColor(budget.percentage)"
                  :show-text="false"
                />
              </div>
              <div class="budget-status">
                <span v-if="budget.percentage < 80" class="status-good">
                  还可支出 ¥{{ formatAmount(budget.remaining) }}
                </span>
                <span v-else-if="budget.percentage < 100" class="status-warning">
                  接近预算上限
                </span>
                <span v-else class="status-danger">
                  已超支 ¥{{ formatAmount(Math.abs(budget.remaining)) }}
                </span>
              </div>
            </div>
            
            <div v-if="activeBudgets.length === 0" class="empty-state">
              <el-empty description="暂无预算设置" />
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart, PieChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
} from 'echarts/components'
import VChart from 'vue-echarts'
import { useAppStore } from '../stores'
import dayjs from 'dayjs'

// 注册ECharts组件
use([
  CanvasRenderer,
  LineChart,
  PieChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
])

const store = useAppStore()
const trendPeriod = ref('6')
const currentMonth = ref(dayjs().format('YYYY-MM'))
const trendChartRef = ref()
const pieChartRef = ref()

// 计算属性
const recentTransactions = computed(() => {
  return store.transactions.slice(0, 8)
})

const activeBudgets = computed(() => {
  return store.budgets.slice(0, 5)
})

// 趋势图配置
const trendChartOption = computed(() => {
  const data = store.monthlyStats.slice(0, parseInt(trendPeriod.value))
  return {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#1a1a28',
      borderColor: 'rgba(255,255,255,0.1)',
      borderRadius: 10,
      textStyle: {
        color: '#e2e8f0',
        fontSize: 13
      },
      formatter: (params: any) => {
        let result = `<div style="font-weight:600;margin-bottom:6px;color:#94a3b8">${params[0].axisValue}</div>`
        params.forEach((param: any) => {
          result += `<div style="display:flex;align-items:center;gap:6px;padding:2px 0">${param.marker}<span style="color:#94a3b8">${param.seriesName}</span><span style="font-weight:600;margin-left:auto">¥${formatAmount(param.value)}</span></div>`
        })
        return result
      }
    },
    legend: {
      data: ['收入', '支出'],
      textStyle: {
        color: '#64748b',
        fontSize: 13
      },
      itemGap: 20
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '12%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: data.map(item => item.month).reverse(),
      axisLine: { lineStyle: { color: 'rgba(255,255,255,0.06)' } },
      axisTick: { show: false },
      axisLabel: { color: '#475569', fontSize: 12 }
    },
    yAxis: {
      type: 'value',
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: '#475569',
        fontSize: 12,
        formatter: (value: number) => `¥${formatAmount(value)}`
      },
      splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)', type: 'dashed' } }
    },
    series: [
      {
        name: '收入',
        type: 'line',
        data: data.map(item => item.income).reverse(),
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        itemStyle: { color: '#10b981' },
        lineStyle: { color: '#10b981', width: 2.5 },
        areaStyle: {
          color: {
            type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(16,185,129,0.25)' },
              { offset: 1, color: 'rgba(16,185,129,0.02)' }
            ]
          }
        }
      },
      {
        name: '支出',
        type: 'line',
        data: data.map(item => item.expense).reverse(),
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        itemStyle: { color: '#f87171' },
        lineStyle: { color: '#f87171', width: 2.5 },
        areaStyle: {
          color: {
            type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(248,113,113,0.25)' },
              { offset: 1, color: 'rgba(248,113,113,0.02)' }
            ]
          }
        }
      }
    ]
  }
})

// 饼图配置
const pieChartOption = computed(() => {
  const data = store.categoryStats.map(item => ({
    name: item.category_name,
    value: item.amount
  }))
  
  const palette = ['#6366f1', '#8b5cf6', '#06b6d4', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#84cc16']

  return {
    backgroundColor: 'transparent',
    color: palette,
    tooltip: {
      trigger: 'item',
      backgroundColor: '#1a1a28',
      borderColor: 'rgba(255,255,255,0.1)',
      borderRadius: 10,
      textStyle: {
        color: '#e2e8f0',
        fontSize: 13
      },
      formatter: (params: any) => {
        return `<div style="font-weight:600;margin-bottom:4px">${params.name}</div><div>¥${formatAmount(params.value)} <span style="color:#64748b">(${params.percent}%)</span></div>`
      }
    },
    legend: {
      orient: 'vertical',
      left: 'left',
      textStyle: { color: '#64748b', fontSize: 12 },
      itemWidth: 10,
      itemHeight: 10,
      borderRadius: 5
    },
    series: [
      {
        name: '支出分布',
        type: 'pie',
        radius: ['42%', '68%'],
        center: ['60%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderColor: '#151520',
          borderWidth: 3,
          borderRadius: 6
        },
        label: {
          show: false,
          position: 'center',
          color: '#e2e8f0'
        },
        emphasis: {
          label: {
            show: true,
            fontSize: 15,
            fontWeight: 'bold',
            color: '#e2e8f0'
          },
          itemStyle: {
            shadowBlur: 20,
            shadowColor: 'rgba(0,0,0,0.3)'
          }
        },
        labelLine: { show: false },
        data: data
      }
    ]
  }
})

// 方法
const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const formatDate = (dateStr: string) => {
  return dayjs(dateStr).format('MM-DD')
}

const getProgressColor = (percentage: number) => {
  if (percentage < 60) return '#10b981'
  if (percentage < 80) return '#f59e0b'
  return '#ef4444'
}

const updateTrendChart = async () => {
  await store.fetchMonthlyStats(parseInt(trendPeriod.value))
}

const updateCategoryChart = async () => {
  const startDate = dayjs(currentMonth.value).startOf('month').format('YYYY-MM-DD')
  const endDate = dayjs(currentMonth.value).endOf('month').format('YYYY-MM-DD')
  await store.fetchCategoryStats(startDate, endDate, 'expense')
}

const goToTransactions = () => {
  // 导航到交易记录页面
  // 这里可以通过emit或路由跳转实现
}

const goToBudget = () => {
  // 导航到预算管理页面
}

onMounted(async () => {
  // 初始化图表数据
  await updateCategoryChart()
  
  // 确保图表正确渲染
  nextTick(() => {
    if (trendChartRef.value) {
      trendChartRef.value.resize()
    }
    if (pieChartRef.value) {
      pieChartRef.value.resize()
    }
  })
})
</script>

<style scoped>
.dashboard {
  color: #e2e8f0;
}

.stats-cards {
  margin-bottom: 24px;
}

.stat-card {
  border-radius: 16px;
  overflow: hidden;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  border: 1px solid rgba(255, 255, 255, 0.07);
  cursor: default;
  position: relative;
}

.stat-card::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 16px;
  opacity: 0;
  transition: opacity 0.25s;
}

.stat-card:hover {
  transform: translateY(-3px);
  border-color: rgba(255, 255, 255, 0.12);
}

.stat-card:hover::before {
  opacity: 1;
}

.income-card {
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.1) 0%, rgba(16, 185, 129, 0.03) 100%);
  border-color: rgba(16, 185, 129, 0.2) !important;
}

.income-card:hover {
  box-shadow: 0 8px 32px rgba(16, 185, 129, 0.15);
}

.expense-card {
  background: linear-gradient(135deg, rgba(248, 113, 113, 0.1) 0%, rgba(248, 113, 113, 0.03) 100%);
  border-color: rgba(248, 113, 113, 0.2) !important;
}

.expense-card:hover {
  box-shadow: 0 8px 32px rgba(248, 113, 113, 0.15);
}

.balance-card {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.12) 0%, rgba(99, 102, 241, 0.04) 100%);
  border-color: rgba(99, 102, 241, 0.25) !important;
}

.balance-card:hover {
  box-shadow: 0 8px 32px rgba(99, 102, 241, 0.2);
}

.transactions-card {
  background: linear-gradient(135deg, rgba(251, 191, 36, 0.1) 0%, rgba(251, 191, 36, 0.03) 100%);
  border-color: rgba(251, 191, 36, 0.2) !important;
}

.transactions-card:hover {
  box-shadow: 0 8px 32px rgba(251, 191, 36, 0.15);
}

.stat-content {
  display: flex;
  align-items: center;
  padding: 6px 0;
  gap: 16px;
}

.stat-icon {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}

.income-card .stat-icon {
  background: linear-gradient(135deg, #10b981, #34d399);
  color: white;
  box-shadow: 0 4px 14px rgba(16, 185, 129, 0.35);
}

.expense-card .stat-icon {
  background: linear-gradient(135deg, #ef4444, #f87171);
  color: white;
  box-shadow: 0 4px 14px rgba(239, 68, 68, 0.35);
}

.balance-card .stat-icon {
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  color: white;
  box-shadow: 0 4px 14px rgba(99, 102, 241, 0.35);
}

.transactions-card .stat-icon {
  background: linear-gradient(135deg, #f59e0b, #fbbf24);
  color: white;
  box-shadow: 0 4px 14px rgba(245, 158, 11, 0.35);
}

.stat-info {
  flex: 1;
  min-width: 0;
}

.stat-label {
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  margin-bottom: 6px;
}

.income-card .stat-label { color: rgba(16, 185, 129, 0.8); }
.expense-card .stat-label { color: rgba(248, 113, 113, 0.8); }
.balance-card .stat-label { color: rgba(165, 180, 252, 0.8); }
.transactions-card .stat-label { color: rgba(251, 191, 36, 0.8); }

.stat-value {
  font-size: 26px;
  font-weight: 700;
  color: #f1f5f9;
  letter-spacing: -0.5px;
  line-height: 1;
}

.stat-value.negative {
  color: #f87171;
}

.charts-section {
  margin-bottom: 24px;
}

.chart-card {
  height: 380px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: #e2e8f0;
  font-size: 15px;
  font-weight: 600;
}

.chart-container {
  height: 295px;
}

.trend-chart,
.pie-chart {
  width: 100%;
  height: 100%;
}

.bottom-section {
  margin-bottom: 24px;
}

.recent-transactions,
.budget-progress {
  height: 460px;
}

.transactions-list,
.budget-list {
  max-height: 370px;
  overflow-y: auto;
}

.transaction-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 11px 4px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  transition: background 0.15s;
  border-radius: 8px;
  margin: 0 -4px;
}

.transaction-item:last-child {
  border-bottom: none;
}

.transaction-item:hover {
  background: rgba(255, 255, 255, 0.03);
}

.transaction-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.transaction-icon {
  font-size: 18px;
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(99, 102, 241, 0.12);
  flex-shrink: 0;
}

.transaction-info {
  flex: 1;
}

.transaction-desc {
  font-size: 14px;
  font-weight: 500;
  color: #cbd5e1;
  margin-bottom: 2px;
  line-height: 1.3;
}

.transaction-date {
  font-size: 12px;
  color: #475569;
}

.transaction-amount {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.2px;
}

.transaction-amount.income {
  color: #34d399;
}

.transaction-amount.expense {
  color: #f87171;
}

.budget-item {
  margin-bottom: 16px;
  padding: 14px 16px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  transition: all 0.2s;
}

.budget-item:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.1);
}

.budget-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.budget-category {
  display: flex;
  align-items: center;
  font-weight: 600;
  font-size: 14px;
  color: #e2e8f0;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
  line-height: 1;
}

.budget-amount {
  font-size: 13px;
  color: #64748b;
  font-weight: 500;
}

.budget-progress-bar {
  margin-bottom: 8px;
}

.budget-status {
  font-size: 12px;
  font-weight: 500;
}

.status-good {
  color: #34d399;
}

.status-warning {
  color: #fbbf24;
}

.status-danger {
  color: #f87171;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 200px;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .stat-content {
    flex-direction: column;
    text-align: center;
  }
  
  .chart-card {
    height: 300px;
  }
  
  .chart-container {
    height: 220px;
  }
  
  .recent-transactions,
  .budget-progress {
    height: auto;
  }
  
  .transactions-list,
  .budget-list {
    max-height: 300px;
  }
}
</style> 