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
              :key="budget.budget.id"
              class="budget-item"
            >
              <div class="budget-header">
                <div class="budget-category">
                  <span class="category-icon" :style="{ color: budget.category_color }">
                    {{ budget.category_icon || '💰' }}
                  </span>
                  {{ budget.category_name }}
                </div>
                <div class="budget-amount">
                  ¥{{ formatAmount(budget.spent) }} / ¥{{ formatAmount(budget.budget.amount) }}
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
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) => {
        let result = `${params[0].axisValue}<br/>`
        params.forEach((param: any) => {
          result += `${param.marker}${param.seriesName}: ¥${formatAmount(param.value)}<br/>`
        })
        return result
      }
    },
    legend: {
      data: ['收入', '支出']
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: data.map(item => item.month).reverse()
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        formatter: (value: number) => `¥${formatAmount(value)}`
      }
    },
    series: [
      {
        name: '收入',
        type: 'line',
        data: data.map(item => item.income).reverse(),
        itemStyle: { color: '#67C23A' },
        areaStyle: { opacity: 0.3, color: '#67C23A' }
      },
      {
        name: '支出',
        type: 'line',
        data: data.map(item => item.expense).reverse(),
        itemStyle: { color: '#F56C6C' },
        areaStyle: { opacity: 0.3, color: '#F56C6C' }
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
  
  return {
    tooltip: {
      trigger: 'item',
      formatter: '{a} <br/>{b}: ¥{c} ({d}%)'
    },
    legend: {
      orient: 'vertical',
      left: 'left'
    },
    series: [
      {
        name: '支出分布',
        type: 'pie',
        radius: ['40%', '70%'],
        avoidLabelOverlap: false,
        label: {
          show: false,
          position: 'center'
        },
        emphasis: {
          label: {
            show: true,
            fontSize: '16',
            fontWeight: 'bold'
          }
        },
        labelLine: {
          show: false
        },
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
  if (percentage < 60) return '#67C23A'
  if (percentage < 80) return '#E6A23C'
  return '#F56C6C'
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
  width: 100%;
}

.stats-cards {
  margin-bottom: 24px;
}

.stat-card {
  border: none;
  border-radius: 12px;
  overflow: hidden;
  transition: all 0.3s ease;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.15);
}

.stat-content {
  display: flex;
  align-items: center;
  padding: 8px 0;
}

.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  margin-right: 16px;
}

.income-card .stat-icon {
  background: linear-gradient(135deg, #67C23A, #85CE61);
  color: white;
}

.expense-card .stat-icon {
  background: linear-gradient(135deg, #F56C6C, #F78989);
  color: white;
}

.balance-card .stat-icon {
  background: linear-gradient(135deg, #409EFF, #66B1FF);
  color: white;
}

.transactions-card .stat-icon {
  background: linear-gradient(135deg, #E6A23C, #EEBE77);
  color: white;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.stat-value.negative {
  color: #F56C6C;
}

.charts-section {
  margin-bottom: 24px;
}

.chart-card {
  height: 400px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-container {
  height: 320px;
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
  height: 500px;
}

.transactions-list,
.budget-list {
  max-height: 400px;
  overflow-y: auto;
}

.transaction-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid #f0f0f0;
}

.transaction-item:last-child {
  border-bottom: none;
}

.transaction-left {
  display: flex;
  align-items: center;
}

.transaction-icon {
  font-size: 20px;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(64, 158, 255, 0.1);
  margin-right: 12px;
}

.transaction-info {
  flex: 1;
}

.transaction-desc {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
  margin-bottom: 2px;
}

.transaction-date {
  font-size: 12px;
  color: #909399;
}

.transaction-amount {
  font-size: 16px;
  font-weight: 600;
}

.transaction-amount.income {
  color: #67C23A;
}

.transaction-amount.expense {
  color: #F56C6C;
}

.budget-item {
  margin-bottom: 20px;
  padding: 16px;
  background: #f8f9fa;
  border-radius: 8px;
}

.budget-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.budget-category {
  display: flex;
  align-items: center;
  font-weight: 500;
}

.category-icon {
  margin-right: 8px;
  font-size: 18px;
}

.budget-amount {
  font-size: 14px;
  color: #606266;
}

.budget-progress-bar {
  margin-bottom: 8px;
}

.budget-status {
  font-size: 12px;
}

.status-good {
  color: #67C23A;
}

.status-warning {
  color: #E6A23C;
}

.status-danger {
  color: #F56C6C;
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
  
  .stat-icon {
    margin-right: 0;
    margin-bottom: 8px;
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
    min-height: 300px;
  }
}
</style> 