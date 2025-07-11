<template>
  <div class="categories-view">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <h3>分类管理</h3>
      <div class="toolbar-actions">
        <el-button-group>
          <el-button 
            :type="currentType === 'expense' ? 'primary' : 'default'" 
            @click="setCurrentType('expense')"
            size="small"
          >
            支出分类
          </el-button>
          <el-button 
            :type="currentType === 'income' ? 'primary' : 'default'" 
            @click="setCurrentType('income')"
            size="small"
          >
            收入分类
          </el-button>
        </el-button-group>
      </div>
    </div>

    <div class="categories-container">
      <!-- 左栏：大类 -->
      <el-card class="parent-categories-card">
        <template #header>
          <div class="card-header">
            <span>大类</span>
            <el-button 
              @click="showAddParentDialog = true" 
              type="primary" 
              size="small"
              class="add-btn"
            >
              <el-icon><Plus /></el-icon>
              添加大类
            </el-button>
          </div>
        </template>

        <div class="categories-list">
          <div 
            v-for="category in currentParentCategories" 
            :key="category.id"
            class="category-item"
            :class="{ 'active': selectedParentId === category.id }"
            @click="selectParentCategory(category.id)"
          >
            <div class="category-info">
              <span class="category-icon" :style="{ color: category.color }">
                {{ category.icon || '📁' }}
              </span>
              <span class="category-name">{{ category.name }}</span>
              <el-tag v-if="category.is_system" size="small" type="info">系统</el-tag>
            </div>
            <div class="category-actions">
              <el-button 
                @click.stop="editParentCategory(category)" 
                type="primary" 
                size="small" 
                text
              >
                <el-icon><Edit /></el-icon>
              </el-button>
              <el-button 
                @click.stop="deleteParentCategory(category)" 
                type="danger" 
                size="small" 
                text
              >
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>

          <div v-if="currentParentCategories.length === 0" class="empty-state">
            <el-empty description="暂无大类" />
          </div>
        </div>
      </el-card>

      <!-- 右栏：小类 -->
      <el-card class="sub-categories-card">
        <template #header>
          <div class="card-header">
            <span>{{ selectedParentName ? `${selectedParentName} - 小类` : '小类' }}</span>
            <el-button 
              @click="showAddSubDialog = true" 
              type="primary" 
              size="small"
              :disabled="!selectedParentId"
              class="add-btn"
            >
              <el-icon><Plus /></el-icon>
              添加小类
            </el-button>
          </div>
        </template>

        <div class="categories-list">
          <div 
            v-for="category in currentSubCategories" 
            :key="category.id"
            class="category-item"
          >
            <div class="category-info">
              <span class="category-icon" :style="{ color: category.color }">
                {{ category.icon || '📋' }}
              </span>
              <span class="category-name">{{ category.name }}</span>
              <el-tag v-if="category.is_system" size="small" type="info">系统</el-tag>
            </div>
            <div class="category-actions">
              <el-button 
                @click="editSubCategory(category)" 
                type="primary" 
                size="small" 
                text
              >
                <el-icon><Edit /></el-icon>
              </el-button>
              <el-button 
                @click="deleteSubCategory(category)" 
                type="danger" 
                size="small" 
                text
              >
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>

          <div v-if="!selectedParentId" class="empty-state">
            <el-empty description="请先选择一个大类" />
          </div>

          <div v-else-if="currentSubCategories.length === 0" class="empty-state">
            <el-empty description="该大类下暂无小类" />
          </div>
        </div>
      </el-card>
    </div>

    <!-- 添加/编辑大类对话框 -->
    <el-dialog
      v-model="showAddParentDialog"
      :title="editingParent ? '编辑大类' : '添加大类'"
      width="400px"
      class="category-dialog"
    >
      <el-form :model="parentForm" label-width="80px">
        <el-form-item label="分类名称" required>
          <el-input 
            v-model="parentForm.name" 
            placeholder="请输入分类名称"
            maxlength="20"
            show-word-limit
          />
        </el-form-item>
        
        <el-form-item label="图标">
          <div class="icon-selector">
            <el-input 
              v-model="parentForm.icon" 
              placeholder="选择图标"
              readonly
              class="icon-input"
            />
            <div class="icon-grid">
              <div 
                v-for="icon in categoryIcons" 
                :key="icon"
                class="icon-option"
                :class="{ 'selected': parentForm.icon === icon }"
                @click="parentForm.icon = icon"
              >
                {{ icon }}
              </div>
            </div>
          </div>
        </el-form-item>
        
        <el-form-item label="颜色">
          <el-color-picker v-model="parentForm.color" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="cancelParentEdit">取消</el-button>
          <el-button type="primary" @click="saveParentCategory" :loading="saving">
            {{ editingParent ? '更新' : '添加' }}
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- 添加/编辑小类对话框 -->
    <el-dialog
      v-model="showAddSubDialog"
      :title="editingSub ? '编辑小类' : '添加小类'"
      width="400px"
      class="category-dialog"
    >
      <el-form :model="subForm" label-width="80px">
        <el-form-item label="分类名称" required>
          <el-input 
            v-model="subForm.name" 
            placeholder="请输入分类名称"
            maxlength="20"
            show-word-limit
          />
        </el-form-item>
        
        <el-form-item label="图标">
          <div class="icon-selector">
            <el-input 
              v-model="subForm.icon" 
              placeholder="选择图标"
              readonly
              class="icon-input"
            />
            <div class="icon-grid">
              <div 
                v-for="icon in categoryIcons" 
                :key="icon"
                class="icon-option"
                :class="{ 'selected': subForm.icon === icon }"
                @click="subForm.icon = icon"
              >
                {{ icon }}
              </div>
            </div>
          </div>
        </el-form-item>
        
        <el-form-item label="颜色">
          <el-color-picker v-model="subForm.color" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="cancelSubEdit">取消</el-button>
          <el-button type="primary" @click="saveSubCategory" :loading="saving">
            {{ editingSub ? '更新' : '添加' }}
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import { useAppStore, type Category } from '../stores'

const store = useAppStore()

// 响应式数据
const currentType = ref<'income' | 'expense'>('expense')
const selectedParentId = ref<number | null>(null)
const showAddParentDialog = ref(false)
const showAddSubDialog = ref(false)
const editingParent = ref<Category | null>(null)
const editingSub = ref<Category | null>(null)
const saving = ref(false)

// 表单数据
const parentForm = ref({
  name: '',
  icon: '📁',
  color: '#409eff'
})

const subForm = ref({
  name: '',
  icon: '📋',
  color: '#67c23a'
})

// 图标选项
const categoryIcons = [
  '📁', '📂', '📋', '📄', '💰', '💳', '🏠', '🚗', '🍔', '👕', 
  '📱', '💊', '🎬', '🎮', '📚', '✈️', '🏥', '🎓', '💼', '🛒',
  '🍎', '☕', '🎵', '🏃', '💡', '🔧', '🎁', '💍', '🚇', '⛽'
]

// 计算属性
const currentParentCategories = computed(() => {
  return currentType.value === 'income' 
    ? store.parentIncomeCategories 
    : store.parentExpenseCategories
})

const currentSubCategories = computed(() => {
  return selectedParentId.value 
    ? store.getSubCategories(selectedParentId.value)
    : []
})

const selectedParentName = computed(() => {
  if (!selectedParentId.value) return null
  const parent = store.categories.find(cat => cat.id === selectedParentId.value)
  return parent?.name || null
})

// 方法
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
    color: category.color || '#409eff'
  }
  showAddParentDialog.value = true
}

const editSubCategory = (category: Category) => {
  editingSub.value = category
  subForm.value = {
    name: category.name,
    icon: category.icon || '📋',
    color: category.color || '#67c23a'
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
  parentForm.value = {
    name: '',
    icon: '📁',
    color: '#409eff'
  }
}

const resetSubForm = () => {
  subForm.value = {
    name: '',
    icon: '📋',
    color: '#67c23a'
  }
}

const saveParentCategory = async () => {
  if (!parentForm.value.name.trim()) {
    ElMessage.warning('请输入分类名称')
    return
  }

  try {
    saving.value = true
    
    if (editingParent.value) {
      // 更新大类
      await store.updateCategory(editingParent.value.id, {
        name: parentForm.value.name,
        icon: parentForm.value.icon,
        color: parentForm.value.color,
        parent_id: null
      })
      ElMessage.success('更新大类成功')
    } else {
      // 添加大类
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
      // 更新小类
      await store.updateCategory(editingSub.value.id, {
        name: subForm.value.name,
        icon: subForm.value.icon,
        color: subForm.value.color,
        parent_id: selectedParentId.value
      })
      ElMessage.success('更新小类成功')
    } else {
      // 添加小类
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
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    await store.deleteCategory(category.id)
    ElMessage.success('删除成功')
    
    // 如果删除的是当前选中的大类，清空选择
    if (selectedParentId.value === category.id) {
      selectedParentId.value = null
    }
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败，请重试')
    }
  }
}

const deleteSubCategory = async (category: Category) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除小类"${category.name}"吗？`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    await store.deleteCategory(category.id)
    ElMessage.success('删除成功')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败，请重试')
    }
  }
}

onMounted(() => {
  store.fetchCategories()
})
</script>

<style scoped>
.categories-view {
  padding: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.toolbar h3 {
  margin: 0;
  color: #ffffff;
  font-size: 20px;
  font-weight: 600;
}

.categories-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  height: calc(100vh - 160px);
  min-height: 600px;
}

.parent-categories-card,
.sub-categories-card {
  background: #2a2a2a;
  border: 1px solid #404040;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: #ffffff;
  font-weight: 600;
  padding: 16px;
  border-bottom: 1px solid #404040;
  flex-shrink: 0;
}

.add-btn {
  background: #606060;
  border-color: #707070;
  color: #ffffff;
}

.add-btn:hover {
  background: #707070;
  border-color: #808080;
}

.categories-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px 16px;
  min-height: 0;
  max-height: calc(100vh - 260px);
}

.category-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  margin-bottom: 8px;
  background: #1a1a1a;
  border: 1px solid #404040;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.category-item:hover {
  background: #3a3a3a;
  border-color: #606060;
}

.category-item.active {
  background: #2d4a6b;
  border-color: #409eff;
}

.category-info {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
}

.category-icon {
  font-size: 20px;
}

.category-name {
  color: #ffffff;
  font-weight: 500;
}

.category-actions {
  display: flex;
  gap: 8px;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.category-item:hover .category-actions {
  opacity: 1;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 200px;
}

.category-dialog {
  background: #2a2a2a;
}

.icon-selector {
  width: 100%;
}

.icon-input {
  margin-bottom: 12px;
}

.icon-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 8px;
  max-height: 200px;
  overflow-y: auto;
  padding: 8px;
  background: #1a1a1a;
  border: 1px solid #404040;
  border-radius: 6px;
}

.icon-option {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.3s ease;
}

.icon-option:hover {
  background: #404040;
}

.icon-option.selected {
  background: #409eff;
  color: #ffffff;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

/* 滚动条样式 */
.categories-list::-webkit-scrollbar,
.icon-grid::-webkit-scrollbar {
  width: 8px;
}

.categories-list::-webkit-scrollbar-track,
.icon-grid::-webkit-scrollbar-track {
  background: #1a1a1a;
  border-radius: 4px;
  margin: 2px;
}

.categories-list::-webkit-scrollbar-thumb,
.icon-grid::-webkit-scrollbar-thumb {
  background: #606060;
  border-radius: 4px;
  border: 1px solid #404040;
}

.categories-list::-webkit-scrollbar-thumb:hover,
.icon-grid::-webkit-scrollbar-thumb:hover {
  background: #808080;
}

.categories-list::-webkit-scrollbar-corner {
  background: #1a1a1a;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .categories-container {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .toolbar {
    flex-direction: column;
    gap: 12px;
  }
  
  .category-actions {
    opacity: 1;
  }
}
</style> 