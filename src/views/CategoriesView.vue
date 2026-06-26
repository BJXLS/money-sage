<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import { useAppStore, type Category } from '../stores'
import dayjs from 'dayjs'

const store = useAppStore()

const currentType = ref<'income' | 'expense'>('expense')
const selectedParentId = ref<number | null>(null)
const showAddParentDialog = ref(false)
const showAddSubDialog = ref(false)
const editingParent = ref<Category | null>(null)
const editingSub = ref<Category | null>(null)
const saving = ref(false)

const parentForm = ref({ name: '', icon: '📁', color: '#6366f1' })
const subForm = ref({ name: '', icon: '📋', color: '#8b5cf6' })

const categoryIcons = [
  '📁', '📂', '📋', '📄', '💰', '💳', '🏠', '🚗', '🍔', '👕',
  '📱', '💊', '🎬', '🎮', '📚', '✈️', '🏥', '🎓', '💼', '🛒',
  '🍎', '☕', '🎵', '🏃', '💡', '🔧', '🎁', '💍', '🚇', '⛽'
]

const currentParentCategories = computed(() => {
  return currentType.value === 'income' ? store.parentIncomeCategories : store.parentExpenseCategories
})

const currentSubCategories = computed(() => {
  return selectedParentId.value ? store.getSubCategories(selectedParentId.value) : []
})

const selectedParentName = computed(() => {
  if (!selectedParentId.value) return null
  const parent = store.categories.find(cat => cat.id === selectedParentId.value)
  return parent?.name || null
})

const subCategoryCount = (parentId: number) => {
  return store.getSubCategories(parentId).length
}

const getCurrentMonthAmount = (categoryId: number) => {
  const now = dayjs()
  return store.transactions
    .filter(t => t.category_id === categoryId && dayjs(t.date).isSame(now, 'month'))
    .reduce((sum, t) => sum + t.amount, 0)
}

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 0, maximumFractionDigits: 2 })
}

const setCurrentType = (type: 'income' | 'expense') => {
  currentType.value = type
  selectedParentId.value = null
}

const selectParentCategory = (id: number) => {
  selectedParentId.value = id
}

const editParentCategory = (category: Category) => {
  editingParent.value = category
  parentForm.value = {
    name: category.name,
    icon: category.icon || '📁',
    color: category.color || '#6366f1'
  }
  showAddParentDialog.value = true
}

const editSubCategory = (category: Category) => {
  editingSub.value = category
  subForm.value = {
    name: category.name,
    icon: category.icon || '📋',
    color: category.color || '#8b5cf6'
  }
  showAddSubDialog.value = true
}

const cancelParentEdit = () => {
  editingParent.value = null
  showAddParentDialog.value = false
  resetParentForm()
}

const cancelSubEdit = () => {
  editingSub.value = null
  showAddSubDialog.value = false
  resetSubForm()
}

const resetParentForm = () => {
  parentForm.value = { name: '', icon: '📁', color: '#6366f1' }
}

const resetSubForm = () => {
  subForm.value = { name: '', icon: '📋', color: '#8b5cf6' }
}

const saveParentCategory = async () => {
  if (!parentForm.value.name.trim()) {
    ElMessage.warning('请输入分类名称')
    return
  }

  try {
    saving.value = true
    if (editingParent.value) {
      await store.updateCategory(editingParent.value.id, {
        name: parentForm.value.name,
        icon: parentForm.value.icon,
        color: parentForm.value.color,
        parent_id: null
      })
      ElMessage.success('更新大类成功')
    } else {
      await store.createCategory({
        name: parentForm.value.name,
        icon: parentForm.value.icon,
        color: parentForm.value.color,
        type: currentType.value,
        parent_id: null
      })
      ElMessage.success('添加大类成功')
    }
    cancelParentEdit()
  } catch (error) {
    ElMessage.error('操作失败，请重试')
  } finally {
    saving.value = false
  }
}

const saveSubCategory = async () => {
  if (!subForm.value.name.trim()) {
    ElMessage.warning('请输入分类名称')
    return
  }
  if (!selectedParentId.value) {
    ElMessage.warning('请先选择一个大类')
    return
  }

  try {
    saving.value = true
    if (editingSub.value) {
      await store.updateCategory(editingSub.value.id, {
        name: subForm.value.name,
        icon: subForm.value.icon,
        color: subForm.value.color,
        parent_id: selectedParentId.value
      })
      ElMessage.success('更新小类成功')
    } else {
      await store.createCategory({
        name: subForm.value.name,
        icon: subForm.value.icon,
        color: subForm.value.color,
        type: currentType.value,
        parent_id: selectedParentId.value
      })
      ElMessage.success('添加小类成功')
    }
    cancelSubEdit()
  } catch (error) {
    ElMessage.error('操作失败，请重试')
  } finally {
    saving.value = false
  }
}

const deleteParentCategory = async (category: Category) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除大类"${category.name}"吗？这将同时删除该大类下的所有小类。`,
      '确认删除',
      { confirmButtonText: '确定', cancelButtonText: '取消', type: 'warning' }
    )
    await store.deleteCategory(category.id)
    ElMessage.success('删除成功')
    if (selectedParentId.value === category.id) selectedParentId.value = null
  } catch (error) {
    if (error !== 'cancel') ElMessage.error('删除失败，请重试')
  }
}

const deleteSubCategory = async (category: Category) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除小类"${category.name}"吗？`,
      '确认删除',
      { confirmButtonText: '确定', cancelButtonText: '取消', type: 'warning' }
    )
    await store.deleteCategory(category.id)
    ElMessage.success('删除成功')
  } catch (error) {
    if (error !== 'cancel') ElMessage.error('删除失败，请重试')
  }
}

onMounted(async () => {
  await store.fetchCategories()
  const start = dayjs().startOf('month').format('YYYY-MM-DD')
  const end = dayjs().endOf('month').format('YYYY-MM-DD')
  await store.fetchTransactionsByDateRange(start, end)
})
</script>

<template>
  <div class="categories-view">
    <!-- Toolbar -->
    <div class="toolbar">
      <div class="type-segment">
        <button :class="['segment-btn', { active: currentType === 'expense' }]" @click="setCurrentType('expense')">支出</button>
        <button :class="['segment-btn', { active: currentType === 'income' }]" @click="setCurrentType('income')">收入</button>
      </div>
      <button class="add-btn" @click="showAddParentDialog = true">
        <el-icon><Plus /></el-icon>
        <span>添加分类</span>
      </button>
    </div>

    <!-- Grid -->
    <div class="categories-grid">
      <!-- Parent Categories -->
      <div class="category-card">
        <h3 class="card-title">大类</h3>
        <div class="parent-list">
          <div
            v-for="category in currentParentCategories"
            :key="category.id"
            class="parent-item"
            :class="{ active: selectedParentId === category.id }"
            @click="selectParentCategory(category.id)"
          >
            <div class="parent-info">
              <div class="category-icon" :style="{ backgroundColor: `${category.color || '#6366f1'}1a` }">
                {{ category.icon || '📁' }}
              </div>
              <span class="category-name">{{ category.name }}</span>
            </div>
            <div class="parent-meta">
              <span class="sub-count">{{ subCategoryCount(category.id) }} 个小类</span>
              <div class="category-actions" @click.stop>
                <button class="action-btn" @click="editParentCategory(category)"><el-icon><Edit /></el-icon></button>
                <button class="action-btn danger" @click="deleteParentCategory(category)"><el-icon><Delete /></el-icon></button>
              </div>
            </div>
          </div>
          <div v-if="currentParentCategories.length === 0" class="empty-state">暂无大类</div>
        </div>
      </div>

      <!-- Sub Categories -->
      <div class="category-card sub-card">
        <div class="card-header">
          <h3 class="card-title">{{ selectedParentName ? `${selectedParentName} · 小类` : '小类' }}</h3>
          <button v-if="selectedParentId" class="add-sub-btn" @click="showAddSubDialog = true">添加小类</button>
        </div>
        <div v-if="!selectedParentId" class="empty-state">请先选择一个大类</div>
        <div v-else-if="currentSubCategories.length === 0" class="empty-state">该大类下暂无小类</div>
        <div v-else class="sub-grid">
          <div v-for="category in currentSubCategories" :key="category.id" class="sub-item">
            <div class="sub-info">
              <div class="category-icon" :style="{ backgroundColor: `${category.color || '#8b5cf6'}1a` }">
                {{ category.icon || '📋' }}
              </div>
              <div>
                <div class="category-name">{{ category.name }}</div>
                <div class="month-amount">本月 ¥ {{ formatAmount(getCurrentMonthAmount(category.id)) }}</div>
              </div>
            </div>
            <div class="category-actions">
              <button class="action-btn" @click="editSubCategory(category)"><el-icon><Edit /></el-icon></button>
              <button class="action-btn danger" @click="deleteSubCategory(category)"><el-icon><Delete /></el-icon></button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Parent Dialog -->
    <el-dialog v-model="showAddParentDialog" :title="editingParent ? '编辑大类' : '添加大类'" width="400px">
      <el-form :model="parentForm" label-width="80px">
        <el-form-item label="分类名称" required>
          <el-input v-model="parentForm.name" placeholder="请输入分类名称" maxlength="20" show-word-limit />
        </el-form-item>
        <el-form-item label="图标">
          <div class="icon-selector">
            <el-input v-model="parentForm.icon" placeholder="选择图标" readonly />
            <div class="icon-grid">
              <div v-for="icon in categoryIcons" :key="icon" class="icon-option" :class="{ selected: parentForm.icon === icon }" @click="parentForm.icon = icon">{{ icon }}</div>
            </div>
          </div>
        </el-form-item>
        <el-form-item label="颜色">
          <el-color-picker v-model="parentForm.color" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="cancelParentEdit">取消</el-button>
        <el-button type="primary" @click="saveParentCategory" :loading="saving">{{ editingParent ? '更新' : '添加' }}</el-button>
      </template>
    </el-dialog>

    <!-- Sub Dialog -->
    <el-dialog v-model="showAddSubDialog" :title="editingSub ? '编辑小类' : '添加小类'" width="400px">
      <el-form :model="subForm" label-width="80px">
        <el-form-item label="分类名称" required>
          <el-input v-model="subForm.name" placeholder="请输入分类名称" maxlength="20" show-word-limit />
        </el-form-item>
        <el-form-item label="图标">
          <div class="icon-selector">
            <el-input v-model="subForm.icon" placeholder="选择图标" readonly />
            <div class="icon-grid">
              <div v-for="icon in categoryIcons" :key="icon" class="icon-option" :class="{ selected: subForm.icon === icon }" @click="subForm.icon = icon">{{ icon }}</div>
            </div>
          </div>
        </el-form-item>
        <el-form-item label="颜色">
          <el-color-picker v-model="subForm.color" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="cancelSubEdit">取消</el-button>
        <el-button type="primary" @click="saveSubCategory" :loading="saving">{{ editingSub ? '更新' : '添加' }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.categories-view {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-5);
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.type-segment {
  display: flex;
  padding: 4px;
  border-radius: var(--ms-radius-md);
  background-color: var(--ms-bg-tertiary);
  border: 1px solid var(--ms-border-subtle);
}

.segment-btn {
  padding: 6px 16px;
  border-radius: var(--ms-radius-md);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.segment-btn.active {
  color: white;
  background: var(--ms-gradient-primary);
  box-shadow: var(--ms-shadow-sm);
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: var(--ms-radius-lg);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: white;
  background: var(--ms-gradient-primary);
  border: none;
  cursor: pointer;
  transition: opacity 0.2s ease;
}

.add-btn:hover {
  opacity: 0.9;
}

.categories-grid {
  display: grid;
  grid-template-columns: 320px 1fr;
  gap: var(--ms-space-5);
  height: calc(100vh - 220px);
  min-height: 500px;
  overflow: hidden;
}

.category-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) var(--ms-space-5);
  display: flex;
  flex-direction: column;
  min-height: 0;
  box-shadow: var(--ms-shadow-sm);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--ms-space-4);
}

.card-title {
  font-size: var(--ms-text-base);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
  margin: 0;
}

.add-sub-btn {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-primary-500);
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: var(--ms-radius-md);
  transition: background 0.2s ease;
}

.add-sub-btn:hover {
  background: var(--ms-bg-tertiary);
}

.parent-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-2);
  min-height: 0;
}

.parent-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-3);
  border-radius: var(--ms-radius-lg);
  background-color: var(--ms-surface-secondary);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.2s ease;
}

.parent-item:hover {
  background-color: var(--ms-surface-hover);
}

.parent-item.active {
  background-color: var(--ms-bg-tertiary);
  border-color: var(--ms-border-default);
}

.parent-info {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
  flex: 1;
  min-width: 0;
}

.parent-meta {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
  flex-shrink: 0;
}

.sub-count {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.category-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--ms-radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.category-name {
  color: var(--ms-text-primary);
  font-weight: var(--ms-font-medium);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.month-amount {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-top: 2px;
}

.category-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.parent-item:hover .category-actions,
.sub-item:hover .category-actions {
  opacity: 1;
}

.action-btn {
  width: 28px;
  height: 28px;
  border-radius: var(--ms-radius-md);
  border: none;
  background-color: transparent;
  color: var(--ms-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.action-btn:hover {
  background-color: var(--ms-bg-tertiary);
  color: var(--ms-text-primary);
}

.action-btn.danger:hover {
  background-color: rgba(244, 63, 94, 0.1);
  color: var(--ms-expense);
}

.sub-card {
  min-height: 0;
}

.sub-grid {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-2);
  min-height: 0;
}

.sub-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-3) var(--ms-space-4);
  border-radius: var(--ms-radius-lg);
  background-color: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  transition: all 0.2s ease;
}

.sub-item:hover {
  background-color: var(--ms-surface-hover);
}

.sub-info {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
  min-width: 0;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 200px;
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-sm);
}

.icon-selector {
  width: 100%;
}

.icon-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 8px;
  max-height: 200px;
  overflow-y: auto;
  padding: 10px;
  margin-top: 10px;
  background-color: var(--ms-bg-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
}

.icon-option {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  cursor: pointer;
  border-radius: var(--ms-radius-md);
  transition: all 0.2s ease;
}

.icon-option:hover {
  background-color: var(--ms-surface-hover);
}

.icon-option.selected {
  background: var(--ms-gradient-primary);
  color: white;
}

@media (max-width: 1024px) {
  .categories-grid {
    grid-template-columns: 1fr;
    height: auto;
    min-height: auto;
    overflow: visible;
  }

  .category-card {
    max-height: 480px;
  }
}

@media (max-width: 768px) {
  .category-actions {
    opacity: 1;
  }
}
</style>
