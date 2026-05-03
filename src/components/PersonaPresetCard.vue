<script setup lang="ts">
defineProps<{
  title: string
  summary: string
  active?: boolean
  isBuiltin?: boolean
}>()
const emit = defineEmits<{
  (e: 'apply'): void
  (e: 'edit'): void
  (e: 'delete'): void
  (e: 'reset'): void
}>()
</script>

<template>
  <el-card class="preset-card" :class="{ active }" shadow="hover">
    <div class="header">
      <div class="title">{{ title }}</div>
      <el-tag v-if="isBuiltin" size="small" type="info" effect="plain">内置</el-tag>
    </div>
    <div class="summary">{{ summary || '（无简介）' }}</div>
    <div class="actions">
      <el-button size="small" type="primary" @click="emit('apply')">应用</el-button>
      <el-button v-if="!isBuiltin" size="small" @click="emit('edit')">编辑</el-button>
      <el-button v-if="!isBuiltin" size="small" type="danger" plain @click="emit('delete')">删除</el-button>
      <el-button v-if="isBuiltin" size="small" @click="emit('reset')">重置默认</el-button>
      <el-button v-if="isBuiltin" size="small" @click="emit('edit')">编辑</el-button>
    </div>
  </el-card>
</template>

<style scoped>
.preset-card { border-color: #2a2a44; user-select: none; }
.preset-card.active { border-color: #6366f1; box-shadow: 0 0 0 1px #6366f1 inset; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.title { font-weight: 600; }
.summary { color: #9090b0; margin-bottom: 10px; font-size: 12px; min-height: 32px; }
.actions { display: flex; gap: 6px; flex-wrap: wrap; }
</style>
