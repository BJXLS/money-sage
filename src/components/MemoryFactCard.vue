<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore, type MemoryFact } from '../stores'

const props = defineProps<{
  fact: MemoryFact
  showConfirm?: boolean
}>()
const emit = defineEmits<{
  (e: 'refresh'): void
}>()

const store = useAppStore()

const retire = async () => {
  await store.retireMemoryFact(props.fact.id)
  emit('refresh')
}

const confirm = async () => {
  await store.confirmMemoryFact(props.fact.id)
  emit('refresh')
}

const title = computed(() => {
  const v = props.fact.value_json
  switch (props.fact.fact_type) {
    case 'classification_rule':
      return `分类规则：${v?.pattern || '-'}`
    case 'recurring_event':
      return `固定事件：${v?.title || '-'}`
    case 'financial_goal':
      return `财务目标：${v?.title || '-'}`
    case 'user_profile':
      return `用户画像：${v?.text?.slice(0, 20) || '-'}`
    case 'agent_role':
      return `人格设定：${v?.display_name || '-'} (${v?.scope || 'global'})`
    default:
      return props.fact.fact_type
  }
})

const subtitle = computed(() => {
  const v = props.fact.value_json
  switch (props.fact.fact_type) {
    case 'classification_rule':
      return `${v?.match_type || 'contains'} → ${v?.target_category_path || '-'}`
    case 'recurring_event':
      return `${v?.cron || '-'} / ¥${v?.amount || 0}`
    case 'financial_goal':
      return `${v?.metric || '-'} ${v?.direction || 'le'} ${v?.target ?? '-'}`
    case 'user_profile':
      return (v?.tags || []).join(', ')
    case 'agent_role':
      return `称呼：${v?.user_address || '-'} / 风格：${v?.tone?.style || '-'}`
    default:
      return ''
  }
})

const statusType = computed(() => {
  switch (props.fact.status) {
    case 'active': return 'success'
    case 'provisional': return 'warning'
    case 'superseded': return 'info'
    case 'retired': return 'danger'
    default: return 'info'
  }
})

const statusLabel = computed(() => {
  switch (props.fact.status) {
    case 'active': return '生效中'
    case 'provisional': return '待确认'
    case 'superseded': return '已取代'
    case 'retired': return '已退役'
    default: return props.fact.status
  }
})

const sourceLabel = computed(() => {
  switch (props.fact.source) {
    case 'user': return '用户'
    case 'preset': return '预设'
    case 'analysis': return 'AI分析'
    case 'import': return '导入'
    case 'recap': return '回顾'
    case 'quick_note': return '快速记账'
    default: return props.fact.source
  }
})
</script>

<template>
  <el-card shadow="hover" :body-style="{ padding: '12px' }" style="margin-bottom: 8px">
    <div style="display:flex;justify-content:space-between;align-items:flex-start;">
      <div style="flex:1;min-width:0;">
        <div style="font-weight:500;font-size:14px;">{{ title }}</div>
        <div v-if="subtitle" style="font-size:12px;color:#666;margin-top:2px;">{{ subtitle }}</div>
        <div style="margin-top:6px;">
          <el-tag size="small" :type="statusType" effect="plain">{{ statusLabel }}</el-tag>
          <el-tag size="small" type="info" effect="plain" style="margin-left:4px;">{{ sourceLabel }}</el-tag>
          <el-tag v-if="fact.confidence !== undefined" size="small" type="info" effect="plain" style="margin-left:4px;">
            置信度 {{ fact.confidence.toFixed(2) }}
          </el-tag>
        </div>
      </div>
      <div style="display:flex;gap:6px;flex-shrink:0;margin-left:8px;">
        <el-button v-if="showConfirm && fact.status === 'provisional'" size="small" type="primary" @click="confirm">
          确认
        </el-button>
        <el-button v-if="fact.status !== 'retired' && fact.status !== 'superseded'" size="small" @click="retire">
          退役
        </el-button>
      </div>
    </div>
  </el-card>
</template>
