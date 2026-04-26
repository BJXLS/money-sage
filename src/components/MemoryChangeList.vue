<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores'

const store = useAppStore()

const refresh = async () => {
  await store.fetchMemoryChanges(100)
}

const undo = async (id: number) => {
  await store.undoMemoryChange(id)
  await refresh()
}

onMounted(refresh)
</script>

<template>
  <el-table :data="store.memoryChanges" size="small">
    <el-table-column prop="created_at" label="时间" width="180" />
    <el-table-column prop="op" label="操作" width="130" />
    <el-table-column prop="actor" label="来源" width="100" />
    <el-table-column prop="fact_id" label="fact_id" width="90" />
    <el-table-column label="操作">
      <template #default="{ row }">
        <el-button size="small" @click="undo(row.id)">撤销</el-button>
      </template>
    </el-table-column>
  </el-table>
</template>
