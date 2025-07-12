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
import { ref, reactive, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Money, ArrowDown, ArrowRight } from '@element-plus/icons-vue'
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

// 分类选择相关
const showCategoryPanel = ref(false)
const hoveredParentId = ref<number | null>(null)
const categorySelectRef = ref()

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

const availableParentCategories = computed(() => {
  return store.categories.filter(cat => cat.type === form.type && !cat.parent_id)
})

const selectedCategory = computed(() => {
  if (!form.category_id) return null
  return store.categories.find(cat => cat.id === form.category_id)
})

const hoveredSubCategories = computed(() => {
  if (!hoveredParentId.value) return []
  return store.categories.filter(cat => cat.parent_id === hoveredParentId.value)
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
  showCategoryPanel.value = false
  hoveredParentId.value = null
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
  showCategoryPanel.value = false
  hoveredParentId.value = null
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
  form.category_id = category.id
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

// 生命周期钩子
onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
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
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
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
  min-width: 0;
  overflow: hidden;
}

.record-desc {
  font-size: 14px;
  color: #ffffff;
  margin-bottom: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.record-time {
  font-size: 12px;
  color: #b0b0b0;
}

.record-amount {
  font-size: 16px;
  font-weight: 600;
  flex-shrink: 0;
  min-width: 80px;
  text-align: right;
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