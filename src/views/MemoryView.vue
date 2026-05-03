<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore, type FactType, type FactStatus } from '../stores'
import MemoryFactEditor from '../components/MemoryFactEditor.vue'
import MemoryFactCard from '../components/MemoryFactCard.vue'
import MemoryChangeList from '../components/MemoryChangeList.vue'
import PersonaTab from '../components/PersonaTab.vue'

const store = useAppStore()
const active = ref<'persona' | 'profile' | 'rules' | 'pending' | 'changes'>('persona')
const ruleType = ref<FactType>('classification_rule')
const statusFilter = ref<FactStatus | undefined>(undefined)

const refreshFacts = async () => {
  if (active.value === 'profile') {
    await store.fetchMemoryFacts('user_profile', statusFilter.value)
  } else if (active.value === 'rules') {
    await store.fetchMemoryFacts(ruleType.value, statusFilter.value)
  } else if (active.value === 'pending') {
    await store.fetchPendingFacts()
  } else {
    store.memoryFacts.length = 0
  }
}

watch(active, refreshFacts)
watch(ruleType, () => {
  if (active.value === 'rules') void refreshFacts()
})
watch(statusFilter, () => {
  if (active.value === 'profile' || active.value === 'rules') void refreshFacts()
})

onMounted(refreshFacts)

const onFactSaved = () => {
  refreshFacts()
}
</script>

<template>
  <div>
    <el-tabs v-model="active">
      <el-tab-pane label="人格" name="persona">
        <PersonaTab />
      </el-tab-pane>

      <el-tab-pane label="画像与目标" name="profile">
        <el-space style="margin-bottom: 8px;">
          <el-select v-model="statusFilter" clearable placeholder="全部状态" style="width: 140px">
            <el-option label="全部" :value="undefined" />
            <el-option label="生效中" value="active" />
            <el-option label="待确认" value="provisional" />
            <el-option label="已退役" value="retired" />
          </el-select>
        </el-space>
        <MemoryFactEditor type="user_profile" @saved="onFactSaved" />
        <div style="margin-top: 10px">
          <MemoryFactCard
            v-for="fact in store.memoryFacts"
            :key="fact.id"
            :fact="fact"
            @refresh="refreshFacts"
          />
          <el-empty v-if="!store.memoryFacts.length" description="暂无数据" />
        </div>
      </el-tab-pane>

      <el-tab-pane label="规则与事件" name="rules">
        <el-space style="margin-bottom: 8px;">
          <el-select v-model="ruleType" style="width: 220px">
            <el-option label="分类规则" value="classification_rule" />
            <el-option label="固定事件" value="recurring_event" />
            <el-option label="财务目标" value="financial_goal" />
          </el-select>
          <el-select v-model="statusFilter" clearable placeholder="全部状态" style="width: 140px">
            <el-option label="全部" :value="undefined" />
            <el-option label="生效中" value="active" />
            <el-option label="待确认" value="provisional" />
            <el-option label="已退役" value="retired" />
          </el-select>
        </el-space>
        <MemoryFactEditor :type="ruleType" @saved="onFactSaved" />
        <div style="margin-top: 10px">
          <MemoryFactCard
            v-for="fact in store.memoryFacts"
            :key="fact.id"
            :fact="fact"
            @refresh="refreshFacts"
          />
          <el-empty v-if="!store.memoryFacts.length" description="暂无数据" />
        </div>
      </el-tab-pane>

      <el-tab-pane label="待确认" name="pending">
        <div>
          <MemoryFactCard
            v-for="fact in store.pendingFacts"
            :key="fact.id"
            :fact="fact"
            show-confirm
            @refresh="refreshFacts"
          />
          <el-empty v-if="!store.pendingFacts.length" description="没有待确认的记忆" />
        </div>
      </el-tab-pane>

      <el-tab-pane label="最近变动" name="changes">
        <MemoryChangeList />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>
