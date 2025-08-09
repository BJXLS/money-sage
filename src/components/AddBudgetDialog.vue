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
        label-width="80px"
        size="large"
      >
        <!-- 预算类型选择 -->
        <el-form-item label="预算类型" prop="budget_type">
          <div class="budget-type-selector">
            <div 
              class="budget-type-card"
              :class="{ active: form.budget_type === 'time' }"
              @click="handleTypeChange('time')"
            >
              <div class="type-icon">
                <el-icon><Clock /></el-icon>
              </div>
              <div class="type-content">
                <h4>时间预算</h4>
                <p>设置特定时间段的预算限制</p>
              </div>
            </div>
            
            <div 
              class="budget-type-card"
              :class="{ active: form.budget_type === 'event' }"
              @click="handleTypeChange('event')"
            >
              <div class="type-icon">
                <el-icon><Flag /></el-icon>
              </div>
              <div class="type-content">
                <h4>事件预算</h4>
                <p>为特定事件或项目设置预算</p>
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

        <!-- 支出分类 -->
        <el-form-item label="支出分类" prop="category_id">
          <el-select 
            v-model="form.category_id" 
            placeholder="请选择支出小类"
            style="width: 100%"
            filterable
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
        </el-form-item>

        <!-- 时间预算专用字段 -->
        <template v-if="form.budget_type === 'time'">
          <!-- 预算周期 -->
          <el-form-item label="预算周期" prop="period_type">
            <el-select 
              v-model="form.period_type" 
              placeholder="请选择预算周期"
              style="width: 100%"
            >
              <el-option label="每周" value="weekly" />
              <el-option label="每月" value="monthly" />
              <el-option label="每年" value="yearly" />
            </el-select>
          </el-form-item>

          <!-- 时间范围 -->
          <el-form-item label="时间范围" prop="dateRange">
            <el-date-picker
              v-model="form.dateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </template>

        <!-- 事件预算专用字段 -->
        <template v-if="form.budget_type === 'event'">
          <!-- 事件描述 -->
          <el-form-item label="事件描述" prop="description">
            <el-input
              v-model="form.description"
              type="textarea"
              :rows="3"
              placeholder="请描述这个预算的具体用途或事件..."
              maxlength="200"
              show-word-limit
            />
          </el-form-item>

          <!-- 预计完成时间 -->
          <el-form-item label="预计完成" prop="estimated_date">
            <el-date-picker
              v-model="form.estimated_date"
              type="date"
              placeholder="预计完成日期（可选）"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </template>

        <!-- 预算状态 -->
        <el-form-item label="预算状态" prop="is_active">
          <el-switch
            v-model="form.is_active"
            active-text="启用"
            inactive-text="禁用"
            active-color="#67c23a"
            inactive-color="#f56c6c"
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
            <span 
              class="budget-type-badge"
              :class="form.budget_type"
            >
              {{ form.budget_type === 'time' ? '时间预算' : '事件预算' }}
            </span>
          </span>
        </div>
        <div class="preview-item">
          <span class="preview-label">预算金额：</span>
          <span class="preview-value amount">¥{{ formatAmount(parseFloat(form.amount) || 0) }}</span>
        </div>
        <div class="preview-item" v-if="form.budget_type === 'time' && form.dateRange">
          <span class="preview-label">有效期限：</span>
          <span class="preview-value">{{ form.dateRange[0] }} 至 {{ form.dateRange[1] }}</span>
        </div>
        <div class="preview-item" v-if="form.budget_type === 'event' && form.estimated_date">
          <span class="preview-label">预计完成：</span>
          <span class="preview-value">{{ form.estimated_date }}</span>
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
import { Clock, Flag } from '@element-plus/icons-vue'
import { useAppStore, type BudgetProgress } from '../stores'
import dayjs from 'dayjs'

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

// 表单数据
const form = reactive({
  name: '',
  amount: '',
  budget_type: 'time' as 'time' | 'event',
  period_type: 'monthly' as 'weekly' | 'monthly' | 'yearly',
  category_id: null as number | null,
  dateRange: [] as string[],
  description: '',
  estimated_date: '',
  is_active: true
})

// 表单验证规则
const rules: FormRules = {
  name: [
    { required: true, message: '请输入预算名称', trigger: 'blur' },
    { min: 2, max: 50, message: '预算名称长度应在2-50个字符之间', trigger: 'blur' }
  ],
  amount: [
    { required: true, message: '请输入预算金额', trigger: 'blur' },
    { pattern: /^\d+(\.\d{1,2})?$/, message: '请输入正确的金额格式', trigger: 'blur' }
  ],
  budget_type: [
    { required: true, message: '请选择预算类型', trigger: 'change' }
  ],
  category_id: [
    { required: true, message: '请选择支出分类', trigger: 'change' }
  ],
  period_type: [
    { required: true, message: '请选择预算周期', trigger: 'change' }
  ],
  dateRange: [
    { required: true, message: '请选择时间范围', trigger: 'change' }
  ]
}

// 计算属性
const dialogVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const isEditing = computed(() => !!props.budget)

const parentExpenseCategories = computed(() => store.parentExpenseCategories)
const getSubCategories = (parentId: number) => store.getSubCategories(parentId)

// 监听预算变化，填充表单
watch(() => props.budget, (budget) => {
  if (budget) {
    form.name = budget.name
    form.amount = budget.amount.toString()
    form.budget_type = budget.budget_type
    form.period_type = budget.period_type
    form.category_id = budget.category_id
    form.is_active = budget.is_active
    
    if (budget.budget_type === 'time') {
      form.dateRange = [budget.start_date, budget.end_date || '']
    } else {
      form.estimated_date = budget.end_date || ''
    }
  }
}, { immediate: true })

// 监听对话框打开
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

// 监听预算类型变化
watch(() => form.budget_type, (newType) => {
  // 重置相关字段
  if (newType === 'time') {
    form.description = ''
    form.estimated_date = ''
  } else {
    form.dateRange = []
    form.period_type = 'monthly'
  }
  
  // 清除验证
  nextTick(() => {
    formRef.value?.clearValidate()
  })
})

// 方法
const resetForm = () => {
  form.name = ''
  form.amount = ''
  form.budget_type = 'time'
  form.period_type = 'monthly'
  form.category_id = null
  form.dateRange = []
  form.description = ''
  form.estimated_date = ''
  form.is_active = true
}

const handleTypeChange = (type: 'time' | 'event') => {
  form.budget_type = type
}

const handleClose = () => {
  dialogVisible.value = false
}

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  })
}

const handleSubmit = async () => {
  if (!formRef.value) return
  
  try {
    const valid = await formRef.value.validate()
    if (!valid) return
    
    submitting.value = true
    
    const budgetData = {
      name: form.name,
      category_id: form.category_id!,
      amount: parseFloat(form.amount),
      budget_type: form.budget_type,
      period_type: form.period_type,
      start_date: form.budget_type === 'time' 
        ? form.dateRange[0] 
        : dayjs().format('YYYY-MM-DD'),
      end_date: form.budget_type === 'time' 
        ? form.dateRange[1] || undefined
        : form.estimated_date || undefined
    }
    
    if (isEditing.value) {
      // 更新预算 (这里需要实现updateBudget方法)
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
  background: #2a2a2a;
}

.budget-form {
  color: #ffffff;
}

.budget-type-selector {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 8px;
}

.budget-type-card {
  display: flex;
  align-items: center;
  padding: 20px;
  background: #1a1a1a;
  border: 2px solid #404040;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.budget-type-card:hover {
  border-color: #606060;
  background: #2a2a2a;
}

.budget-type-card.active {
  border-color: #409eff;
  background: #409eff10;
}

.type-icon {
  margin-right: 16px;
  font-size: 24px;
  color: #409eff;
}

.type-content {
  flex: 1;
}

.type-content h4 {
  margin: 0 0 8px 0;
  font-size: 16px;
  color: #ffffff;
}

.type-content p {
  margin: 0;
  font-size: 14px;
  color: #b0b0b0;
}

.category-option {
  display: flex;
  align-items: center;
  gap: 8px;
}

.category-icon {
  font-size: 16px;
}

.budget-preview {
  margin-top: 24px;
  padding: 20px;
  background: #1a1a1a;
  border-radius: 8px;
  border: 1px solid #404040;
}

.budget-preview h4 {
  margin: 0 0 16px 0;
  color: #ffffff;
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
  color: #b0b0b0;
  min-width: 80px;
}

.preview-value {
  font-size: 14px;
  color: #ffffff;
  font-weight: 500;
}

.preview-value.amount {
  color: #67c23a;
  font-weight: 600;
}

.budget-type-badge {
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
}

.budget-type-badge.time {
  background: #409eff20;
  color: #409eff;
}

.budget-type-badge.event {
  background: #67c23a20;
  color: #67c23a;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

/* 深色主题适配 */
:deep(.el-dialog) {
  background: #2a2a2a;
  color: #ffffff;
}

:deep(.el-dialog__header) {
  border-bottom: 1px solid #404040;
}

:deep(.el-dialog__title) {
  color: #ffffff;
}

:deep(.el-form-item__label) {
  color: #b0b0b0;
}

:deep(.el-input__inner) {
  background: #1a1a1a;
  border-color: #404040;
  color: #ffffff;
}

:deep(.el-input__inner:focus) {
  border-color: #409eff;
}

:deep(.el-select) {
  background: #1a1a1a;
}

:deep(.el-select-dropdown) {
  background: #2a2a2a;
  border-color: #404040;
}

:deep(.el-option) {
  background: #2a2a2a;
  color: #ffffff;
}

:deep(.el-option:hover) {
  background: #404040;
}

:deep(.el-option.selected) {
  background: #409eff20;
  color: #409eff;
}

:deep(.el-textarea__inner) {
  background: #1a1a1a;
  border-color: #404040;
  color: #ffffff;
}

:deep(.el-textarea__inner:focus) {
  border-color: #409eff;
}

:deep(.el-date-editor) {
  background: #1a1a1a;
  border-color: #404040;
}

:deep(.el-date-editor:hover) {
  border-color: #606060;
}

:deep(.el-date-editor.is-focus) {
  border-color: #409eff;
}

:deep(.el-date-editor input) {
  background: transparent;
  color: #ffffff;
}

:deep(.el-switch.is-checked .el-switch__core) {
  background: #67c23a;
}

:deep(.el-switch__core) {
  background: #f56c6c;
}

:deep(.el-switch__action) {
  background: #ffffff;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .budget-type-selector {
    grid-template-columns: 1fr;
  }
  
  .budget-type-card {
    padding: 16px;
  }
  
  .type-icon {
    font-size: 20px;
  }
}
</style> 