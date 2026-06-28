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
        <div class="input-header">
          <div class="input-label">
            <el-icon class="label-icon"><EditPen /></el-icon>
            <span>请描述您的收支情况</span>
          </div>
          <el-select
            v-if="llmConfigs.length > 0"
            v-model="selectedConfigId"
            class="model-select"
            size="small"
            placeholder="选择模型"
          >
            <el-option
              v-for="cfg in llmConfigs"
              :key="cfg.id"
              :label="`${cfg.config_name || cfg.model}`"
              :value="cfg.id"
            >
              <div class="model-option">
                <span class="model-option-name">{{ cfg.config_name || cfg.model }}</span>
                <span class="model-option-provider">{{ cfg.provider }}</span>
              </div>
            </el-option>
          </el-select>
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
interface LLMConfig {
  id: number
  config_name: string
  provider: string
  model: string
  is_active: boolean
}

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
const llmConfigs = ref<LLMConfig[]>([])
const selectedConfigId = ref<number | null>(null)

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
      // 加载分类和模型配置数据
      loadCategories()
      loadLLMConfigs()
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

// 加载 LLM 配置
const loadLLMConfigs = async () => {
  try {
    llmConfigs.value = await invoke<LLMConfig[]>('get_llm_configs')
    const active = llmConfigs.value.find(c => c.is_active)
    if (active) selectedConfigId.value = active.id
    else if (llmConfigs.value.length > 0) selectedConfigId.value = llmConfigs.value[0].id
    else selectedConfigId.value = null
  } catch (error) {
    console.error('加载模型配置失败:', error)
    llmConfigs.value = []
    selectedConfigId.value = null
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
      request: {
        text: inputText.value.trim(),
        config_id: selectedConfigId.value,
      }
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

.input-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  gap: 12px;
}

.input-label {
  display: flex;
  align-items: center;
  font-size: 16px;
  font-weight: 600;
  color: var(--ms-text-primary);
}

.label-icon {
  margin-right: 8px;
  color: var(--ms-info);
  font-size: 18px;
}

.model-select {
  width: 180px;
  flex-shrink: 0;
}

.model-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.model-option-name {
  font-size: 13px;
  color: var(--ms-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-option-provider {
  font-size: 11px;
  color: var(--ms-text-tertiary);
  flex-shrink: 0;
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
  color: var(--ms-text-secondary);
}

.tip-item .el-icon {
  margin-right: 6px;
  color: var(--ms-info);
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
  color: var(--ms-text-primary);
}

.header-icon {
  margin-right: 8px;
  color: var(--ms-income);
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
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-default);
}

:deep(.el-dialog__header) {
  background: var(--ms-surface-primary);
  border-bottom: 1px solid var(--ms-border-default);
  padding: 20px 24px 16px;
}

:deep(.el-dialog__title) {
  color: var(--ms-text-primary);
  font-size: 18px;
  font-weight: 600;
}

:deep(.el-dialog__headerbtn .el-dialog__close) {
  color: var(--ms-text-secondary);
}

:deep(.el-dialog__headerbtn .el-dialog__close:hover) {
  color: var(--ms-text-primary);
}

:deep(.el-dialog__body) {
  background: var(--ms-surface-primary);
  color: var(--ms-text-primary);
  padding: 24px;
}

:deep(.el-dialog__footer) {
  background: var(--ms-surface-primary);
  border-top: 1px solid var(--ms-border-default);
  padding: 16px 24px 20px;
}

:deep(.text-input .el-textarea__inner) {
  background: var(--ms-border-default);
  border: 1px solid var(--ms-border-default);
  border-radius: 8px;
  color: var(--ms-text-primary);
  font-size: 14px;
  line-height: 1.6;
  padding: 16px;
  min-height: 160px;
  resize: vertical;
}

:deep(.text-input .el-textarea__inner:focus) {
  border-color: var(--ms-info);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

:deep(.text-input .el-textarea__inner::placeholder) {
  color: var(--ms-text-tertiary);
  font-size: 13px;
  line-height: 1.5;
}

:deep(.el-input__count) {
  background: transparent;
  color: var(--ms-text-secondary);
  font-size: 12px;
}

:deep(.el-button) {
  border-radius: 6px;
  font-weight: 500;
}

:deep(.cancel-btn) {
  background: var(--ms-border-default);
  border-color: var(--ms-border-default);
  color: var(--ms-text-primary);
}

:deep(.cancel-btn:hover) {
  background: var(--ms-border-default);
  border-color: var(--ms-text-secondary);
}

:deep(.clear-btn) {
  background: var(--ms-border-default);
  border-color: var(--ms-border-default);
  color: var(--ms-text-primary);
}

:deep(.clear-btn:hover) {
  background: var(--ms-border-default);
  border-color: var(--ms-text-secondary);
}

:deep(.submit-btn) {
  background: var(--ms-info);
  border-color: var(--ms-info);
}

:deep(.submit-btn:hover) {
  background: var(--ms-info);
  border-color: var(--ms-info);
}

:deep(.submit-btn:disabled) {
  background: var(--ms-border-default);
  border-color: var(--ms-border-default);
  color: var(--ms-text-tertiary);
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
  border-color: var(--ms-warning) !important;
}

.dash {
  color: var(--ms-text-tertiary);
}
</style> 