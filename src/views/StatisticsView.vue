<template>
  <div class="statistics-view">
    <!-- 月份选择器 -->
    <div class="month-selector-container">
      <el-date-picker
        v-model="selectedMonth"
        type="month"
        placeholder="选择月份"
        value-format="YYYY-MM"
        @change="onMonthChange"
        class="month-selector"
        size="large"
      />
    </div>

    <!-- 第一区域：当月花费总结 -->
    <div class="monthly-summary-section">
      <h2 class="section-title">当月花费总结</h2>
      <div class="monthly-summary-subtitle">{{ selectedMonthText }}详情</div>
      
      <el-row :gutter="24" class="summary-row">
        <!-- 左侧：扇形图 -->
        <el-col :xs="24" :lg="12">
          <el-card class="pie-chart-card">
            <template #header>
              <div class="card-header">
                <span>支出分布</span>
              </div>
            </template>
            

            
            <div class="chart-container">
              <v-chart 
                :option="pieChartOption" 
                class="pie-chart"
                :loading="store.loading"
                autoresize
              />
            </div>
            
            <!-- 图例 -->
            <div class="chart-legend">
              <div 
                v-for="(item, index) in legendData" 
                :key="item.name"
                class="legend-item"
              >
                <div 
                  class="legend-color" 
                  :style="{ backgroundColor: getLegendColor(index) }"
                ></div>
                <span class="legend-name">{{ item.name }}</span>
                <span class="legend-value">{{ item.percentage }}%</span>
              </div>
            </div>
          </el-card>
        </el-col>
        
        <!-- 右侧：分类详情 -->
        <el-col :xs="24" :lg="12">
          <el-card class="category-details-card">
            <template #header>
              <div class="card-header">
                <span>分类详情</span>
              </div>
            </template>
            <div class="category-details">
              <div 
                v-for="(category, index) in categoryDetails" 
                :key="category.category_id"
                class="category-item"
              >
                <div class="category-info">
                  <div class="category-rank-wrapper" :class="getRankClass(index + 1)">
                    <span class="category-rank">
                      No.{{ index + 1 }}
                    </span>
                  </div>
                  <div class="category-content">
                    <div class="category-name">{{ category.category_name }}</div>
                    <div class="category-stats">
                      <span class="percentage">{{ category.percentage.toFixed(1) }}%</span>
                    </div>
                  </div>
                </div>
                <div class="category-amount">
                  <div class="amount">¥{{ formatAmount(category.amount) }}</div>
                  <div class="change" :class="getChangeClass(category.change)">
                    <el-icon v-if="category.change > 0"><CaretTop /></el-icon>
                    <el-icon v-else-if="category.change < 0"><CaretBottom /></el-icon>
                    <span>{{ Math.abs(category.change).toFixed(1) }}%</span>
                  </div>
                </div>
              </div>
              
              <div v-if="categoryDetails.length === 0" class="empty-state">
                <el-empty description="暂无支出数据" />
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </div>

    <!-- 第二区域：支出趋势 -->
    <div class="expense-trend-section">
      <h2 class="section-title">支出趋势</h2>
      <div class="section-subtitle">以{{ selectedMonthText }}为结束的过去12个月支出分析</div>
      
      <el-row class="trend-row">
        <el-col :span="24">
          <el-card class="trend-card">
            <template #header>
              <div class="card-header">
                <div class="header-left">
                  <span>支出趋势</span>
                  <el-button-group size="small" class="trend-tabs">
                    <el-button 
                      :type="trendType === 'expense' ? 'primary' : 'default'"
                      @click="trendType = 'expense'"
                    >
                      支出
                    </el-button>
                    <el-button 
                      :type="trendType === 'income' ? 'primary' : 'default'"
                      @click="trendType = 'income'"
                    >
                      收支对比
                    </el-button>
                  </el-button-group>
                </div>
                <div class="monthly-total">
                  {{ selectedMonthText }}支出：¥{{ formatAmount(selectedMonthExpense) }}
                </div>
              </div>
            </template>
            <div class="trend-chart-container">
              <v-chart 
                :option="trendChartOption" 
                class="trend-chart"
                :loading="store.loading"
                autoresize
              />
            </div>
          </el-card>
        </el-col>
      </el-row>
    </div>

    <!-- 第三区域：预算管理 -->
    <div class="budget-management-section">
      <h2 class="section-title">预算管理</h2>
      <div class="section-subtitle">{{ selectedMonthText }}预算执行情况</div>
      
      <el-row :gutter="24" class="budget-row">
        <!-- 本月预算概览 -->
        <el-col :xs="24" :lg="16">
          <el-card class="budget-overview-card">
            <template #header>
              <div class="card-header">
                <span>{{ selectedMonthText }}预算概览</span>
              </div>
            </template>
            <div class="budget-overview">
              <div class="budget-stats">
                <div class="stat-item">
                  <div class="stat-icon budget-icon">
                    <el-icon><Wallet /></el-icon>
                  </div>
                  <div class="stat-info">
                    <div class="stat-label">总预算</div>
                    <div class="stat-value">¥{{ formatAmount(totalBudget) }}</div>
                  </div>
                </div>
                
                <div class="stat-item">
                  <div class="stat-icon spent-icon">
                    <el-icon><Minus /></el-icon>
                  </div>
                  <div class="stat-info">
                    <div class="stat-label">已花费</div>
                    <div class="stat-value">¥{{ formatAmount(totalSpent) }}</div>
                  </div>
                </div>
                
                <div class="stat-item">
                  <div class="stat-icon remaining-icon">
                    <el-icon><Trophy /></el-icon>
                  </div>
                  <div class="stat-info">
                    <div class="stat-label">剩余预算</div>
                    <div class="stat-value">¥{{ formatAmount(totalRemaining) }}</div>
                  </div>
                </div>
              </div>
              
              <div class="progress-section">
                <div class="progress-header">
                  <span class="progress-label">预算使用进度</span>
                  <span class="progress-percentage">{{ budgetUsagePercentage.toFixed(0) }}%</span>
                </div>
                <el-progress 
                  :percentage="Math.min(budgetUsagePercentage, 100)" 
                  :color="getBudgetProgressColor(budgetUsagePercentage)"
                  :stroke-width="12"
                  class="budget-progress"
                />
                <div class="progress-info">
                  <span class="days-info">已过去 {{ daysUsed }} 天</span>
                  <span class="remaining-info">剩余 {{ daysRemaining }} 天</span>
                </div>
              </div>
            </div>
          </el-card>
        </el-col>
        
        <!-- 每日预算建议 -->
        <el-col :xs="24" :lg="8">
          <el-card class="daily-budget-card">
            <template #header>
              <div class="card-header">
                <span>每日预算建议</span>
              </div>
            </template>
            <div class="daily-budget">
              <div class="daily-amount">
                <div class="amount-label">剩余预算</div>
                <div class="amount-value">¥{{ formatAmount(totalRemaining) }}</div>
              </div>
              
              <div class="daily-suggestion">
                <div class="suggestion-item">
                  <span class="suggestion-label">剩余天数</span>
                  <span class="suggestion-value">{{ daysRemaining }}天</span>
                </div>
                <div class="suggestion-item">
                  <span class="suggestion-label">建议每日支出</span>
                  <span class="suggestion-value">¥{{ formatAmount(dailySuggestion) }}</span>
                </div>
              </div>
              
              <div class="budget-status">
                <div class="status-item" :class="{ 'status-warning': budgetUsagePercentage > 80 }">
                  <el-icon><InfoFilled /></el-icon>
                  <span v-if="budgetUsagePercentage <= 60" class="status-text">
                    预算控制良好
                  </span>
                  <span v-else-if="budgetUsagePercentage <= 80" class="status-text">
                    预算使用较快，请注意控制
                  </span>
                  <span v-else class="status-text">
                    预算紧张，建议减少支出
                  </span>
                </div>
                
                <div class="average-spending">
                  <span class="spending-label">平均日支出</span>
                  <span class="spending-value">¥{{ formatAmount(averageDailySpending) }}</span>
                </div>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </div>

    <!-- 第四区域：事件预算 -->
    <div class="event-budget-section">
      <h2 class="section-title">事件预算</h2>
      <div class="section-subtitle">特定目标的预算执行进度</div>
      
      <el-row class="event-budget-row">
        <el-col :span="24">
          <el-card class="event-budget-card">
            <template #header>
              <div class="card-header">
                <span>事件预算</span>
                <el-button size="small" type="primary" @click="showAddBudgetDialog">
                  <el-icon><Plus /></el-icon>
                  添加新预算
                </el-button>
              </div>
            </template>
            <div class="event-budget-list">
              <div 
                v-for="budget in store.eventBudgets" 
                :key="budget.id"
                class="event-budget-item"
              >
                <div class="budget-header">
                  <div class="budget-info">
                    <div class="budget-icon-wrapper">
                      <span class="budget-icon" :style="{ color: budget.category_color }">
                        {{ budget.category_icon || getEventIcon(budget.name) }}
                      </span>
                    </div>
                    <div class="budget-details">
                      <div class="budget-name">{{ budget.name }}</div>
                      <div class="budget-category">目标: {{ budget.category_name }}</div>
                      <div class="budget-date">{{ formatEventDate(budget.start_date, budget.end_date) }}</div>
                    </div>
                  </div>
                  <div class="budget-amount-info">
                    <div class="spent-amount">¥{{ formatAmount(budget.spent) }}</div>
                    <div class="total-amount">¥{{ formatAmount(budget.amount) }}</div>
                  </div>
                </div>
                
                <div class="budget-progress-section">
                  <div class="progress-info">
                    <span class="progress-label">{{ budget.percentage.toFixed(1) }}%</span>
                    <span class="remaining-amount">
                      剩余 ¥{{ formatAmount(budget.remaining) }}
                    </span>
                  </div>
                  <el-progress 
                    :percentage="Math.min(budget.percentage, 100)" 
                    :color="getEventProgressColor(budget.percentage)"
                    :stroke-width="8"
                  />
                </div>
                
                <div class="budget-status">
                  <div class="status-indicator" :class="getEventStatusClass(budget.percentage)">
                    <el-icon v-if="budget.percentage < 80"><Check /></el-icon>
                    <el-icon v-else-if="budget.percentage < 100"><Warning /></el-icon>
                    <el-icon v-else><Close /></el-icon>
                    <span class="status-text">
                      {{ getEventStatusText(budget.percentage) }}
                    </span>
                  </div>
                  <div class="progress-days" v-if="budget.end_date">
                    {{ getEventDaysText(budget.start_date, budget.end_date) }}
                  </div>
                </div>
              </div>
              
              <div v-if="store.eventBudgets.length === 0" class="empty-state">
                <el-empty description="暂无事件预算">
                  <el-button type="primary" @click="showAddBudgetDialog">
                    创建第一个事件预算
                  </el-button>
                </el-empty>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
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
import { 
  CaretTop, 
  CaretBottom, 
  Wallet, 
  Minus, 
  Trophy, 
  InfoFilled, 
  Plus,
  Check,
  Warning,
  Close
} from '@element-plus/icons-vue'
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
const trendType = ref<'expense' | 'income'>('expense')

// 月份选择器状态
const selectedMonth = ref(dayjs().format('YYYY-MM'))

// 计算属性
const currentMonthText = computed(() => {
  return dayjs().format('YYYY年M月')
})

const selectedMonthText = computed(() => {
  return dayjs(selectedMonth.value).format('YYYY年M月')
})

// 分类详情数据（模拟环比变化）
const categoryDetails = computed(() => {
  return store.categoryStats.map(category => ({
    ...category,
    change: Math.random() * 20 - 10 // 模拟-10%到+10%的变化
  }))
})

// 选中月份的收入支出
const selectedMonthIncome = computed(() => {
  const monthData = store.monthlyStats.find(stat => stat.month === selectedMonth.value)
  return monthData?.income || 0
})

const selectedMonthExpense = computed(() => {
  const monthData = store.monthlyStats.find(stat => stat.month === selectedMonth.value)
  return monthData?.expense || 0
})





// 总预算计算 - 根据选中月份
const totalBudget = computed(() => {
  return store.budgets
    .filter(b => b.budget_type === 'time' && b.period_type === 'monthly' && b.is_active)
    .reduce((sum, b) => sum + b.amount, 0)
})

const totalSpent = computed(() => {
  // 这里应该基于选中月份的实际支出，而不是预算的spent字段
  // 因为spent字段可能是累计值
  return selectedMonthExpense.value
})

const totalRemaining = computed(() => {
  return totalBudget.value - totalSpent.value
})

const budgetUsagePercentage = computed(() => {
  if (totalBudget.value === 0) return 0
  return (totalSpent.value / totalBudget.value) * 100
})

// 日期计算 - 基于选中月份
const daysUsed = computed(() => {
  const selectedDate = dayjs(selectedMonth.value)
  const now = dayjs()
  
  // 如果是当前月份，使用今天的日期
  if (selectedDate.isSame(now, 'month')) {
    return now.date()
  }
  // 如果是过去的月份，使用该月的总天数
  else if (selectedDate.isBefore(now, 'month')) {
    return selectedDate.daysInMonth()
  }
  // 如果是未来的月份，返回0
  else {
    return 0
  }
})

const daysRemaining = computed(() => {
  const selectedDate = dayjs(selectedMonth.value)
  const now = dayjs()
  
  // 只有当前月份才有剩余天数的概念
  if (selectedDate.isSame(now, 'month')) {
    return selectedDate.daysInMonth() - now.date()
  }
  return 0
})

const dailySuggestion = computed(() => {
  if (daysRemaining.value <= 0) return 0
  return totalRemaining.value / daysRemaining.value
})

const averageDailySpending = computed(() => {
  if (daysUsed.value === 0) return 0
  return totalSpent.value / daysUsed.value
})

// 图例数据
const legendData = computed(() => {
  return store.categoryStats.map(item => ({
    name: item.category_name,
    percentage: item.percentage.toFixed(1)
  }))
})

// 获取图例颜色
const getLegendColor = (index: number) => {
  const colors = ['#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7', '#DDA0DD', '#FFB347', '#87CEEB']
  return colors[index % colors.length]
}

// 扇形图配置
const pieChartOption = computed(() => {
  const data = store.categoryStats.map(item => ({
    name: item.category_name,
    value: item.amount
  }))
  
  const colors = ['#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7', '#DDA0DD', '#FFB347', '#87CEEB']
  
  return {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      backgroundColor: '#2a2a2a',
      borderColor: '#404040',
      textStyle: { color: '#ffffff' },
      formatter: '{a} <br/>{b}: ¥{c} ({d}%)'
    },
    legend: {
      show: false
    },
    color: colors,
    series: [{
      name: '支出分布',
      type: 'pie',
      radius: ['35%', '75%'],
      center: ['50%', '50%'],
      avoidLabelOverlap: false,
      itemStyle: {
        borderRadius: 6,
        borderColor: '#1a1a1a',
        borderWidth: 2
      },
      label: {
        show: false
      },
      emphasis: {
        label: {
          show: true,
          fontSize: '14',
          fontWeight: 'bold',
          color: '#ffffff'
        },
        itemStyle: {
          shadowBlur: 10,
          shadowOffsetX: 0,
          shadowColor: 'rgba(0, 0, 0, 0.5)'
        }
      },
      data: data
    }]
  }
})

// 趋势图配置
const trendChartOption = computed(() => {
  // 基于选中月份获取过去12个月的数据
  const selectedDate = dayjs(selectedMonth.value)
  
  // 生成过去12个月的月份列表
  const months = []
  for (let i = 11; i >= 0; i--) {
    months.push(selectedDate.subtract(i, 'month').format('YYYY-MM'))
  }
  
  // 过滤出这些月份的数据
  const data = months.map(month => {
    const monthData = store.monthlyStats.find(stat => stat.month === month)
    return monthData || {
      month: dayjs(month).format('MM月'),
      income: 0,
      expense: 0,
      balance: 0
    }
  })
  
  if (trendType.value === 'expense') {
    return {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'axis',
        backgroundColor: '#2a2a2a',
        borderColor: '#404040',
        textStyle: { color: '#ffffff' },
        formatter: (params: any) => {
          const param = params[0]
          return `${param.axisValue}<br/>支出: ¥${formatAmount(param.value)}`
        }
      },
      grid: {
        left: '5%',
        right: '5%',
        bottom: '10%',
        top: '5%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        data: data.map(item => typeof item.month === 'string' && item.month.includes('-') 
          ? dayjs(item.month).format('MM月') 
          : item.month),
        axisLine: { lineStyle: { color: '#404040' } },
        axisTick: { lineStyle: { color: '#404040' } },
        axisLabel: { color: '#b0b0b0' }
      },
      yAxis: {
        type: 'value',
        axisLine: { lineStyle: { color: '#404040' } },
        axisTick: { lineStyle: { color: '#404040' } },
        axisLabel: {
          color: '#b0b0b0',
          formatter: (value: number) => `¥${formatAmount(value)}`
        },
        splitLine: { lineStyle: { color: '#404040' } }
      },
      series: [{
        name: '支出',
        type: 'line',
        data: data.map(item => item.expense),
        smooth: true,
        lineStyle: {
          color: '#FF6B6B',
          width: 3
        },
        itemStyle: { color: '#FF6B6B' },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(255, 107, 107, 0.3)' },
              { offset: 1, color: 'rgba(255, 107, 107, 0.1)' }
            ]
          }
        }
      }]
    }
  } else {
    return {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'axis',
        backgroundColor: '#2a2a2a',
        borderColor: '#404040',
        textStyle: { color: '#ffffff' }
      },
      legend: {
        data: ['收入', '支出'],
        textStyle: { color: '#ffffff' }
      },
      grid: {
        left: '5%',
        right: '5%',
        bottom: '10%',
        top: '15%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        data: data.map(item => typeof item.month === 'string' && item.month.includes('-') 
          ? dayjs(item.month).format('MM月') 
          : item.month),
        axisLine: { lineStyle: { color: '#404040' } },
        axisTick: { lineStyle: { color: '#404040' } },
        axisLabel: { color: '#b0b0b0' }
      },
      yAxis: {
        type: 'value',
        axisLine: { lineStyle: { color: '#404040' } },
        axisTick: { lineStyle: { color: '#404040' } },
        axisLabel: {
          color: '#b0b0b0',
          formatter: (value: number) => `¥${formatAmount(value)}`
        },
        splitLine: { lineStyle: { color: '#404040' } }
      },
      series: [
        {
          name: '收入',
          type: 'line',
          data: data.map(item => item.income),
          smooth: true,
          lineStyle: { color: '#67C23A', width: 3 },
          itemStyle: { color: '#67C23A' },
          areaStyle: {
            color: {
              type: 'linear',
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(103, 194, 58, 0.3)' },
                { offset: 1, color: 'rgba(103, 194, 58, 0.1)' }
              ]
            }
          }
        },
        {
          name: '支出',
          type: 'line',
          data: data.map(item => item.expense),
          smooth: true,
          lineStyle: { color: '#F56C6C', width: 3 },
          itemStyle: { color: '#F56C6C' },
          areaStyle: {
            color: {
              type: 'linear',
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(245, 108, 108, 0.3)' },
                { offset: 1, color: 'rgba(245, 108, 108, 0.1)' }
              ]
            }
          }
        }
      ]
    }
  }
})

// 方法
const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', { 
    minimumFractionDigits: 2, 
    maximumFractionDigits: 2 
  })
}

const getCategoryIcon = (index: number) => {
  const icons = ['🍽️', '🚗', '🛍️', '🎮', '🏠', '💊', '📚', '✈️']
  return icons[index % icons.length]
}

const getChangeClass = (change: number) => {
  if (change > 0) return 'increase'
  if (change < 0) return 'decrease'
  return 'no-change'
}



const getRankClass = (rank: number) => {
  if (rank === 1) return 'rank-first'
  if (rank === 2) return 'rank-second'
  if (rank === 3) return 'rank-third'
  return 'rank-other'
}

const getBudgetProgressColor = (percentage: number) => {
  if (percentage < 60) return '#67C23A'
  if (percentage < 80) return '#E6A23C'
  return '#F56C6C'
}

const getEventIcon = (name: string) => {
  const eventIcons: { [key: string]: string } = {
    '春节旅行': '✈️',
    '新电脑': '💻',
    '聚会宴会': '🎉',
    '装修': '🏠',
    '学习': '📚'
  }
  
  for (const key in eventIcons) {
    if (name.includes(key)) {
      return eventIcons[key]
    }
  }
  return '🎯'
}

const formatEventDate = (startDate: string, endDate?: string) => {
  const start = dayjs(startDate)
  if (endDate) {
    const end = dayjs(endDate)
    return `${start.format('MM-DD')} 至 ${end.format('MM-DD')}`
  }
  return `从 ${start.format('MM-DD')} 开始`
}

const getEventProgressColor = (percentage: number) => {
  if (percentage < 70) return '#67C23A'
  if (percentage < 90) return '#E6A23C'
  return '#F56C6C'
}

const getEventStatusClass = (percentage: number) => {
  if (percentage < 80) return 'status-good'
  if (percentage < 100) return 'status-warning'
  return 'status-danger'
}

const getEventStatusText = (percentage: number) => {
  if (percentage < 80) return '进度良好'
  if (percentage < 100) return '接近预算'
  return '已超支'
}

const getEventDaysText = (startDate: string, endDate: string) => {
  const now = dayjs()
  const end = dayjs(endDate)
  const remaining = end.diff(now, 'day')
  
  if (remaining > 0) {
    return `还有 ${remaining} 天`
  } else if (remaining === 0) {
    return '今天截止'
  } else {
    return '已截止'
  }
}

const showAddBudgetDialog = () => {
  // 这里可以添加打开添加预算对话框的逻辑
  console.log('打开添加预算对话框')
}

// 月份改变处理函数
const onMonthChange = async (month: string) => {
  if (!month) return
  
  // 更新选中月份的数据
  const startDate = dayjs(month).startOf('month').format('YYYY-MM-DD')
  const endDate = dayjs(month).endOf('month').format('YYYY-MM-DD')
  
  // 重新获取相关数据
  await fetchSelectedMonthData(startDate, endDate)
}

// 获取选中月份的数据
const fetchSelectedMonthData = async (startDate: string, endDate: string) => {
  try {
    await Promise.all([
      store.fetchCategoryStats(startDate, endDate, 'expense'),
      store.fetchMonthlyStats(12), // 重新获取月度统计数据
    ])
  } catch (error) {
    console.error('获取月份数据失败:', error)
  }
}

onMounted(async () => {
  // 初始化数据
  await Promise.all([
    store.fetchCategories(),
    store.fetchMonthlyStats(12),
    store.fetchBudgets()
  ])
  
  // 获取当前选中月份的分类统计
  const startDate = dayjs(selectedMonth.value).startOf('month').format('YYYY-MM-DD')
  const endDate = dayjs(selectedMonth.value).endOf('month').format('YYYY-MM-DD')
  await store.fetchCategoryStats(startDate, endDate, 'expense')
})
</script>

<style scoped>
.statistics-view {
  padding: 0;
  color: #ffffff;
}

/* 月份选择器样式 */
.month-selector-container {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 24px;
  padding: 0 24px;
}

.month-selector {
  width: 200px;
}

/* 通用样式 */
.section-title {
  font-size: 24px;
  font-weight: 600;
  color: #ffffff;
  margin: 0 0 8px 0;
}

.section-subtitle {
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 24px;
}

.monthly-summary-subtitle {
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 24px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: #ffffff;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

/* 第一区域：当月花费总结 */
.monthly-summary-section {
  margin-bottom: 40px;
}

.summary-row {
  margin-bottom: 0;
}

.pie-chart-card,
.category-details-card {
  height: 580px; /* 增加高度以容纳汇总信息 */
  background: #2a2a2a;
  border: 1px solid #404040;
}

.chart-container {
  height: 320px; /* 为图例留出空间 */
  display: flex;
  align-items: center;
  justify-content: center;
}

.pie-chart {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 图例样式 */
.chart-legend {
  padding: 16px 0;
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  justify-content: center;
  align-items: center;
  border-top: 1px solid #404040;
  margin-top: 16px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: rgba(64, 64, 64, 0.2);
  border-radius: 4px;
  min-width: 0;
}

.legend-color {
  width: 12px;
  height: 12px;
  border-radius: 2px;
  flex-shrink: 0;
}

.legend-name {
  font-size: 12px;
  color: #ffffff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 80px;
}

.legend-value {
  font-size: 11px;
  color: #b0b0b0;
  font-weight: 500;
  flex-shrink: 0;
}

.category-details {
  max-height: 380px;
  overflow-y: auto;
}

.category-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 0;
  border-bottom: 1px solid #404040;
}

.category-item:last-child {
  border-bottom: none;
}

.category-info {
  display: flex;
  align-items: center;
  flex: 1;
}

.category-rank-wrapper {
  width: 50px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(64, 64, 64, 0.3);
  border-radius: 8px;
  margin-right: 12px;
}

.category-rank {
  font-size: 12px;
  font-weight: 600;
  color: #ffffff;
  text-align: center;
}

/* 排名样式 */
.category-rank-wrapper.rank-first {
  background: linear-gradient(135deg, #FFD700, #FFA500);
  box-shadow: 0 2px 8px rgba(255, 215, 0, 0.3);
}

.category-rank-wrapper.rank-second {
  background: linear-gradient(135deg, #C0C0C0, #A8A8A8);
  box-shadow: 0 2px 8px rgba(192, 192, 192, 0.3);
}

.category-rank-wrapper.rank-third {
  background: linear-gradient(135deg, #CD7F32, #B8860B);
  box-shadow: 0 2px 8px rgba(205, 127, 50, 0.3);
}

.category-rank-wrapper.rank-other {
  background: rgba(64, 64, 64, 0.3);
}

.rank-first .category-rank,
.rank-second .category-rank,
.rank-third .category-rank {
  color: #ffffff;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.category-content {
  flex: 1;
}

.category-name {
  font-size: 16px;
  font-weight: 500;
  color: #ffffff;
  margin-bottom: 4px;
}

.category-stats {
  display: flex;
  align-items: center;
  gap: 8px;
}

.percentage {
  font-size: 14px;
  color: #b0b0b0;
}

.category-amount {
  text-align: right;
}

.amount {
  font-size: 18px;
  font-weight: 600;
  color: #ffffff;
  margin-bottom: 4px;
}

.change {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
}

.change.increase {
  color: #F56C6C;
}

.change.decrease {
  color: #67C23A;
}

.change.no-change {
  color: #b0b0b0;
}

/* 第二区域：支出趋势 */
.expense-trend-section {
  margin-bottom: 40px;
}

.trend-card {
  height: 500px;
  background: #2a2a2a;
  border: 1px solid #404040;
}

.trend-tabs {
  margin-left: 16px;
}

.monthly-total {
  font-size: 16px;
  font-weight: 600;
  color: #F56C6C;
}

.trend-chart-container {
  height: 400px;
  width: 100%;
  position: relative;
}

.trend-chart {
  width: 100% !important;
  height: 100% !important;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 第三区域：预算管理 */
.budget-management-section {
  margin-bottom: 40px;
}

.budget-overview-card,
.daily-budget-card {
  height: 400px;
  background: #2a2a2a;
  border: 1px solid #404040;
}

.budget-stats {
  display: flex;
  justify-content: space-between;
  margin-bottom: 32px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 12px;
}

.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}

.budget-icon {
  background: linear-gradient(135deg, #409EFF, #66B1FF);
  color: white;
}

.spent-icon {
  background: linear-gradient(135deg, #F56C6C, #F78989);
  color: white;
}

.remaining-icon {
  background: linear-gradient(135deg, #67C23A, #85CE61);
  color: white;
}

.stat-info {
  text-align: left;
}

.stat-label {
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 20px;
  font-weight: 600;
  color: #ffffff;
}

.progress-section {
  background: #1a1a1a;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid #404040;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.progress-label {
  font-size: 16px;
  color: #ffffff;
}

.progress-percentage {
  font-size: 24px;
  font-weight: 600;
  color: #ffffff;
}

.budget-progress {
  margin-bottom: 12px;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: 14px;
  color: #b0b0b0;
}

.daily-budget {
  height: 300px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.daily-amount {
  text-align: center;
  padding: 24px;
  background: #1a1a1a;
  border-radius: 8px;
  border: 1px solid #404040;
}

.amount-label {
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 8px;
}

.amount-value {
  font-size: 28px;
  font-weight: 600;
  color: #67C23A;
}

.daily-suggestion {
  background: #1a1a1a;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid #404040;
}

.suggestion-item {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
}

.suggestion-item:last-child {
  margin-bottom: 0;
}

.suggestion-label {
  font-size: 14px;
  color: #b0b0b0;
}

.suggestion-value {
  font-size: 14px;
  font-weight: 600;
  color: #ffffff;
}

.budget-status {
  background: #1a1a1a;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid #404040;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  color: #67C23A;
}

.status-item.status-warning {
  color: #E6A23C;
}

.status-text {
  font-size: 14px;
}

.average-spending {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.spending-label {
  font-size: 14px;
  color: #b0b0b0;
}

.spending-value {
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
}

/* 第四区域：事件预算 */
.event-budget-section {
  margin-bottom: 40px;
}

.event-budget-card {
  background: #2a2a2a;
  border: 1px solid #404040;
}

.event-budget-list {
  max-height: 600px;
  overflow-y: auto;
}

.event-budget-item {
  padding: 20px;
  background: #1a1a1a;
  border-radius: 8px;
  border: 1px solid #404040;
  margin-bottom: 16px;
}

.event-budget-item:last-child {
  margin-bottom: 0;
}

.budget-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}

.budget-info {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  flex: 1;
}

.budget-icon-wrapper {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(64, 64, 64, 0.3);
  border-radius: 8px;
}

.budget-icon {
  font-size: 24px;
}

.budget-details {
  flex: 1;
}

.budget-name {
  font-size: 18px;
  font-weight: 600;
  color: #ffffff;
  margin-bottom: 4px;
}

.budget-category {
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 4px;
}

.budget-date {
  font-size: 12px;
  color: #909399;
}

.budget-amount-info {
  text-align: right;
}

.spent-amount {
  font-size: 20px;
  font-weight: 600;
  color: #F56C6C;
  margin-bottom: 4px;
}

.total-amount {
  font-size: 14px;
  color: #b0b0b0;
}

.budget-progress-section {
  margin-bottom: 12px;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.progress-label {
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
}

.remaining-amount {
  font-size: 14px;
  color: #b0b0b0;
}

.budget-status {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
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

.progress-days {
  font-size: 12px;
  color: #909399;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 200px;
}

/* 滚动条样式 */
.category-details::-webkit-scrollbar,
.event-budget-list::-webkit-scrollbar {
  width: 6px;
}

.category-details::-webkit-scrollbar-track,
.event-budget-list::-webkit-scrollbar-track {
  background: #1a1a1a;
  border-radius: 3px;
}

.category-details::-webkit-scrollbar-thumb,
.event-budget-list::-webkit-scrollbar-thumb {
  background: #606060;
  border-radius: 3px;
}

.category-details::-webkit-scrollbar-thumb:hover,
.event-budget-list::-webkit-scrollbar-thumb:hover {
  background: #808080;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .month-selector-container {
    padding: 0 16px;
    margin-bottom: 16px;
  }
  
  .month-selector {
    width: 100%;
  }
  
  .section-title {
    font-size: 20px;
  }
  
  .pie-chart-card,
  .category-details-card {
    height: 500px;
  }
  
  .chart-container {
    height: 250px;
  }
  
  .chart-legend {
    padding: 12px 0;
    gap: 8px;
  }
  
  .legend-item {
    padding: 3px 6px;
    gap: 4px;
  }
  
  .legend-name {
    font-size: 11px;
    max-width: 60px;
  }
  
  .legend-value {
    font-size: 10px;
  }
  
  .trend-chart-container {
    height: 300px;
  }
  
  .category-rank-wrapper {
    width: 45px;
    height: 35px;
  }
  
  .category-rank {
    font-size: 10px;
  }
  
  .category-details {
    max-height: 300px;
  }
  
  .trend-card {
    height: 400px;
  }
  
  .trend-chart-container {
    height: 300px;
  }
  
  .budget-overview-card,
  .daily-budget-card {
    height: auto;
  }
  
  .budget-stats {
    flex-direction: column;
    gap: 16px;
  }
  
  .stat-item {
    justify-content: center;
  }
  
  .header-left {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  
  .card-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
}
</style>
