<template>
  <el-dialog
    v-model="dialogVisible"
    title="编辑交易记录"
    width="500px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="80px"
      size="large"
    >
      <el-form-item label="类型" prop="type">
        <el-radio-group v-model="form.type" @change="handleTypeChange">
          <el-radio-button label="expense">支出</el-radio-button>
          <el-radio-button label="income">收入</el-radio-button>
        </el-radio-group>
      </el-form-item>
      
      <el-form-item label="金额" prop="amount">
        <el-input
          v-model="form.amount"
          type="number"
          placeholder="请输入金额"
          step="0.01"
          min="0"
        >
          <template #prefix>¥</template>
        </el-input>
      </el-form-item>
      
      <el-form-item label="分类" prop="category_id">
        <el-select 
          v-model="form.category_id" 
          placeholder="请选择分类"
          style="width: 100%"
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

      <!-- 事件预算选择 (仅支出时显示) -->
      <el-form-item v-if="form.type === 'expense'" label="事件预算">
        <el-select
          v-model="form.budget_id"
          placeholder="选择事件预算（可选）"
          style="width: 100%"
          clearable
          filterable
        >
          <el-option
            v-for="budget in store.eventBudgets"
            :key="budget.id"
            :label="budget.name"
            :value="budget.id"
          >
            <div class="budget-option">
              <div class="budget-info">
                <span class="budget-name">{{ budget.name }}</span>
                <span class="budget-category">{{ budget.category_name }}</span>
              </div>
              <div class="budget-amount">
                <span class="budget-remaining" :class="{ 'over-budget': budget.remaining < 0 }">
                  剩余 ¥{{ formatAmount(budget.remaining) }}
                </span>
              </div>
            </div>
          </el-option>
        </el-select>
      </el-form-item>
      
      <el-form-item label="日期" prop="date">
        <el-date-picker
          v-model="form.date"
          type="date"
          placeholder="请选择日期"
          style="width: 100%"
          format="YYYY-MM-DD"
          value-format="YYYY-MM-DD"
        />
      </el-form-item>
      
      <el-form-item label="描述" prop="description">
        <el-input
          v-model="form.description"
          placeholder="请输入描述（可选）"
          maxlength="100"
          show-word-limit
        />
      </el-form-item>
      
      <el-form-item label="备注" prop="note">
        <el-input
          v-model="form.note"
          type="textarea"
          :rows="3"
          placeholder="请输入备注（可选）"
          maxlength="200"
          show-word-limit
        />
      </el-form-item>
    </el-form>
    
    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitting">
          保存
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, nextTick } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { useAppStore, type TransactionWithCategory } from '../stores'
import dayjs from 'dayjs'

interface Props {
  modelValue: boolean
  transaction: TransactionWithCategory | null
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
  type: 'expense' as 'income' | 'expense',
  amount: '',
  category_id: null as number | null,
  budget_id: null as number | null,
  date: dayjs().format('YYYY-MM-DD'),
  description: '',
  note: ''
})

// 表单验证规则
const rules: FormRules = {
  type: [
    { required: true, message: '请选择交易类型', trigger: 'change' }
  ],
  amount: [
    { required: true, message: '请输入金额', trigger: 'blur' },
    { pattern: /^\d+(\.\d{1,2})?$/, message: '请输入正确的金额', trigger: 'blur' }
  ],
  category_id: [
    { required: true, message: '请选择分类', trigger: 'change' }
  ],
  date: [
    { required: true, message: '请选择日期', trigger: 'change' }
  ]
}

// 计算属性
const dialogVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const availableCategories = computed(() => {
  return form.type === 'income' ? store.incomeCategories : store.expenseCategories
})

// 监听类型变化，重置分类选择
watch(() => form.type, () => {
  form.category_id = null
})

// 监听transaction变化，填充表单
watch(() => props.transaction, (transaction) => {
  if (transaction) {
    form.type = transaction.type as 'income' | 'expense'
    form.amount = transaction.amount.toString()
    form.category_id = transaction.category_id
    form.budget_id = transaction.budget_id || null
    form.date = transaction.date
    form.description = transaction.description || ''
    form.note = transaction.note || ''
  }
}, { immediate: true })

// 监听对话框打开
watch(dialogVisible, (visible) => {
  if (visible && props.transaction) {
    // 填充表单数据
    form.type = props.transaction.type as 'income' | 'expense'
    form.amount = props.transaction.amount.toString()
    form.category_id = props.transaction.category_id
    form.budget_id = props.transaction.budget_id || null
    form.date = props.transaction.date
    form.description = props.transaction.description || ''
    form.note = props.transaction.note || ''
    
    nextTick(() => {
      formRef.value?.clearValidate()
    })
  }
})

// 方法
const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  })
}

const handleTypeChange = () => {
  form.category_id = null
  form.budget_id = null
}

const handleClose = () => {
  dialogVisible.value = false
}

const handleSubmit = async () => {
  if (!formRef.value || !props.transaction) return
  
  try {
    const valid = await formRef.value.validate()
    if (!valid) return
    
    submitting.value = true
    
    await store.updateTransaction(props.transaction.id, {
      type: form.type,
      amount: parseFloat(form.amount),
      category_id: form.category_id!,
      budget_id: form.budget_id,
      date: form.date,
      description: form.description || undefined,
      note: form.note || undefined
    })
    
    ElMessage.success('更新成功')
    emit('success')
    dialogVisible.value = false
  } catch (error) {
    console.error('更新交易记录失败:', error)
    ElMessage.error('更新失败，请重试')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.category-option {
  display: flex;
  align-items: center;
}

.category-icon {
  margin-right: 8px;
  font-size: 16px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

:deep(.el-radio-button__inner) {
  padding: 8px 20px;
}

/* 预算选择样式 */
.budget-option {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
}

.budget-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.budget-name {
  font-size: 14px;
  font-weight: 500;
  color: #ffffff;
}

.budget-category {
  font-size: 12px;
  color: #b0b0b0;
}

.budget-amount {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}

.budget-remaining {
  font-size: 12px;
  color: #409eff;
}

.budget-remaining.over-budget {
  color: #f56c6c;
}

:deep(.el-input-number) {
  width: 100%;
}
</style> 