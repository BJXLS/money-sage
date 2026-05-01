<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore, type FactType } from '../stores'
import MemoryFactEditor from '../components/MemoryFactEditor.vue'
import MemoryChangeList from '../components/MemoryChangeList.vue'
import PersonaTab from '../components/PersonaTab.vue'

const store = useAppStore()
const active = ref<'persona' | 'profile' | 'rules' | 'changes'>('persona')
const ruleType = ref<FactType>('classification_rule')

const refreshFacts = async () => {
  if (active.value === 'profile') {
    await store.fetchMemoryFacts('user_profile', 'active')
  } else if (active.value === 'rules') {
    await store.fetchMemoryFacts(ruleType.value, 'active')
  } else {
    store.memoryFacts.length = 0
  }
}

watch(active, refreshFacts)
watch(ruleType, () => {
  if (active.value === 'rules') void refreshFacts()
})

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
          <el-select v-model="ruleType" style="width: 220px">
            <el-option label="分类规则" value="classification_rule" />
            <el-option label="固定事件" value="recurring_event" />
            <el-option label="财务目标" value="financial_goal" />
          </el-select>
        </el-space>
        <MemoryFactEditor :type="ruleType" @saved="refreshFacts" />
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
