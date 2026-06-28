<template>
  <el-dialog
    v-model="dialogVisible"
    :title="isEditing ? '编辑预算' : '添加预算'"
    width="600px"
    :close-on-click-modal="false"
    @close="handleClose"
    class="budget-dialog"
  >
    <div class="budget-form">
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="90px"
        size="large"
      >
        <!-- 预算类型选择 -->
        <el-form-item label="预算类型" prop="uiType">
          <div class="budget-type-selector">
            <div
              class="budget-type-card"
              :class="{ active: form.uiType === 'total' }"
              @click="handleTypeChange('total')"
            >
              <div class="type-icon">🏦</div>
              <div class="type-content">
                <h4>总预算</h4>
                <p>控制整体支出</p>
              </div>
            </div>

            <div
              class="budget-type-card"
              :class="{ active: form.uiType === 'category' }"
              @click="handleTypeChange('category')"
            >
              <div class="type-icon">🏷️</div>
              <div class="type-content">
                <h4>分类预算</h4>
                <p>限制某个分类支出</p>
              </div>
            </div>

            <div
              class="budget-type-card"
              :class="{ active: form.uiType === 'event' }"
              @click="handleTypeChange('event')"
            >
              <div class="type-icon">🎯</div>
              <div class="type-content">
                <h4>事件预算</h4>
                <p>一次性事件或项目</p>
              </div>
            </div>
          </div>
        </el-form-item>

        <!-- 预算名称 -->
        <el-form-item label="预算名称" prop="name">
          <el-input
            v-model="form.name"
            placeholder="请输入预算名称"
            clearable
          />
        </el-form-item>

        <!-- 支出分类 -->
        <el-form-item
          v-if="form.uiType === 'category'"
          label="关联分类"
          prop="category_id"
        >
          <el-select
            v-model="form.category_id"
            placeholder="请选择支出分类"
            style="width: 100%"
            filterable
            @change="handleCategoryChange"
          >
            <el-option-group
              v-for="parent in parentExpenseCategories"
              :key="parent.id"
              :label="`${parent.icon || '📁'} ${parent.name}`"
            >
              <el-option
                v-for="sub in getSubCategories(parent.id)"
                :key="sub.id"
                :label="sub.name"
                :value="sub.id"
              >
                <div class="category-option">
                  <span class="category-icon" :style="{ color: sub.color }">
                    {{ sub.icon || '📋' }}
                  </span>
                  <span>{{ sub.name }}</span>
                </div>
              </el-option>
            </el-option-group>
          </el-select>
          <div v-if="recommendedAmount !== null" class="recommend-row">
            <span class="recommend-tag" @click="applyRecommendedAmount">
              💡 推荐金额 ¥{{ formatAmount(recommendedAmount) }}
            </span>
            <span class="recommend-hint">基于近 3 个月平均支出的 90%</span>
          </div>
        </el-form-item>

        <!-- 预算金额 -->
        <el-form-item label="预算金额" prop="amount">
          <el-input
            v-model="form.amount"
            type="number"
            placeholder="请输入预算金额"
            step="0.01"
            min="0"
          >
            <template #prefix>¥</template>
          </el-input>
        </el-form-item>

        <!-- 周期与自动续期（总预算/分类预算） -->
        <template v-if="form.uiType !== 'event'">
          <el-form-item label="预算周期" prop="period_type">
            <el-select
              v-model="form.period_type"
              placeholder="请选择预算周期"
              style="width: 100%"
            >
              <el-option label="每日" value="daily" />
              <el-option label="每周" value="weekly" />
              <el-option label="每月" value="monthly" />
              <el-option label="每年" value="yearly" />
            </el-select>
          </el-form-item>

          <el-form-item label="自动滚动" prop="is_recurring">
            <el-switch
              v-model="form.is_recurring"
              active-text="周期结束后自动续期"
              inactive-text="不自动续期"
            />
          </el-form-item>
        </template>

        <!-- 事件时间范围（事件预算） -->
        <template v-if="form.uiType === 'event'">
          <el-form-item label="事件时间" prop="eventDateRange">
            <div class="event-date-row">
              <el-date-picker
                v-model="form.start_date"
                type="date"
                placeholder="开始日期（可选）"
                format="YYYY-MM-DD"
                value-format="YYYY-MM-DD"
                class="event-date-picker"
              />
              <span class="date-separator">至</span>
              <el-date-picker
                v-model="form.end_date"
                type="date"
                placeholder="结束日期（可选）"
                format="YYYY-MM-DD"
                value-format="YYYY-MM-DD"
                class="event-date-picker"
              />
            </div>
            <p class="form-tip">
              事件预算不限制交易日期和分类，关联的交易会全部计入预算进度。
            </p>
          </el-form-item>
        </template>

        <!-- 预算状态 -->
        <el-form-item label="预算状态" prop="is_active">
          <el-switch
            v-model="form.is_active"
            active-text="启用"
            inactive-text="停用"
          />
        </el-form-item>
      </el-form>
    </div>

    <!-- 预算预览 -->
    <div class="budget-preview" v-if="form.name && form.amount">
      <h4>预算预览</h4>
      <div class="preview-content">
        <div class="preview-item">
          <span class="preview-label">预算名称：</span>
          <span class="preview-value">{{ form.name }}</span>
        </div>
        <div class="preview-item">
          <span class="preview-label">预算类型：</span>
          <span class="preview-value">
            <span class="budget-type-badge" :class="form.uiType">
              {{ typeLabelMap[form.uiType] }}
            </span>
          </span>
        </div>
        <div class="preview-item">
          <span class="preview-label">预算金额：</span>
          <span class="preview-value amount">
            ¥{{ formatAmount(parseFloat(form.amount) || 0) }}
          </span>
        </div>
        <div class="preview-item" v-if="form.uiType === 'category' && selectedCategoryName">
          <span class="preview-label">关联分类：</span>
          <span class="preview-value">{{ selectedCategoryName }}</span>
        </div>
        <div class="preview-item" v-if="form.uiType !== 'event' && form.period_type">
          <span class="preview-label">预算周期：</span>
          <span class="preview-value">{{ periodLabelMap[form.period_type] }}</span>
        </div>
        <div class="preview-item" v-if="form.uiType === 'event' && (form.start_date || form.end_date)">
          <span class="preview-label">事件时间：</span>
          <span class="preview-value">{{ eventDatePreview }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose" size="large">取消</el-button>
        <el-button
          type="primary"
          @click="handleSubmit"
          :loading="submitting"
          size="large"
        >
          {{ isEditing ? '更新预算' : '创建预算' }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, nextTick } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { useAppStore, type BudgetProgress } from '../stores'
import dayjs from 'dayjs'

type BudgetUiType = 'total' | 'category' | 'event'

interface Props {
  modelValue: boolean
  budget?: BudgetProgress | null
}

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  (e: 'success'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const store = useAppStore()
const formRef = ref<FormInstance>()
const submitting = ref(false)
const recentTransactions = ref<any[]>([])

const typeLabelMap: Record<BudgetUiType, string> = {
  total: '总预算',
  category: '分类预算',
  event: '事件预算',
}

const periodLabelMap: Record<string, string> = {
  daily: '每日',
  weekly: '每周',
  monthly: '每月',
  yearly: '每年',
}

const form = reactive({
  name: '',
  amount: '',
  uiType: 'category' as BudgetUiType,
  period_type: 'monthly' as 'daily' | 'weekly' | 'monthly' | 'yearly' | null,
  category_id: null as number | null,
  start_date: '' as string | null,
  end_date: '' as string | null,
  is_recurring: true,
  is_active: true,
})

const getUiTypeFromBudget = (budget: BudgetProgress): BudgetUiType => {
  if (budget.budget_type === 'total') return 'total'
  if (budget.budget_type === 'event') return 'event'
  return 'category'
}

const rules: FormRules = {
  name: [
    { required: true, message: '请输入预算名称', trigger: 'blur' },
    { min: 1, max: 50, message: '预算名称长度应在1-50个字符之间', trigger: 'blur' },
  ],
  amount: [
    { required: true, message: '请输入预算金额', trigger: 'blur' },
    {
      validator: (_rule: any, value: any, callback: any) => {
        const num = parseFloat(value)
        if (isNaN(num) || num <= 0) {
          callback(new Error('请输入大于0的金额'))
        } else {
          callback()
        }
      },
      trigger: 'blur',
    },
  ],
  category_id: [
    {
      required: true,
      message: '请选择关联分类',
      trigger: 'change',
      validator: (_rule: any, value: any, callback: any) => {
        if (form.uiType === 'category' && !value) {
          callback(new Error('请选择关联分类'))
        } else {
          callback()
        }
      },
    },
  ],
  period_type: [
    {
      required: true,
      message: '请选择预算周期',
      trigger: 'change',
      validator: (_rule: any, value: any, callback: any) => {
        if (form.uiType !== 'event' && !value) {
          callback(new Error('请选择预算周期'))
        } else {
          callback()
        }
      },
    },
  ],
  eventDateRange: [
    {
      validator: (_rule: any, _value: any, callback: any) => {
        if (form.uiType === 'event' && form.start_date && form.end_date) {
          if (dayjs(form.end_date).isBefore(dayjs(form.start_date))) {
            callback(new Error('结束日期不能早于开始日期'))
            return
          }
        }
        callback()
      },
      trigger: 'change',
    },
  ],
}

const dialogVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})

const isEditing = computed(() => !!props.budget)

const parentExpenseCategories = computed(() => store.parentExpenseCategories)
const getSubCategories = (parentId: number) => store.getSubCategories(parentId)

const selectedCategoryName = computed(() => {
  if (!form.category_id) return ''
  const cat = store.categories.find((c) => c.id === form.category_id)
  return cat?.name || ''
})

const recommendedAmount = computed(() => {
  if (form.uiType !== 'category' || !form.category_id || recentTransactions.value.length === 0) {
    return null
  }
  const threeMonthsAgo = dayjs().subtract(3, 'month').startOf('day')
  const categoryTx = recentTransactions.value.filter(
    (t) =>
      t.type === 'expense' &&
      t.category_id === form.category_id &&
      dayjs(t.date).isAfter(threeMonthsAgo)
  )
  if (categoryTx.length === 0) return null
  const total = categoryTx.reduce((sum, t) => sum + t.amount, 0)
  // 按近 3 个月总支出平均到每月，再取 90%
  const avgMonthly = total / 3
  return Math.round(avgMonthly * 0.9 * 100) / 100
})

const eventDatePreview = computed(() => {
  const start = form.start_date || '未设置'
  const end = form.end_date || '未设置'
  return `${start} 至 ${end}`
})

const applyRecommendedAmount = () => {
  if (recommendedAmount.value !== null) {
    form.amount = recommendedAmount.value.toString()
  }
}

const fetchRecentTransactions = async () => {
  try {
    const end = dayjs().format('YYYY-MM-DD')
    const start = dayjs().subtract(3, 'month').format('YYYY-MM-DD')
    const list = await store.fetchTransactionsByDateRange(start, end)
    recentTransactions.value = list || []
  } catch (e) {
    recentTransactions.value = []
  }
}

const handleCategoryChange = () => {
  if (form.uiType === 'category' && form.category_id) {
    fetchRecentTransactions()
  }
}

const resetForm = () => {
  form.name = ''
  form.amount = ''
  form.uiType = 'category'
  form.period_type = 'monthly'
  form.category_id = null
  form.start_date = null
  form.end_date = null
  form.is_recurring = true
  form.is_active = true
  recentTransactions.value = []
}

const handleTypeChange = (type: BudgetUiType) => {
  form.uiType = type
  if (type === 'event') {
    form.period_type = null
    form.is_recurring = false
    form.category_id = null
  } else {
    form.period_type = form.period_type || 'monthly'
    form.is_recurring = true
  }
  nextTick(() => {
    formRef.value?.clearValidate()
  })
}

const handleClose = () => {
  dialogVisible.value = false
}

const fillFormFromBudget = (budget: BudgetProgress) => {
  form.name = budget.name
  form.amount = budget.amount.toString()
  form.uiType = getUiTypeFromBudget(budget)
  form.period_type = budget.period_type || 'monthly'
  form.category_id = budget.category_id ?? null
  form.start_date = budget.start_date || null
  form.end_date = budget.end_date || null
  form.is_recurring = budget.is_recurring
  form.is_active = budget.is_active
}

watch(
  () => props.budget,
  (budget) => {
    if (budget) {
      fillFormFromBudget(budget)
      if (budget.category_id) {
        fetchRecentTransactions()
      }
    } else {
      resetForm()
    }
  },
  { immediate: true }
)

watch(dialogVisible, (visible) => {
  if (visible) {
    if (!props.budget) {
      resetForm()
    }
    nextTick(() => {
      formRef.value?.clearValidate()
    })
  }
})

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', {
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  })
}

const handleSubmit = async () => {
  if (!formRef.value) return

  try {
    const valid = await formRef.value.validate()
    if (!valid) return

    submitting.value = true

    const budget_type =
      form.uiType === 'total' ? 'total' : form.uiType === 'event' ? 'event' : 'time'

    const budgetData = {
      name: form.name,
      category_id: form.uiType === 'category' ? form.category_id : null,
      amount: parseFloat(form.amount),
      budget_type: budget_type as 'time' | 'event' | 'total',
      period_type: form.uiType === 'event' ? null : form.period_type,
      start_date: form.uiType === 'event' ? form.start_date || null : null,
      end_date: form.uiType === 'event' ? form.end_date || null : null,
      is_recurring: form.uiType !== 'event' && form.is_recurring,
      is_active: form.is_active,
    }

    if (isEditing.value && props.budget) {
      await store.updateBudget(props.budget.id, budgetData)
      ElMessage.success('预算更新成功')
    } else {
      await store.createBudget(budgetData)
      ElMessage.success('预算创建成功')
    }

    emit('success')
    dialogVisible.value = false
  } catch (error) {
    console.error('操作预算失败:', error)
    ElMessage.error('操作失败，请重试')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.budget-dialog {
  background: var(--ms-surface-primary);
}

.budget-form {
  color: var(--ms-text-primary);
}

.budget-type-selector {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 8px;
}

.budget-type-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 12px;
  background: var(--ms-bg-secondary);
  border: 2px solid var(--ms-border-default);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: center;
}

.budget-type-card:hover {
  border-color: var(--ms-border-default);
  background: var(--ms-surface-primary);
}

.budget-type-card.active {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}

.type-icon {
  font-size: 28px;
  margin-bottom: 8px;
}

.type-content h4 {
  margin: 0 0 4px 0;
  font-size: 14px;
  color: var(--ms-text-primary);
}

.type-content p {
  margin: 0;
  font-size: 12px;
  color: var(--ms-text-secondary);
}

.category-option {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
}

.recommend-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

.recommend-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 12px;
  background: rgba(16, 185, 129, 0.1);
  color: var(--ms-income);
  cursor: pointer;
  transition: opacity 0.2s ease;
}

.recommend-tag:hover {
  opacity: 0.8;
}

.recommend-hint {
  font-size: 12px;
  color: var(--ms-text-tertiary);
}

.event-date-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.event-date-picker {
  flex: 1;
}

.date-separator {
  color: var(--ms-text-secondary);
  font-size: 14px;
  flex-shrink: 0;
}

.form-tip {
  margin: 8px 0 0 0;
  font-size: 12px;
  color: var(--ms-text-tertiary);
  line-height: 1.5;
}

.budget-preview {
  margin-top: 24px;
  padding: 20px;
  background: var(--ms-bg-secondary);
  border-radius: var(--ms-radius-lg);
  border: 1px solid var(--ms-border-default);
}

.budget-preview h4 {
  margin: 0 0 16px 0;
  color: var(--ms-text-primary);
  font-size: 16px;
}

.preview-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.preview-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.preview-label {
  font-size: 14px;
  color: var(--ms-text-secondary);
  min-width: 80px;
}

.preview-value {
  font-size: 14px;
  color: var(--ms-text-primary);
  font-weight: 500;
}

.preview-value.amount {
  color: var(--ms-income);
  font-weight: 600;
}

.budget-type-badge {
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
}

.budget-type-badge.total {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

.budget-type-badge.category {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
}

.budget-type-badge.event {
  background: rgba(139, 92, 246, 0.1);
  color: #8b5cf6;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

:deep(.el-dialog) {
  background: var(--ms-surface-primary);
  color: var(--ms-text-primary);
}

:deep(.el-dialog__header) {
  border-bottom: 1px solid var(--ms-border-default);
}

:deep(.el-dialog__title) {
  color: var(--ms-text-primary);
}

:deep(.el-form-item__label) {
  color: var(--ms-text-secondary);
}

:deep(.el-input__inner) {
  background: var(--ms-bg-secondary);
  border-color: var(--ms-border-default);
  color: var(--ms-text-primary);
}

:deep(.el-input__inner:focus) {
  border-color: var(--el-color-primary);
}

:deep(.el-select) {
  background: var(--ms-bg-secondary);
}

:deep(.el-select-dropdown) {
  background: var(--ms-surface-primary);
  border-color: var(--ms-border-default);
}

:deep(.el-option) {
  background: var(--ms-surface-primary);
  color: var(--ms-text-primary);
}

:deep(.el-option:hover) {
  background: var(--ms-surface-hover);
}

:deep(.el-option.selected) {
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

:deep(.el-textarea__inner) {
  background: var(--ms-bg-secondary);
  border-color: var(--ms-border-default);
  color: var(--ms-text-primary);
}

:deep(.el-textarea__inner:focus) {
  border-color: var(--el-color-primary);
}

:deep(.el-date-editor) {
  background: var(--ms-bg-secondary);
  border-color: var(--ms-border-default);
  width: 100%;
}

:deep(.el-date-editor:hover) {
  border-color: var(--ms-border-default);
}

:deep(.el-date-editor.is-focus) {
  border-color: var(--el-color-primary);
}

:deep(.el-date-editor input) {
  background: transparent;
  color: var(--ms-text-primary);
}

@media (max-width: 768px) {
  .budget-type-selector {
    grid-template-columns: 1fr;
  }

  .event-date-row {
    flex-direction: column;
    align-items: stretch;
  }

  .date-separator {
    text-align: center;
  }
}
</style>
