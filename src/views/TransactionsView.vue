<template>
  <div class="transactions-view">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <div class="date-navigation">
        <el-button @click="prevMonth" :icon="ArrowLeft" size="small" />
        <span class="current-month">{{ currentMonthText }}</span>
        <el-button @click="nextMonth" :icon="ArrowRight" size="small" />
      </div>
      
      <div class="view-controls">
        <el-button-group>
          <el-button 
            :type="viewMode === 'day' ? 'primary' : 'default'" 
            @click="setViewMode('day')"
            size="small"
          >
            日
          </el-button>
          <el-button 
            :type="viewMode === 'week' ? 'primary' : 'default'" 
            @click="setViewMode('week')"
            size="small"
          >
            周
          </el-button>
          <el-button 
            :type="viewMode === 'month' ? 'primary' : 'default'" 
            @click="setViewMode('month')"
            size="small"
          >
            月
          </el-button>
        </el-button-group>
      </div>
    </div>

    <!-- 日历视图 -->
    <el-card class="calendar-card">
      <div class="calendar-container">
        <!-- 日历头部 -->
        <div class="calendar-header">
          <div class="weekday" v-for="day in weekdays" :key="day">{{ day }}</div>
        </div>
        
        <!-- 日历主体 -->
        <div class="calendar-body">
          <div 
            v-for="day in calendarDays" 
            :key="day.date"
            class="calendar-day"
            :class="{
              'other-month': day.isOtherMonth,
              'today': day.isToday,
              'selected': day.date === selectedDate,
              'has-transactions': day.transactions.length > 0
            }"
            @click="selectDate(day.date)"
          >
            <div class="day-number">{{ day.dayNumber }}</div>
            <div class="day-summary" v-if="day.transactions.length > 0">
              <div class="expense-amount">-¥{{ formatAmount(day.totalExpense) }}</div>
              <div class="transaction-count">{{ day.transactions.length }}笔</div>
            </div>
          </div>
        </div>
      </div>
    </el-card>

    <!-- 记账对话框 -->
    <el-dialog
      v-model="showRecordDialog"
      title="记账"
      width="500px"
      :before-close="handleDialogClose"
      class="record-dialog"
    >
      <div class="record-form">
        <!-- 日期显示 -->
        <div class="date-header">
          <h3>{{ selectedDateText }}</h3>
          <div class="daily-summary">
            <div class="summary-item income">
              <span class="label">收入</span>
              <span class="amount">¥{{ formatAmount(dailyStats.income) }}</span>
            </div>
            <div class="summary-item expense">
              <span class="label">支出</span>
              <span class="amount">¥{{ formatAmount(dailyStats.expense) }}</span>
            </div>
            <div class="summary-item balance">
              <span class="label">结余</span>
              <span class="amount">¥{{ formatAmount(dailyStats.balance) }}</span>
            </div>
          </div>
        </div>

        <!-- 收入/支出切换 -->
        <div class="type-buttons">
          <el-button
            :type="transactionForm.type === 'expense' ? 'primary' : 'default'"
            @click="transactionForm.type = 'expense'"
            class="type-btn"
          >
            支出
          </el-button>
          <el-button
            :type="transactionForm.type === 'income' ? 'primary' : 'default'"
            @click="transactionForm.type = 'income'"
            class="type-btn"
          >
            收入
          </el-button>
        </div>

        <!-- 表单 -->
        <el-form :model="transactionForm" label-width="60px">
          <el-row :gutter="20">
            <el-col :span="12">
              <el-form-item label="金额">
                <el-input
                  v-model="transactionForm.amount"
                  placeholder="0.00"
                  type="number"
                  class="amount-input"
                  prefix-icon="Money"
                />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="分类">
                <el-select
                  v-model="transactionForm.categoryId"
                  placeholder="选择分类"
                  class="category-select"
                >
                  <el-option
                    v-for="category in availableCategories"
                    :key="category.id"
                    :label="category.name"
                    :value="category.id"
                  >
                    <div class="category-option">
                      <span class="category-icon">{{ category.icon || '💰' }}</span>
                      <span>{{ category.name }}</span>
                    </div>
                  </el-option>
                </el-select>
              </el-form-item>
            </el-col>
          </el-row>

          <el-row :gutter="20">
            <el-col :span="12">
              <el-form-item label="账户">
                <el-select
                  v-model="transactionForm.account"
                  placeholder="现金"
                  class="account-select"
                >
                  <el-option label="现金" value="现金" />
                  <el-option label="银行卡" value="银行卡" />
                  <el-option label="支付宝" value="支付宝" />
                  <el-option label="微信" value="微信" />
                </el-select>
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="时间">
                                  <el-input
                    v-model="transactionForm.time"
                    placeholder="11:05"
                    type="time"
                  />
              </el-form-item>
            </el-col>
          </el-row>

          <el-form-item label="备注">
            <el-input
              v-model="transactionForm.note"
              placeholder="添加备注信息..."
              type="textarea"
              :rows="2"
              maxlength="100"
              show-word-limit
            />
          </el-form-item>
        </el-form>

        <!-- 操作按钮 -->
        <div class="form-actions">
          <el-button @click="handleDialogClose" size="large">取消</el-button>
          <el-button type="primary" @click="saveTransaction" size="large">
            <el-icon><Money /></el-icon>
            保存记录
          </el-button>
        </div>

        <!-- 当日记录 -->
        <div class="daily-records">
          <h4>当日记录</h4>
          <div class="records-list">
            <div 
              v-for="record in dailyTransactions" 
              :key="record.id"
              class="record-item"
            >
              <div class="record-left">
                <span class="record-icon" :style="{ color: record.category_color }">
                  {{ record.category_icon || '💰' }}
                </span>
                <div class="record-info">
                  <div class="record-desc">{{ record.description || record.category_name }}</div>
                  <div class="record-time">{{ formatTime(record.created_at) }}</div>
                </div>
              </div>
              <div class="record-amount" :class="record.type">
                {{ record.type === 'income' ? '+' : '-' }}¥{{ formatAmount(record.amount) }}
              </div>
            </div>
            <div v-if="dailyTransactions.length === 0" class="no-records">
              暂无记录
            </div>
          </div>
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { ArrowLeft, ArrowRight, Money } from '@element-plus/icons-vue'
import { useAppStore } from '../stores'
import dayjs from 'dayjs'

const store = useAppStore()

// 响应式数据
const currentDate = ref(dayjs())
const selectedDate = ref(dayjs().format('YYYY-MM-DD'))
const viewMode = ref<'day' | 'week' | 'month'>('month')
const showRecordDialog = ref(false)

// 交易表单
const transactionForm = ref({
  type: 'expense' as 'income' | 'expense',
  amount: '',
  categoryId: null as number | null,
  account: '现金',
  time: '11:05',
  note: ''
})

// 周天数组
const weekdays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

// 计算属性
const currentMonthText = computed(() => {
  return currentDate.value.format('YYYY年M月')
})

const selectedDateText = computed(() => {
  return dayjs(selectedDate.value).format('YYYY年M月D日')
})

const availableCategories = computed(() => {
  return store.categories.filter(cat => cat.type === transactionForm.value.type)
})

const dailyTransactions = computed(() => {
  return store.transactions.filter(t => 
    dayjs(t.date).format('YYYY-MM-DD') === selectedDate.value
  )
})

const dailyStats = computed(() => {
  const dayTransactions = dailyTransactions.value
  const income = dayTransactions
    .filter(t => t.type === 'income')
    .reduce((sum, t) => sum + t.amount, 0)
  const expense = dayTransactions
    .filter(t => t.type === 'expense')
    .reduce((sum, t) => sum + t.amount, 0)
  
  return {
    income,
    expense,
    balance: income - expense
  }
})

const calendarDays = computed(() => {
  const year = currentDate.value.year()
  const month = currentDate.value.month()
  
  // 获取当月第一天和最后一天
  const firstDay = dayjs().year(year).month(month).date(1)
  const lastDay = firstDay.endOf('month')
  
  // 获取日历开始和结束日期（包含上月末和下月初）
  const startDate = firstDay.startOf('week')
  const endDate = lastDay.endOf('week')
  
  const days = []
  let current = startDate
  
  while (current.isBefore(endDate) || current.isSame(endDate)) {
    const dateStr = current.format('YYYY-MM-DD')
    const dayTransactions = store.transactions.filter(t => 
      dayjs(t.date).format('YYYY-MM-DD') === dateStr
    )
    
    const totalExpense = dayTransactions
      .filter(t => t.type === 'expense')
      .reduce((sum, t) => sum + t.amount, 0)
    
    days.push({
      date: dateStr,
      dayNumber: current.date(),
      isOtherMonth: current.month() !== month,
      isToday: current.isSame(dayjs(), 'day'),
      transactions: dayTransactions,
      totalExpense
    })
    
    current = current.add(1, 'day')
  }
  
  return days
})

// 方法
const setViewMode = (mode: 'day' | 'week' | 'month') => {
  viewMode.value = mode
}

const prevMonth = () => {
  currentDate.value = currentDate.value.subtract(1, 'month')
}

const nextMonth = () => {
  currentDate.value = currentDate.value.add(1, 'month')
}

const selectDate = (date: string) => {
  selectedDate.value = date
  showRecordDialog.value = true
  resetForm()
}

const resetForm = () => {
  transactionForm.value = {
    type: 'expense',
    amount: '',
    categoryId: null,
    account: '现金',
    time: '11:05',
    note: ''
  }
}

const handleDialogClose = () => {
  showRecordDialog.value = false
  resetForm()
}

const saveTransaction = async () => {
  if (!transactionForm.value.amount || !transactionForm.value.categoryId) {
    ElMessage.warning('请填写完整的交易信息')
    return
  }

  const transaction = {
    type: transactionForm.value.type,
    amount: parseFloat(transactionForm.value.amount),
    category_id: transactionForm.value.categoryId,
    date: selectedDate.value,
    description: transactionForm.value.note,
    note: transactionForm.value.note
  }

  try {
    await store.createTransaction(transaction)
    ElMessage.success('记录保存成功')
    handleDialogClose()
  } catch (error) {
    ElMessage.error('保存失败，请重试')
  }
}

const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', { 
    minimumFractionDigits: 2, 
    maximumFractionDigits: 2 
  })
}

const formatTime = (dateTime: string) => {
  return dayjs(dateTime).format('HH:mm')
}

onMounted(() => {
  store.fetchTransactions()
  store.fetchCategories()
})
</script>

<style scoped>
.transactions-view {
  padding: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.date-navigation {
  display: flex;
  align-items: center;
  gap: 12px;
}

.current-month {
  font-size: 18px;
  font-weight: 600;
  color: #ffffff;
  min-width: 120px;
  text-align: center;
}

.view-controls {
  display: flex;
  gap: 8px;
}

.calendar-card {
  background: #2a2a2a;
  border: 1px solid #404040;
}

.calendar-container {
  width: 100%;
}

.calendar-header {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
  margin-bottom: 1px;
}

.weekday {
  padding: 12px;
  text-align: center;
  font-weight: 600;
  color: #b0b0b0;
  background: #404040;
}

.calendar-body {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
  background: #404040;
}

.calendar-day {
  min-height: 100px;
  padding: 8px;
  background: #2a2a2a;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.calendar-day:hover {
  background: #3a3a3a;
}

.calendar-day.other-month {
  background: #1a1a1a;
  color: #666666;
}

.calendar-day.today {
  background: #2d4a6b;
}

.calendar-day.selected {
  background: #409eff;
}

.calendar-day.has-transactions {
  border-left: 4px solid #67c23a;
}

.day-number {
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
  margin-bottom: 4px;
}

.day-summary {
  font-size: 12px;
  color: #b0b0b0;
}

.expense-amount {
  color: #f56c6c;
  font-weight: 500;
}

.transaction-count {
  margin-top: 2px;
  color: #909399;
}

.record-dialog {
  background: #2a2a2a;
}

.record-form {
  color: #ffffff;
}

.date-header {
  margin-bottom: 24px;
}

.date-header h3 {
  margin: 0 0 12px 0;
  font-size: 20px;
  color: #ffffff;
}

.daily-summary {
  display: flex;
  gap: 20px;
  padding: 16px;
  background: #1a1a1a;
  border-radius: 8px;
  border: 1px solid #404040;
}

.summary-item {
  text-align: center;
}

.summary-item .label {
  display: block;
  font-size: 14px;
  color: #b0b0b0;
  margin-bottom: 4px;
}

.summary-item .amount {
  display: block;
  font-size: 18px;
  font-weight: 600;
}

.summary-item.income .amount {
  color: #67c23a;
}

.summary-item.expense .amount {
  color: #f56c6c;
}

.summary-item.balance .amount {
  color: #409eff;
}

.type-buttons {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.type-btn {
  flex: 1;
  height: 40px;
}

.form-actions {
  display: flex;
  gap: 12px;
  margin: 24px 0;
}

.form-actions .el-button {
  flex: 1;
}

.daily-records {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid #404040;
}

.daily-records h4 {
  margin: 0 0 16px 0;
  color: #ffffff;
}

.records-list {
  max-height: 200px;
  overflow-y: auto;
}

.record-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid #404040;
}

.record-item:last-child {
  border-bottom: none;
}

.record-left {
  display: flex;
  align-items: center;
}

.record-icon {
  font-size: 18px;
  margin-right: 12px;
}

.record-info {
  flex: 1;
}

.record-desc {
  font-size: 14px;
  color: #ffffff;
  margin-bottom: 2px;
}

.record-time {
  font-size: 12px;
  color: #b0b0b0;
}

.record-amount {
  font-size: 16px;
  font-weight: 600;
}

.record-amount.income {
  color: #67c23a;
}

.record-amount.expense {
  color: #f56c6c;
}

.no-records {
  text-align: center;
  color: #b0b0b0;
  padding: 24px;
}

.category-option {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
}

/* 滚动条样式 */
.records-list::-webkit-scrollbar {
  width: 6px;
}

.records-list::-webkit-scrollbar-track {
  background: #1a1a1a;
  border-radius: 3px;
}

.records-list::-webkit-scrollbar-thumb {
  background: #606060;
  border-radius: 3px;
}

.records-list::-webkit-scrollbar-thumb:hover {
  background: #808080;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .toolbar {
    flex-direction: column;
    gap: 12px;
  }
  
  .calendar-day {
    min-height: 80px;
  }
  
  .daily-summary {
    flex-direction: column;
    gap: 12px;
  }
  
  .type-buttons {
    flex-direction: column;
  }
}
</style> 