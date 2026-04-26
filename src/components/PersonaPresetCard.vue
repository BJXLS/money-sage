<script setup lang="ts">
defineProps<{
  title: string
  summary: string
  active?: boolean
}>()
const emit = defineEmits<{ (e: 'use-preset'): void }>()
</script>

<template>
  <el-card class="preset-card" :class="{ active }" shadow="hover" @click="emit('use-preset')">
    <div class="title">{{ title }}</div>
    <div class="summary">{{ summary }}</div>
    <el-button size="small" @click.stop="emit('use-preset')">应用</el-button>
  </el-card>
</template>

<style scoped>
.preset-card { border-color: #2a2a44; cursor: pointer; user-select: none; }
.preset-card.active { border-color: #6366f1; }
.title { font-weight: 600; margin-bottom: 6px; }
.summary { color: #9090b0; margin-bottom: 10px; font-size: 12px; }
</style>
