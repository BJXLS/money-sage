<script setup lang="ts">
import { ref } from 'vue'
import MemoryView from './MemoryView.vue'
import UsageStatsView from './UsageStatsView.vue'
import LLMConfigPanel from '../components/LLMConfigPanel.vue'
import McpConfigPanel from '../components/McpConfigPanel.vue'

const activeTab = ref<'memory' | 'usage' | 'llm' | 'mcp'>('memory')
</script>

<template>
  <div class="system-view-root">
    <div class="tab-header">
      <button
        :class="['tab-btn', { active: activeTab === 'memory' }]"
        @click="activeTab = 'memory'"
      >
        记忆管理
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'usage' }]"
        @click="activeTab = 'usage'"
      >
        用量统计
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'llm' }]"
        @click="activeTab = 'llm'"
      >
        模型配置
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'mcp' }]"
        @click="activeTab = 'mcp'"
      >
        MCP
      </button>
    </div>
    <div class="tab-content">
      <MemoryView v-if="activeTab === 'memory'" />
      <UsageStatsView v-else-if="activeTab === 'usage'" />
      <LLMConfigPanel v-else-if="activeTab === 'llm'" class="config-panel" />
      <McpConfigPanel v-else class="config-panel" />
    </div>
  </div>
</template>

<style scoped>
.system-view-root {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}

.tab-header {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--ms-border-subtle);
  flex-shrink: 0;
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
  overflow: hidden;
}

.config-panel {
  height: 100%;
}
</style>
