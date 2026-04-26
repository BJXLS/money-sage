<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore, type FactType } from '../stores'
import MemoryFactEditor from '../components/MemoryFactEditor.vue'
import MemoryChangeList from '../components/MemoryChangeList.vue'
import PersonaTab from '../components/PersonaTab.vue'

const store = useAppStore()
const active = ref('persona')
const factType = ref<FactType>('classification_rule')

const refreshFacts = async () => {
  await store.fetchMemoryFacts(factType.value, 'active')
}

onMounted(refreshFacts)
</script>

<template>
  <div>
    <el-tabs v-model="active">
      <el-tab-pane label="人格" name="persona">
        <PersonaTab />
      </el-tab-pane>
      <el-tab-pane label="画像与目标" name="profile">
        <MemoryFactEditor type="user_profile" @saved="refreshFacts" />
        <el-table :data="store.memoryFacts" style="margin-top: 10px">
          <el-table-column prop="key" label="Key" width="200" />
          <el-table-column label="内容">
            <template #default="{ row }">{{ JSON.stringify(row.value_json) }}</template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="规则与事件" name="rules">
        <el-space>
          <el-select v-model="factType" @change="refreshFacts" style="width: 220px">
            <el-option label="分类规则" value="classification_rule" />
            <el-option label="固定事件" value="recurring_event" />
            <el-option label="财务目标" value="financial_goal" />
          </el-select>
        </el-space>
        <MemoryFactEditor :type="factType" @saved="refreshFacts" />
        <el-table :data="store.memoryFacts" style="margin-top: 10px">
          <el-table-column prop="key" label="Key" width="220" />
          <el-table-column label="内容">
            <template #default="{ row }">{{ JSON.stringify(row.value_json) }}</template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="最近变动" name="changes">
        <MemoryChangeList />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>
