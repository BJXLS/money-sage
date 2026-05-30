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

const form = reactive({
  items: props.draft.items.map(i => ({
    ...i,
    category_id: i.category_id && i.category_id > 0 ? i.category_id : null,
    budget_id: i.budget_id && i.budget_id > 0 ? i.budget_id : null,
  }))
})
const allCategories = ref<Category[]>([])

const loadMeta = async () => {
  const categories = await invoke<Category[]>('get_categories')
  allCategories.value = categories || []

  // 加载分类后，尝试根据 raw_category_name 自动匹配分类
  // AI 可能返回 "大类-小类" 格式，需要拆分后优先匹配小类
  for (const item of form.items) {
    if (!item.category_id && item.raw_category_name) {
      const raw = item.raw_category_name.trim()
      if (!raw) continue

      const txType = item.transaction_type
      const rawLower = raw.toLowerCase()

      // 辅助函数：优先按类型过滤分类
      const filterByType = (cats: Category[]) => {
        const sameType = cats.filter(c => c.type === txType)
        return sameType.length > 0 ? sameType : cats
      }

      // 1. 精确匹配（优先同类型）
      let matched = allCategories.value.find(c => c.name === raw && c.type === txType)
      if (!matched) {
        matched = allCategories.value.find(c => c.name === raw)
      }

      // 2. 拆分 "大类-小类"，优先匹配小类
      if (!matched && raw.includes('-')) {
        const parts = raw.split('-').map(s => s.trim()).filter(s => s)
        if (parts.length > 1) {
          const subName = parts[parts.length - 1]
          const candidates = filterByType(allCategories.value)
          matched = candidates.find(c => c.name === subName)

          // 3. 尝试匹配大类
          if (!matched) {
            const parentName = parts[0]
            const candidates = filterByType(allCategories.value)
            matched = candidates.find(c => c.name === parentName)
          }
        }
      }

      // 4. 模糊匹配（优先同类型）
      if (!matched) {
        const candidates = filterByType(allCategories.value)
        matched = candidates.find(c => {
          const cLower = c.name.toLowerCase()
          return rawLower.includes(cLower) || cLower.includes(rawLower)
        })
      }
      // 5. 跨类型兜底模糊匹配
      if (!matched) {
        matched = allCategories.value.find(c => {
          const cLower = c.name.toLowerCase()
          return rawLower.includes(cLower) || cLower.includes(rawLower)
        })
      }

      if (matched) {
        item.category_id = matched.id
      }
    }
  }
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
      <el-table-column prop="category_id" label="分类" width="220">
        <template #default="{ row }">
          <div class="category-cell">
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
            <span v-if="row.raw_category_name" class="ai-category-hint">
              AI识别：{{ row.raw_category_name }}
            </span>
          </div>
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
.category-cell { display: flex; flex-direction: column; gap: 4px; }
.ai-category-hint { font-size: 11px; color: #f59e0b; }
</style>
