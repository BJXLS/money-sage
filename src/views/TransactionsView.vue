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
      :title="editingTransactionId ? '编辑记录' : '记账'"
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
                <div class="category-tree-select" ref="categorySelectRef">
                  <div 
                    class="category-display" 
                    @click="toggleCategoryPanel"
                    :class="{ 'active': showCategoryPanel }"
                  >
                    <div v-if="selectedCategory" class="selected-category">
                      <span class="category-icon" :style="{ color: selectedCategory.color }">
                        {{ selectedCategory.icon || '💰' }}
                      </span>
                      <span>{{ selectedCategory.name }}</span>
                    </div>
                    <span v-else class="placeholder">选择分类</span>
                    <el-icon class="arrow-icon" :class="{ 'rotate': showCategoryPanel }">
                      <ArrowDown />
                    </el-icon>
                  </div>
                  
                  <div v-if="showCategoryPanel" class="category-panel">
                    <div class="parent-categories">
                      <div 
                        v-for="parentCategory in availableParentCategories" 
                        :key="parentCategory.id"
                        class="parent-category-item"
                        @mouseenter="setHoveredParent(parentCategory.id)"
                        @click="selectCategory(parentCategory)"
                      >
                        <span class="category-icon" :style="{ color: parentCategory.color }">
                          {{ parentCategory.icon || '📁' }}
                        </span>
                        <span class="category-name">{{ parentCategory.name }}</span>
                        <el-icon class="arrow-right">
                          <ArrowRight />
                        </el-icon>
                      </div>
                    </div>
                    
                    <div class="sub-categories">
                      <template v-if="hoveredSubCategories.length > 0">
                        <div 
                          v-for="subCategory in hoveredSubCategories" 
                          :key="subCategory.id"
                          class="sub-category-item"
                          @click="selectCategory(subCategory)"
                        >
                          <span class="category-icon" :style="{ color: subCategory.color }">
                            {{ subCategory.icon || '📋' }}
                          </span>
                          <span class="category-name">{{ subCategory.name }}</span>
                        </div>
                      </template>
                      <div v-else class="sub-categories-placeholder">
                        <span class="placeholder-text">
                          {{ hoveredParentId ? '没有小类' : '请选择左侧大类' }}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
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
            {{ editingTransactionId ? '更新记录' : '保存记录' }}
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
              <div class="record-right">
                <div class="record-amount" :class="record.type">
                  {{ record.type === 'income' ? '+' : '-' }}¥{{ formatAmount(record.amount) }}
                </div>
                <div class="record-actions">
                  <el-button 
                    @click="editTransaction(record)" 
                    type="primary" 
                    size="small" 
                    text
                    class="action-btn"
                  >
                    <el-icon><Edit /></el-icon>
                  </el-button>
                  <el-button 
                    @click="deleteTransaction(record.id)" 
                    type="danger" 
                    size="small" 
                    text
                    class="action-btn"
                  >
                    <el-icon><Delete /></el-icon>
                  </el-button>
                </div>
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
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, ArrowRight, ArrowDown, Money, Edit, Delete } from '@element-plus/icons-vue'
import { useAppStore } from '../stores'
import dayjs from 'dayjs'

const store = useAppStore()

// 响应式数据
const currentDate = ref(dayjs())
const selectedDate = ref(dayjs().format('YYYY-MM-DD'))
const viewMode = ref<'day' | 'week' | 'month'>('month')
const showRecordDialog = ref(false)

// 分类选择相关
const showCategoryPanel = ref(false)
const hoveredParentId = ref<number | null>(null)
const categorySelectRef = ref()

// 编辑相关
const editingTransactionId = ref<number | null>(null)

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

const availableParentCategories = computed(() => {
  return store.categories.filter(cat => cat.type === transactionForm.value.type && !cat.parent_id)
})

const selectedCategory = computed(() => {
  if (!transactionForm.value.categoryId) return null
  return store.categories.find(cat => cat.id === transactionForm.value.categoryId)
})

const hoveredSubCategories = computed(() => {
  if (!hoveredParentId.value) return []
  return store.categories.filter(cat => cat.parent_id === hoveredParentId.value)
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
  editingTransactionId.value = null
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

  try {
    if (editingTransactionId.value) {
      // 更新现有交易
      const updateData = {
        type: transactionForm.value.type,
        amount: parseFloat(transactionForm.value.amount),
        category_id: transactionForm.value.categoryId,
        date: selectedDate.value,
        description: transactionForm.value.note,
        note: transactionForm.value.note
      }
      await store.updateTransaction(editingTransactionId.value, updateData)
      ElMessage.success('记录更新成功')
    } else {
      // 创建新交易
      const transaction = {
        type: transactionForm.value.type,
        amount: parseFloat(transactionForm.value.amount),
        category_id: transactionForm.value.categoryId,
        date: selectedDate.value,
        description: transactionForm.value.note,
        note: transactionForm.value.note
      }
      await store.createTransaction(transaction)
      ElMessage.success('记录保存成功')
    }
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

// 分类选择相关方法
const toggleCategoryPanel = () => {
  showCategoryPanel.value = !showCategoryPanel.value
  if (!showCategoryPanel.value) {
    hoveredParentId.value = null
  }
}

const setHoveredParent = (parentId: number) => {
  hoveredParentId.value = parentId
}

const selectCategory = (category: any) => {
  transactionForm.value.categoryId = category.id
  showCategoryPanel.value = false
  hoveredParentId.value = null
}

// 点击外部关闭面板
const handleClickOutside = (event: MouseEvent) => {
  if (categorySelectRef.value && !categorySelectRef.value.contains(event.target as Node)) {
    showCategoryPanel.value = false
    hoveredParentId.value = null
  }
}

onMounted(() => {
  store.fetchTransactions()
  store.fetchCategories()
  document.addEventListener('click', handleClickOutside)
})

// 编辑交易记录
const editTransaction = (transaction: any) => {
  editingTransactionId.value = transaction.id
  transactionForm.value = {
    type: transaction.type,
    amount: transaction.amount.toString(),
    categoryId: transaction.category_id,
    account: '现金', // 默认值，因为原数据可能没有account字段
    time: formatTime(transaction.created_at),
    note: transaction.description || transaction.note || ''
  }
  showRecordDialog.value = true
}

// 删除交易记录
const deleteTransaction = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这条记录吗？', '确认删除', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    await store.deleteTransaction(id)
    ElMessage.success('删除成功')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败，请重试')
    }
  }
}

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
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

/* 分类树选择器样式 */
.category-tree-select {
  position: relative;
  width: 100%;
}

.category-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #1a1a1a;
  border: 1px solid #404040;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s ease;
  height: 32px;
  box-sizing: border-box;
}

.category-display:hover {
  border-color: #606060;
}

.category-display.active {
  border-color: #409eff;
}

.selected-category {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #ffffff;
}

.placeholder {
  color: #b0b0b0;
}

.arrow-icon {
  transition: transform 0.3s ease;
}

.arrow-icon.rotate {
  transform: rotate(180deg);
}

.category-panel {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 1000;
  background: #1a1a1a;
  border: 1px solid #404040;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  display: flex;
  max-height: 300px;
  overflow: hidden;
  margin-top: 4px;
  width: 400px;
}

.parent-categories {
  width: 200px;
  border-right: 1px solid #404040;
  overflow-y: auto;
}

.sub-categories {
  width: 200px;
  overflow-y: auto;
}

.parent-category-item,
.sub-category-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  cursor: pointer;
  transition: background-color 0.3s ease;
  border-bottom: 1px solid #2a2a2a;
}

.parent-category-item:hover,
.sub-category-item:hover {
  background: #2a2a2a;
}

.parent-category-item .category-name,
.sub-category-item .category-name {
  flex: 1;
  margin-left: 8px;
  color: #ffffff;
  font-size: 14px;
}

.parent-category-item .category-icon,
.sub-category-item .category-icon {
  font-size: 16px;
}

.arrow-right {
  color: #b0b0b0;
  font-size: 12px;
}

/* 滚动条样式 */
.parent-categories::-webkit-scrollbar,
.sub-categories::-webkit-scrollbar {
  width: 6px;
}

.parent-categories::-webkit-scrollbar-track,
.sub-categories::-webkit-scrollbar-track {
  background: #1a1a1a;
  border-radius: 3px;
}

.parent-categories::-webkit-scrollbar-thumb,
.sub-categories::-webkit-scrollbar-thumb {
  background: #606060;
  border-radius: 3px;
}

.parent-categories::-webkit-scrollbar-thumb:hover,
.sub-categories::-webkit-scrollbar-thumb:hover {
  background: #808080;
}

.sub-categories-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100px;
  color: #b0b0b0;
}

.placeholder-text {
  font-size: 14px;
  color: #b0b0b0;
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
  transition: all 0.3s ease;
}

.record-item:hover {
  background: #3a3a3a;
  margin: 0 -12px;
  padding: 12px;
  border-radius: 6px;
}

.record-item:hover .record-actions {
  opacity: 1;
}

.record-item:last-child {
  border-bottom: none;
}

.record-left {
  display: flex;
  align-items: center;
  flex: 1;
}

.record-right {
  display: flex;
  align-items: center;
  gap: 12px;
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

.record-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.action-btn {
  padding: 4px 8px !important;
  border: none !important;
}

/* 移动端显示操作按钮 */
@media (max-width: 768px) {
  .record-actions {
    opacity: 1;
  }
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