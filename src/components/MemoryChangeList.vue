<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores'
import { formatHistoryOp, extractChangeSummary } from '../utils/memoryHistoryFormatter'

const store = useAppStore()

const refresh = async () => {
  await store.fetchMemoryChanges(100)
}

const undo = async (id: number) => {
  await store.undoMemoryChange(id)
  await refresh()
}

const formatOp = (row: any) => {
  const factType = row.before_json?.fact_type || row.after_json?.fact_type
  return formatHistoryOp(row.op, factType)
}

const formatSummary = (row: any) => {
  return extractChangeSummary(row)
}

onMounted(refresh)
</script>

<template>
  <el-table :data="store.memoryChanges" size="small">
    <el-table-column prop="created_at" label="时间" width="170" />
    <el-table-column label="操作" width="140">
      <template #default="{ row }">
        {{ formatOp(row) }}
      </template>
    </el-table-column>
    <el-table-column prop="actor" label="来源" width="90" />
    <el-table-column prop="fact_id" label="ID" width="70" />
    <el-table-column label="变更摘要">
      <template #default="{ row }">
        <el-text size="small" type="info">{{ formatSummary(row) }}</el-text>
      </template>
    </el-table-column>
    <el-table-column label="操作" width="80">
      <template #default="{ row }">
        <el-button size="small" @click="undo(row.id)">撤销</el-button>
      </template>
    </el-table-column>
  </el-table>
</template>
