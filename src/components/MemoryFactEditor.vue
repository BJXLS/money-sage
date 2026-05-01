<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { useAppStore, type FactType } from '../stores'

const props = defineProps<{ type: FactType }>()
const emit = defineEmits<{ (e: 'saved'): void }>()
const store = useAppStore()

// 4 套独立表单状态，避免切换 fact_type 时字段串味
const ruleForm = reactive({
  pattern: '',
  match_type: 'contains' as 'exact' | 'contains' | 'regex',
  target_category_path: '',
  default_amount: null as number | null,
})

const eventForm = reactive({
  title: '',
  cron: '',
  amount: null as number | null,
  amount_tolerance: 0.1,
  transaction_type: 'expense' as 'income' | 'expense',
  category_path: '',
  enabled: true,
})

const goalForm = reactive({
  title: '',
  metric: 'category_spend' as
    | 'category_spend'
    | 'total_spend'
    | 'savings_rate'
    | 'category_income'
    | 'budget_remaining',
  filter: '{}',
  period: 'monthly' as 'daily' | 'weekly' | 'monthly' | 'yearly',
  target: null as number | null,
  direction: 'le' as 'le' | 'ge' | 'eq',
  priority: 'normal' as 'low' | 'normal' | 'high',
})

const profileForm = reactive({
  text: '',
  tags: '',
})

const reset = () => {
  ruleForm.pattern = ''
  ruleForm.match_type = 'contains'
  ruleForm.target_category_path = ''
  ruleForm.default_amount = null
  eventForm.title = ''
  eventForm.cron = ''
  eventForm.amount = null
  eventForm.amount_tolerance = 0.1
  eventForm.transaction_type = 'expense'
  eventForm.category_path = ''
  eventForm.enabled = true
  goalForm.title = ''
  goalForm.metric = 'category_spend'
  goalForm.filter = '{}'
  goalForm.period = 'monthly'
  goalForm.target = null
  goalForm.direction = 'le'
  goalForm.priority = 'normal'
  profileForm.text = ''
  profileForm.tags = ''
}

watch(() => props.type, reset)

const buildPayload = (): { key: string | null; value_json: Record<string, any> } | null => {
  switch (props.type) {
    case 'classification_rule': {
      if (!ruleForm.pattern || !ruleForm.target_category_path) {
        ElMessage.warning('请至少填写匹配模式与目标分类')
        return null
      }
      return {
        key: ruleForm.pattern,
        value_json: {
          pattern: ruleForm.pattern,
          match_type: ruleForm.match_type,
          target_category_path: ruleForm.target_category_path,
          default_amount: ruleForm.default_amount,
        },
      }
    }
    case 'recurring_event': {
      if (!eventForm.title || !eventForm.cron) {
        ElMessage.warning('请至少填写事件名称与 cron 表达式')
        return null
      }
      return {
        key: eventForm.title,
        value_json: {
          title: eventForm.title,
          cron: eventForm.cron,
          amount: eventForm.amount,
          amount_tolerance: eventForm.amount_tolerance,
          transaction_type: eventForm.transaction_type,
          category_path: eventForm.category_path,
          enabled: eventForm.enabled,
        },
      }
    }
    case 'financial_goal': {
      if (!goalForm.title || goalForm.target === null) {
        ElMessage.warning('请至少填写目标名称与目标值')
        return null
      }
      let filter: any = {}
      try {
        filter = goalForm.filter ? JSON.parse(goalForm.filter) : {}
      } catch (e) {
        ElMessage.error('Filter 必须是合法 JSON')
        return null
      }
      return {
        key: `${goalForm.metric}:${goalForm.title}`,
        value_json: {
          title: goalForm.title,
          metric: goalForm.metric,
          filter,
          period: goalForm.period,
          target: goalForm.target,
          direction: goalForm.direction,
          priority: goalForm.priority,
        },
      }
    }
    case 'user_profile': {
      if (!profileForm.text) {
        ElMessage.warning('请填写画像内容')
        return null
      }
      const tags = profileForm.tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean)
      return {
        key: null,
        value_json: { text: profileForm.text, tags },
      }
    }
    default:
      return null
  }
}

const save = async () => {
  const payload = buildPayload()
  if (!payload) return
  await store.upsertMemoryFact({
    fact_type: props.type,
    key: payload.key,
    value_json: payload.value_json,
    source: 'user',
  })
  reset()
  emit('saved')
}

const isRule = computed(() => props.type === 'classification_rule')
const isEvent = computed(() => props.type === 'recurring_event')
const isGoal = computed(() => props.type === 'financial_goal')
const isProfile = computed(() => props.type === 'user_profile')
</script>

<template>
  <el-form inline label-width="100px" style="margin-top: 8px">
    <template v-if="isRule">
      <el-form-item label="匹配模式">
        <el-input v-model="ruleForm.pattern" placeholder="如：星巴克" style="width: 180px" />
      </el-form-item>
      <el-form-item label="匹配方式">
        <el-select v-model="ruleForm.match_type" style="width: 120px">
          <el-option label="包含" value="contains" />
          <el-option label="精确" value="exact" />
          <el-option label="正则" value="regex" />
        </el-select>
      </el-form-item>
      <el-form-item label="目标分类">
        <el-input v-model="ruleForm.target_category_path" placeholder="如：餐饮/咖啡" style="width: 180px" />
      </el-form-item>
      <el-form-item label="默认金额">
        <el-input-number v-model="ruleForm.default_amount" :precision="2" :min="0" controls-position="right" style="width: 140px" />
      </el-form-item>
    </template>

    <template v-else-if="isEvent">
      <el-form-item label="事件名称">
        <el-input v-model="eventForm.title" placeholder="如：房租" style="width: 180px" />
      </el-form-item>
      <el-form-item label="Cron">
        <el-input v-model="eventForm.cron" placeholder="0 0 1 * *" style="width: 180px" />
      </el-form-item>
      <el-form-item label="金额">
        <el-input-number v-model="eventForm.amount" :precision="2" :min="0" controls-position="right" style="width: 140px" />
      </el-form-item>
      <el-form-item label="容差">
        <el-input-number v-model="eventForm.amount_tolerance" :precision="2" :min="0" :max="1" :step="0.05" controls-position="right" style="width: 120px" />
      </el-form-item>
      <el-form-item label="收支">
        <el-select v-model="eventForm.transaction_type" style="width: 100px">
          <el-option label="支出" value="expense" />
          <el-option label="收入" value="income" />
        </el-select>
      </el-form-item>
      <el-form-item label="分类">
        <el-input v-model="eventForm.category_path" placeholder="如：住房/房租" style="width: 180px" />
      </el-form-item>
      <el-form-item label="启用">
        <el-switch v-model="eventForm.enabled" />
      </el-form-item>
    </template>

    <template v-else-if="isGoal">
      <el-form-item label="目标名称">
        <el-input v-model="goalForm.title" placeholder="如：餐饮月控" style="width: 180px" />
      </el-form-item>
      <el-form-item label="指标">
        <el-select v-model="goalForm.metric" style="width: 160px">
          <el-option label="分类支出" value="category_spend" />
          <el-option label="总支出" value="total_spend" />
          <el-option label="储蓄率" value="savings_rate" />
          <el-option label="分类收入" value="category_income" />
          <el-option label="预算剩余" value="budget_remaining" />
        </el-select>
      </el-form-item>
      <el-form-item label="周期">
        <el-select v-model="goalForm.period" style="width: 120px">
          <el-option label="日" value="daily" />
          <el-option label="周" value="weekly" />
          <el-option label="月" value="monthly" />
          <el-option label="年" value="yearly" />
        </el-select>
      </el-form-item>
      <el-form-item label="目标值">
        <el-input-number v-model="goalForm.target" :precision="2" controls-position="right" style="width: 160px" />
      </el-form-item>
      <el-form-item label="方向">
        <el-select v-model="goalForm.direction" style="width: 100px">
          <el-option label="≤" value="le" />
          <el-option label="≥" value="ge" />
          <el-option label="=" value="eq" />
        </el-select>
      </el-form-item>
      <el-form-item label="优先级">
        <el-select v-model="goalForm.priority" style="width: 100px">
          <el-option label="低" value="low" />
          <el-option label="中" value="normal" />
          <el-option label="高" value="high" />
        </el-select>
      </el-form-item>
      <el-form-item label="Filter(JSON)">
        <el-input v-model="goalForm.filter" type="textarea" :rows="1" placeholder='{"category":"餐饮"}' style="width: 260px" />
      </el-form-item>
    </template>

    <template v-else-if="isProfile">
      <el-form-item label="画像内容">
        <el-input v-model="profileForm.text" type="textarea" :rows="2" placeholder="如：我每月初发薪，喜欢咖啡" style="width: 360px" />
      </el-form-item>
      <el-form-item label="标签">
        <el-input v-model="profileForm.tags" placeholder="逗号分隔，如：作息,偏好" style="width: 220px" />
      </el-form-item>
    </template>

    <el-form-item>
      <el-button type="primary" @click="save">保存</el-button>
    </el-form-item>
  </el-form>
</template>
