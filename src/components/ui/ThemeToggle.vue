<script setup lang="ts">
import { computed } from 'vue'
import { useTheme } from '../../composables/useTheme'
import { Sunny, Moon } from '@element-plus/icons-vue'

const { mode, setMode } = useTheme()

const isDark = computed(() => {
  if (mode.value === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  }
  return mode.value === 'dark'
})

const options = [
  { value: 'light', label: '浅色', icon: Sunny },
  { value: 'dark', label: '深色', icon: Moon },
  { value: 'system', label: '跟随系统', icon: null },
] as const
</script>

<template>
  <el-dropdown trigger="click" @command="(cmd: string) => setMode(cmd as any)">
    <button class="ms-theme-toggle" :title="isDark ? '深色模式' : '浅色模式'">
      <el-icon v-if="isDark" :size="18"><Moon /></el-icon>
      <el-icon v-else :size="18"><Sunny /></el-icon>
    </button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="opt in options"
          :key="opt.value"
          :command="opt.value"
          :class="{ 'is-active': mode === opt.value }"
        >
          <el-icon v-if="opt.icon" :size="14" class="mr-2">
            <component :is="opt.icon" />
          </el-icon>
          <span v-else class="w-[14px] mr-2"></span>
          {{ opt.label }}
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<style scoped>
.ms-theme-toggle {
  width: 40px;
  height: 40px;
  border-radius: var(--ms-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border: 1px solid var(--ms-border-subtle);
  background-color: var(--ms-surface-secondary);
  color: var(--ms-text-secondary);
  transition: all 0.2s ease;
}

.ms-theme-toggle:hover {
  background-color: var(--ms-surface-hover);
  color: var(--ms-text-primary);
}
</style>
