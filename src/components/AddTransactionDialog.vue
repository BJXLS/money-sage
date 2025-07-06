<template>
  <el-dialog
    v-model="dialogVisible"
    title="记账"
    width="500px"
    :close-on-click-modal="false"
    @close="handleClose"
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
          :type="form.type === 'expense' ? 'primary' : 'default'"
          @click="form.type = 'expense'"
          class="type-btn"
        >
          支出
        </el-button>
        <el-button
          :type="form.type === 'income' ? 'primary' : 'default'"
          @click="form.type = 'income'"
          class="type-btn"
        >
          收入
        </el-button>
      </div>

      <!-- 表单 -->
      <el-form :model="form" label-width="60px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="金额">
              <el-input
                v-model="form.amount"
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
                v-model="form.category_id"
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
                v-model="form.account"
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
                v-model="form.time"
                placeholder="11:05"
                type="time"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="备注">
          <el-input
            v-model="form.note"
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
        <el-button @click="handleClose" size="large">取消</el-button>
        <el-button type="primary" @click="handleSubmit" size="large" :loading="submitting">
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
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { Money } from '@element-plus/icons-vue'
import { useAppStore } from '../stores'
import dayjs from 'dayjs'

interface Props {
  modelValue: boolean
}

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  (e: 'success'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const store = useAppStore()
const submitting = ref(false)

// 表单数据
const form = reactive({
  type: 'expense' as 'income' | 'expense',
  amount: '',
  category_id: null as number | null,
  account: '现金',
  time: '11:05',
  note: '',
  date: dayjs().format('YYYY-MM-DD')
})

// 计算属性
const dialogVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const selectedDateText = computed(() => {
  return dayjs(form.date).format('YYYY年M月D日')
})

const availableCategories = computed(() => {
  return store.categories.filter(cat => cat.type === form.type)
})

const dailyTransactions = computed(() => {
  return store.transactions.filter(t => 
    dayjs(t.date).format('YYYY-MM-DD') === form.date
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

// 监听类型变化，重置分类选择
watch(() => form.type, () => {
  form.category_id = null
})

// 监听对话框打开
watch(dialogVisible, (visible) => {
  if (visible) {
    resetForm()
    nextTick(() => {
      // 刷新数据以获取最新的交易记录
      store.fetchTransactions()
    })
  }
})

// 方法
const resetForm = () => {
  form.type = 'expense'
  form.amount = ''
  form.category_id = null
  form.account = '现金'
  form.time = '11:05'
  form.note = ''
  form.date = dayjs().format('YYYY-MM-DD')
}

const handleClose = () => {
  dialogVisible.value = false
}

const handleSubmit = async () => {
  if (!form.amount || !form.category_id) {
    ElMessage.warning('请填写完整的交易信息')
    return
  }

  const transaction = {
    type: form.type,
    amount: parseFloat(form.amount),
    category_id: form.category_id,
    date: form.date,
    description: form.note,
    note: form.note
  }

  try {
    submitting.value = true
    await store.createTransaction(transaction)
    ElMessage.success('记录保存成功')
    emit('success')
    handleClose()
  } catch (error) {
    ElMessage.error('保存失败，请重试')
  } finally {
    submitting.value = false
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
</script>

<style scoped>
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
  .daily-summary {
    flex-direction: column;
    gap: 12px;
  }
  
  .type-buttons {
    flex-direction: column;
  }
}
</style> 