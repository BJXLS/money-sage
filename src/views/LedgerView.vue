<script setup lang="ts">
import { ref } from 'vue'
import TransactionsView from './TransactionsView.vue'
import CategoriesView from './CategoriesView.vue'
import BudgetView from './BudgetView.vue'

const activeTab = ref<'transactions' | 'categories' | 'budget'>('transactions')
</script>

<template>
  <div class="ledger-view-root">
    <div class="tab-header">
      <button
        :class="['tab-btn', { active: activeTab === 'transactions' }]"
        @click="activeTab = 'transactions'"
      >
        收支记录
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'categories' }]"
        @click="activeTab = 'categories'"
      >
        分类管理
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'budget' }]"
        @click="activeTab = 'budget'"
      >
        预算设置
      </button>
    </div>
    <div class="tab-content">
      <TransactionsView v-if="activeTab === 'transactions'" />
      <CategoriesView v-else-if="activeTab === 'categories'" />
      <BudgetView v-else />
    </div>
  </div>
</template>

<style scoped>
.ledger-view-root {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tab-header {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--ms-border-subtle);
}

.tab-btn {
  padding: 8px 16px;
  border-radius: var(--ms-radius-md);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.tab-btn:hover {
  color: var(--ms-text-primary);
  background-color: var(--ms-surface-hover);
}

.tab-btn.active {
  color: white;
  background: var(--ms-gradient-primary);
}

.tab-content {
  flex: 1;
  min-height: 0;
  overflow: visible;
}
</style>
