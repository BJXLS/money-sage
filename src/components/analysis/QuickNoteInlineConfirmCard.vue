<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { QuickNoteDraft } from '../../stores'

const props = defineProps<{
  draft: QuickNoteDraft
}>()
const emit = defineEmits<{
  (e: 'confirm', payload: { draftId: string; token: string; items: any[] }): void
  (e: 'cancel', draftId: string): void
}>()

interface Category {
  id: number
  name: string
  type: 'income' | 'expense'
  icon?: string
  color?: string
  parent_id?: number | null
}

interface BudgetProgress {
  id: number
  name: string
  category_name: string
  is_active: boolean
}

const form = reactive({
  items: props.draft.items.map(i => ({ ...i }))
})
const allCategories = ref<Category[]>([])
const allBudgets = ref<BudgetProgress[]>([])

const loadMeta = async () => {
  const [categories, budgets] = await Promise.all([
    invoke<Category[]>('get_categories'),
    invoke<BudgetProgress[]>('get_budgets')
  ])
  allCategories.value = categories || []
  allBudgets.value = (budgets || []).filter(b => b.is_active)
}

onMounted(loadMeta)

const getParentCategories = (transactionType: string) =>
  allCategories.value.filter(cat => cat.type === transactionType && !cat.parent_id)

const getSubCategories = (parentId: number) =>
  allCategories.value.filter(cat => cat.parent_id === parentId)

const onConfirm = () => {
  const invalid = form.items.find(i => !i.date || !i.amount || i.amount <= 0 || !i.category_id)
  if (invalid) {
    ElMessage.warning('请完善每条记录的日期、金额和分类')
    return
  }
  emit('confirm', {
    draftId: props.draft.draft_id,
    token: props.draft.confirmation_token || '',
    items: form.items.map(i => ({
      date: i.date,
      amount: i.amount,
      transaction_type: i.transaction_type,
      category_id: i.category_id ?? null,
      budget_id: i.budget_id ?? null,
      description: i.description || '',
    })),
  })
}
</script>

<template>
  <div class="draft-card">
    <div class="title">待确认记账草稿（{{ draft.draft_id.slice(0, 8) }}）</div>
    <el-table :data="form.items" size="small">
      <el-table-column prop="date" label="日期" width="120">
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
      <el-table-column prop="amount" label="金额" width="120">
        <template #default="{ row }"><el-input-number v-model="row.amount" :min="0.01" :precision="2" /></template>
      </el-table-column>
      <el-table-column prop="transaction_type" label="类型" width="120">
        <template #default="{ row }">
          <el-select v-model="row.transaction_type">
            <el-option label="支出" value="expense" />
            <el-option label="收入" value="income" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column prop="category_id" label="分类" width="180">
        <template #default="{ row }">
          <el-select
            v-model="row.category_id"
            filterable
            placeholder="选择分类"
            size="small"
            style="width: 100%"
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
              />
            </el-option-group>
          </el-select>
        </template>
      </el-table-column>
      <el-table-column prop="budget_id" label="预算" width="200">
        <template #default="{ row }">
          <el-select
            v-if="row.transaction_type === 'expense'"
            v-model="row.budget_id"
            filterable
            clearable
            placeholder="选择预算（可选）"
            size="small"
          >
            <el-option
              v-for="budget in allBudgets"
              :key="budget.id"
              :label="`${budget.name}（${budget.category_name}）`"
              :value="budget.id"
            />
          </el-select>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述">
        <template #default="{ row }"><el-input v-model="row.description" /></template>
      </el-table-column>
    </el-table>
    <div class="actions">
      <el-button type="primary" @click="onConfirm">确认保存</el-button>
      <el-button @click="$emit('cancel', draft.draft_id)">取消</el-button>
    </div>
  </div>
</template>

<style scoped>
.draft-card { border: 1px solid #2a2a44; border-radius: 8px; padding: 12px; background: #12121f; }
.title { color: #a5b4fc; margin-bottom: 8px; font-weight: 600; }
.actions { margin-top: 10px; display: flex; gap: 8px; justify-content: flex-end; }
</style>
