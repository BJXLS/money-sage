<template>
  <div class="categories-budget-root">
    <el-tabs v-model="activeTab" class="categories-budget-tabs">
      <el-tab-pane label="分类管理" name="categories">
        <div class="tab-pane-inner">
          <CategoriesView />
        </div>
      </el-tab-pane>
      <el-tab-pane label="预算设置" name="budget" lazy>
        <div class="tab-pane-inner">
          <BudgetView />
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import CategoriesView from './CategoriesView.vue'
import BudgetView from './BudgetView.vue'

const activeTab = ref<'categories' | 'budget'>('categories')
</script>

<style scoped>
.categories-budget-root {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.categories-budget-tabs :deep(.el-tabs__header) {
  margin: 0 0 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.categories-budget-tabs :deep(.el-tabs__nav-wrap::after) {
  display: none;
}

.categories-budget-tabs :deep(.el-tabs__item) {
  color: #94a3b8;
  font-weight: 500;
}

.categories-budget-tabs :deep(.el-tabs__item.is-active) {
  color: #a5b4fc;
}

.categories-budget-tabs :deep(.el-tabs__active-bar) {
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
}

.categories-budget-tabs :deep(.el-tabs__content) {
  overflow: visible;
}

.tab-pane-inner {
  margin: 0 -4px;
  min-height: 0;
}
</style>
