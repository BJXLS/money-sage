<template>
  <el-dialog
    v-model="visible"
    title="快速记账"
    width="1000px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <div class="quick-booking-content">
      <!-- 输入界面 -->
      <div v-if="!showConfirmation" class="input-section">
        <div class="input-label">
          <el-icon class="label-icon"><EditPen /></el-icon>
          <span>请描述您的收支情况</span>
        </div>
        <el-input
          v-model="inputText"
          type="textarea"
          :rows="8"
          placeholder="例如：今天中午在餐厅花了38元吃午饭&#10;昨天收到工资5000元&#10;买了一本编程书籍89元"
          class="text-input"
          maxlength="500"
          show-word-limit
          clearable
        />
        <div class="input-tips">
          <div class="tip-item">
            <el-icon><InfoFilled /></el-icon>
            <span>支持多条记录，每行一条</span>
          </div>
          <div class="tip-item">
            <el-icon><InfoFilled /></el-icon>
            <span>AI将自动识别金额、分类和时间</span>
          </div>
        </div>
      </div>

      <!-- 确认界面 -->
      <div v-else class="confirmation-section">
        <div class="confirmation-header">
          <el-icon class="header-icon"><CircleCheck /></el-icon>
          <span>AI识别结果确认</span>
        </div>
        <div class="confirmation-tip">
          <el-alert
            title="请检查并确认AI识别的记账信息，您可以直接编辑修改"
            type="info"
            :closable="false"
            show-icon
          />
        </div>
        
        <div class="transactions-table">
          <el-table :data="parsedTransactions" style="width: 100%">
            <el-table-column label="日期" width="140">
              <template #default="{ row }">
                <el-date-picker
                  v-model="row.date"
                  type="date"
                  format="YYYY-MM-DD"
                  value-format="YYYY-MM-DD"
                  size="small"
                  style="width: 100%"
                />
              </template>
            </el-table-column>
            
            <el-table-column label="金额" width="150">
              <template #default="{ row }">
                <el-input-number
                  v-model="row.amount"
                  :min="0.01"
                  :precision="2"
                  size="small"
                  style="width: 100%"
                />
              </template>
            </el-table-column>
            
            <el-table-column label="类型" width="100">
              <template #default="{ row }">
                <el-select v-model="row.transaction_type" size="small" style="width: 100%">
                  <el-option label="支出" value="expense" />
                  <el-option label="收入" value="income" />
                </el-select>
              </template>
            </el-table-column>
            
            <el-table-column label="分类" width="150">
              <template #default="{ row }">
                <el-select 
                  v-model="row.category_id" 
                  size="small" 
                  style="width: 100%"
                  filterable
                  placeholder="选择分类"
                >
                  <el-option-group
                    v-for="parentCategory in getParentCategories(row.transaction_type)"
                    :key="parentCategory.id"
                    :label="`${parentCategory.icon || '📁'} ${parentCategory.name}`"
                  >
                    <el-option
                      v-for="subCategory in getSubCategories(parentCategory.id)"
                      :key="subCategory.id"
                      :label="subCategory.name"
                      :value="subCategory.id"
                    >
                      <div class="category-option">
                        <span class="category-icon" :style="{ color: subCategory.color }">
                          {{ subCategory.icon || '📋' }}
                        </span>
                        <span>{{ subCategory.name }}</span>
                      </div>
                    </el-option>
                  </el-option-group>
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="预算" width="200">
              <template #default="{ row }">
                <el-select
                  v-if="row.transaction_type === 'expense'"
                  v-model="row.budget_id"
                  size="small"
                  clearable
                  filterable
                  :suffix-icon="(getAllBudgets().length > 1 && !row.budget_id) ? Warning : undefined"
                  :class="{ 'warning-select': row.category_id && !row.budget_id }"
                  placeholder="选择预算（可选）"
                >
                  <el-option
                    v-for="budget in getAllBudgets()"
                    :key="budget.id"
                    :label="`${budget.name}（${budget.category_name}）`"
                    :value="budget.id"
                  />
                </el-select>
                <span v-else class="dash">-</span>
              </template>
            </el-table-column>
            
            <el-table-column label="描述" min-width="150">
              <template #default="{ row }">
                <el-input
                  v-model="row.description"
                  size="small"
                  placeholder="输入描述"
                />
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <!-- 输入界面的按钮 -->
        <template v-if="!showConfirmation">
          <el-button @click="handleClose" class="cancel-btn">
            取消
          </el-button>
          <el-button @click="handleClear" class="clear-btn">
            清空
          </el-button>
          <el-button 
            type="primary" 
            @click="handleSubmit" 
            :loading="processing"
            :disabled="!inputText.trim()"
            class="submit-btn"
          >
            <el-icon v-if="!processing"><Lightning /></el-icon>
            {{ processing ? '处理中...' : '智能记账' }}
          </el-button>
        </template>

        <!-- 确认界面的按钮 -->
        <template v-else>
          <el-button @click="handleClose" class="cancel-btn">
            取消
          </el-button>
          <el-button @click="handleBackToEdit" class="back-btn">
            <el-icon><ArrowLeft /></el-icon>
            返回编辑
          </el-button>
          <el-button 
            type="primary" 
            @click="handleSaveTransactions" 
            :loading="processing"
            class="confirm-btn"
          >
            <el-icon v-if="!processing"><Check /></el-icon>
            {{ processing ? '保存中...' : '确认保存' }}
          </el-button>
        </template>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { InfoFilled, EditPen, Lightning, CircleCheck, ArrowLeft, Check, Warning } from '@element-plus/icons-vue'

// 类型定义
interface ParsedTransaction {
  original_text: string
  date: string
  amount: number
  transaction_type: string
  category_name: string
  category_id: number | null
  budget_id?: number | null
  description: string
  confidence: number
}

interface Category {
  id: number
  name: string
  type: string
  icon?: string
  color?: string
  parent_id?: number | null
}

interface QuickBookingResult {
  success: boolean
  message: string
  parsed_transactions: ParsedTransaction[]
  failed_lines: any[]
}

interface SaveTransactionsResult {
  success: boolean
  message: string
  saved_count: number
  failed_count: number
}

// Props和Emits
const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'success': [data: any]
}>()

// 响应式数据
const visible = ref(false)
const inputText = ref('')
const processing = ref(false)
const showConfirmation = ref(false)
const parsedTransactions = ref<ParsedTransaction[]>([])
const allCategories = ref<Category[]>([])
const allBudgets = ref<any[]>([])

// 分类选择相关 - 简化实现

// 监听props变化
watch(
  () => props.modelValue,
  (newVal) => {
    visible.value = newVal
    if (newVal) {
      // 打开对话框时重置状态
      inputText.value = ''
      processing.value = false
      showConfirmation.value = false
      parsedTransactions.value = []
      // 加载分类数据
      loadCategories()
    }
  },
  { immediate: true }
)

// 监听visible变化
watch(visible, (newVal) => {
  emit('update:modelValue', newVal)
})

// 加载分类数据
const loadCategories = async () => {
  try {
    const categories = await invoke<Category[]>('get_categories')
    allCategories.value = categories
    const budgets = await invoke<any[]>('get_budgets')
    allBudgets.value = budgets || []
  } catch (error) {
    console.error('加载分类失败:', error)
    ElMessage.error('加载分类失败')
  }
}
// 根据行数据匹配预算（按小类和事件/时间预算均可）
const getAllBudgets = () => allBudgets.value.filter(b => b.is_active)

// 预算和分类已解除绑定，用户可以独立选择

// 清空输入
const handleClear = () => {
  inputText.value = ''
}

// 提交处理 - AI解析
const handleSubmit = async () => {
  if (!inputText.value.trim()) {
    ElMessage.warning('请输入记账信息')
    return
  }

  processing.value = true
  try {
    // 调用后端API进行AI文本解析
    const result = await invoke<QuickBookingResult>('process_quick_booking_text', {
      text: inputText.value.trim()
    })

    console.log('AI解析结果:', result)

    if (result.success && result.parsed_transactions && result.parsed_transactions.length > 0) {
      // AI解析成功，显示确认界面
      parsedTransactions.value = result.parsed_transactions.map(transaction => ({
        ...transaction,
        // 确保有默认的分类ID，如果没有则设置为null
        category_id: transaction.category_id || null,
        budget_id: null
      }))
      showConfirmation.value = true

      ElMessage.success(`AI识别成功！解析出${result.parsed_transactions.length}条记录，请确认后保存`)
    } else {
      // AI解析失败
      ElMessage.error(result.message || 'AI解析失败，请检查输入内容')
    }
  } catch (error) {
    console.error('AI解析失败:', error)
    ElMessage.error('AI解析失败，请检查输入格式或稍后重试')
  } finally {
    processing.value = false
  }
}

// 保存确认的交易记录
const handleSaveTransactions = async () => {
  // 验证所有交易记录是否完整
  const invalidTransactions = parsedTransactions.value.filter(t => 
    !t.date || !t.amount || t.amount <= 0 || !t.category_id || !t.transaction_type
  )
  
  if (invalidTransactions.length > 0) {
    ElMessage.warning('请完善所有交易记录的信息（日期、金额、分类等）')
    return
  }

  processing.value = true
  try {
    // 准备保存的数据
    const transactionsToSave = parsedTransactions.value.map(transaction => ({
      date: transaction.date,
      amount: parseFloat(transaction.amount.toString()),
      transaction_type: transaction.transaction_type,
      category_id: parseInt(transaction.category_id!.toString()),
      budget_id: transaction.budget_id ?? null,
      description: transaction.description || ''
    }))

    // 调用保存接口
    const result = await invoke<SaveTransactionsResult>('save_confirmed_transactions', {
      request: {
        transactions: transactionsToSave
      }
    })

    console.log('保存结果:', result)

    if (result.success) {
      ElMessage.success(result.message)
      emit('success', result)
      handleClose()
    } else {
      ElMessage.error(result.message || '保存失败')
    }
  } catch (error) {
    console.error('保存交易记录失败:', error)
    ElMessage.error('保存失败，请稍后重试')
  } finally {
    processing.value = false
  }
}

// 返回编辑界面
const handleBackToEdit = () => {
  showConfirmation.value = false
}

// 关闭对话框
const handleClose = () => {
  if ((inputText.value.trim() || showConfirmation.value) && !processing.value) {
    ElMessageBox.confirm(
      '确定要关闭吗？输入的内容将会丢失。',
      '确认关闭',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    ).then(() => {
      visible.value = false
      inputText.value = ''
      showConfirmation.value = false
      parsedTransactions.value = []

    }).catch(() => {
      // 用户取消关闭
    })
  } else {
    visible.value = false
    inputText.value = ''
    showConfirmation.value = false
    parsedTransactions.value = []
    
  }
}





const getParentCategories = (transactionType: string) => {
  return allCategories.value.filter(cat => cat.type === transactionType && !cat.parent_id)
}

const getSubCategories = (parentId: number) => {
  return allCategories.value.filter(cat => cat.parent_id === parentId)
}


</script>

<style scoped>
.quick-booking-content {
  padding: 8px 0;
}

.input-section {
  margin-bottom: 16px;
}

.input-label {
  display: flex;
  align-items: center;
  margin-bottom: 12px;
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
}

.label-icon {
  margin-right: 8px;
  color: #409eff;
  font-size: 18px;
}

.text-input {
  margin-bottom: 16px;
}

.input-tips {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tip-item {
  display: flex;
  align-items: center;
  font-size: 13px;
  color: #b0b0b0;
}

.tip-item .el-icon {
  margin-right: 6px;
  color: #409eff;
  font-size: 14px;
}

/* 确认界面样式 */
.confirmation-section {
  margin-bottom: 16px;
}

.confirmation-header {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
}

.header-icon {
  margin-right: 8px;
  color: #67c23a;
  font-size: 18px;
}

.confirmation-tip {
  margin-bottom: 16px;
}

.transactions-table {
  margin-top: 16px;
}

.back-btn,
.confirm-btn {
  padding: 8px 20px;
  font-weight: 600;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.cancel-btn,
.clear-btn {
  padding: 8px 20px;
}

.submit-btn {
  padding: 8px 24px;
  font-weight: 600;
}

.submit-btn .el-icon {
  margin-right: 6px;
}

/* 深度样式 */
:deep(.el-dialog) {
  background: #2a2a2a;
  border: 1px solid #404040;
}

:deep(.el-dialog__header) {
  background: #2a2a2a;
  border-bottom: 1px solid #404040;
  padding: 20px 24px 16px;
}

:deep(.el-dialog__title) {
  color: #ffffff;
  font-size: 18px;
  font-weight: 600;
}

:deep(.el-dialog__headerbtn .el-dialog__close) {
  color: #b0b0b0;
}

:deep(.el-dialog__headerbtn .el-dialog__close:hover) {
  color: #ffffff;
}

:deep(.el-dialog__body) {
  background: #2a2a2a;
  color: #ffffff;
  padding: 24px;
}

:deep(.el-dialog__footer) {
  background: #2a2a2a;
  border-top: 1px solid #404040;
  padding: 16px 24px 20px;
}

:deep(.text-input .el-textarea__inner) {
  background: #404040;
  border: 1px solid #606060;
  border-radius: 8px;
  color: #ffffff;
  font-size: 14px;
  line-height: 1.6;
  padding: 16px;
  min-height: 160px;
  resize: vertical;
}

:deep(.text-input .el-textarea__inner:focus) {
  border-color: #409eff;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

:deep(.text-input .el-textarea__inner::placeholder) {
  color: #909399;
  font-size: 13px;
  line-height: 1.5;
}

:deep(.el-input__count) {
  background: transparent;
  color: #b0b0b0;
  font-size: 12px;
}

:deep(.el-button) {
  border-radius: 6px;
  font-weight: 500;
}

:deep(.cancel-btn) {
  background: #404040;
  border-color: #606060;
  color: #ffffff;
}

:deep(.cancel-btn:hover) {
  background: #606060;
  border-color: #808080;
}

:deep(.clear-btn) {
  background: #606060;
  border-color: #707070;
  color: #ffffff;
}

:deep(.clear-btn:hover) {
  background: #707070;
  border-color: #909090;
}

:deep(.submit-btn) {
  background: #409eff;
  border-color: #409eff;
}

:deep(.submit-btn:hover) {
  background: #66b1ff;
  border-color: #66b1ff;
}

:deep(.submit-btn:disabled) {
  background: #404040;
  border-color: #606060;
  color: #909399;
}

/* 分类选择器样式 */
.category-option {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
}

/* 预算选择高亮（空缺提示） */
:deep(.warning-select .el-select__wrapper) {
  border-color: #e6a23c !important;
}

.dash {
  color: #909399;
}
</style> 