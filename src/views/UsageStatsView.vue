<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import dayjs from 'dayjs'
import { useAppStore, type TokenUsageFilter, type TokenUsageSummary, type TokenUsageEntry } from '../stores'

const store = useAppStore()

const dateRange = ref<[string, string] | null>(null)
const configFilter = ref<number | null>(null)
const modelFilter = ref<string>('')
const successOnly = ref(false)

const configSummary = ref<TokenUsageSummary[]>([])
const modelSummary = ref<TokenUsageSummary[]>([])
const entries = ref<TokenUsageEntry[]>([])
const loading = ref(false)

const page = ref(1)
const pageSize = 50

const currentFilter = computed<TokenUsageFilter>(() => {
  const f: TokenUsageFilter = {}
  if (dateRange.value && dateRange.value[0]) f.start_date = `${dateRange.value[0]} 00:00:00`
  // 后端 SQL 使用 `< end_date`，这里传次日零点确保结束日整天被覆盖
  if (dateRange.value && dateRange.value[1]) {
    f.end_date = dayjs(dateRange.value[1]).add(1, 'day').format('YYYY-MM-DD 00:00:00')
  }
  if (configFilter.value != null) f.config_id = configFilter.value
  if (modelFilter.value.trim()) f.model = modelFilter.value.trim()
  if (successOnly.value) f.success_only = true
  return f
})

const refresh = async () => {
  loading.value = true
  try {
    const filter = currentFilter.value
    const [byConfig, byModel, list] = await Promise.all([
      // 直接使用每次调用返回值，避免并发时读取共享 state 导致统计串数据
      store.getTokenUsageSummary('config', filter).then((res) => structuredClone(res || [])),
      store.getTokenUsageSummary('model', filter).then((res) => structuredClone(res || [])),
      store.listTokenUsage({ ...filter, limit: pageSize, offset: (page.value - 1) * pageSize }),
    ])
    configSummary.value = byConfig as TokenUsageSummary[]
    modelSummary.value = byModel as TokenUsageSummary[]
    entries.value = list
  } finally {
    loading.value = false
  }
}

const totalCalls = computed(() => configSummary.value.reduce((s, x) => s + x.call_count, 0))
const totalTokens = computed(() => configSummary.value.reduce((s, x) => s + x.total_tokens, 0))
const totalPromptTokens = computed(() => configSummary.value.reduce((s, x) => s + x.prompt_tokens, 0))
const totalCompletionTokens = computed(() => configSummary.value.reduce((s, x) => s + x.completion_tokens, 0))
const totalSuccess = computed(() => configSummary.value.reduce((s, x) => s + x.success_count, 0))
const successRate = computed(() => {
  if (totalCalls.value === 0) return '—'
  return `${Math.round((totalSuccess.value / totalCalls.value) * 1000) / 10}%`
})

const knownConfigs = computed(() => {
  const map = new Map<number, string>()
  for (const it of configSummary.value) {
    if (it.config_id != null) map.set(it.config_id, it.config_name || `配置 ${it.config_id}`)
  }
  return [...map.entries()].map(([id, name]) => ({ id, name }))
})

const knownModels = computed(() => {
  const set = new Set<string>()
  for (const it of modelSummary.value) {
    if (it.model) set.add(it.model)
  }
  return [...set]
})

const formatNumber = (n: number) => {
  if (n >= 1e9) return `${(n / 1e9).toFixed(2)}B`
  if (n >= 1e6) return `${(n / 1e6).toFixed(2)}M`
  if (n >= 1e3) return `${(n / 1e3).toFixed(2)}K`
  return n.toLocaleString()
}

const formatTime = (s?: string | null) => {
  if (!s) return '—'
  return dayjs(s).format('YYYY-MM-DD HH:mm:ss')
}

const setQuickRange = (days: number) => {
  const end = dayjs().format('YYYY-MM-DD')
  const start = dayjs().subtract(days - 1, 'day').format('YYYY-MM-DD')
  dateRange.value = [start, end]
  page.value = 1
  void refresh()
}

const clearFilters = () => {
  dateRange.value = null
  configFilter.value = null
  modelFilter.value = ''
  successOnly.value = false
  page.value = 1
  void refresh()
}

const onPrevPage = async () => {
  if (page.value <= 1) return
  page.value -= 1
  await refresh()
}

const onNextPage = async () => {
  if (entries.value.length < pageSize) return
  page.value += 1
  await refresh()
}

const onPurgeOldLogs = async () => {
  const cutoff = dayjs().subtract(90, 'day').format('YYYY-MM-DD HH:mm:ss')
  try {
    const removed = await store.purgeTokenUsageLogs(cutoff)
    await refresh()
    // eslint-disable-next-line no-alert
    alert(`已清理 ${removed} 条 90 天前的记录`)
  } catch (e) {
    // eslint-disable-next-line no-alert
    alert(`清理失败: ${e}`)
  }
}

onMounted(() => void refresh())
</script>

<template>
  <div class="usage-stats">
    <!-- 筛选栏 -->
    <div class="filter-card">
      <div class="filter-row">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          unlink-panels
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
          style="width: 280px"
        />
        <el-select
          v-model="configFilter"
          placeholder="配置"
          clearable
          style="width: 200px"
        >
          <el-option
            v-for="cfg in knownConfigs"
            :key="cfg.id"
            :label="cfg.name"
            :value="cfg.id"
          />
        </el-select>
        <el-select
          v-model="modelFilter"
          placeholder="模型"
          clearable
          filterable
          allow-create
          style="width: 200px"
        >
          <el-option
            v-for="m in knownModels"
            :key="m"
            :label="m"
            :value="m"
          />
        </el-select>
        <el-checkbox v-model="successOnly">仅成功调用</el-checkbox>

        <div class="filter-actions">
          <el-button size="small" @click="setQuickRange(1)">今天</el-button>
          <el-button size="small" @click="setQuickRange(7)">近 7 天</el-button>
          <el-button size="small" @click="setQuickRange(30)">近 30 天</el-button>
          <el-button size="small" @click="clearFilters">清空</el-button>
          <el-button type="primary" size="small" @click="refresh" :loading="loading">
            <el-icon><Refresh /></el-icon>
            <span style="margin-left:4px">刷新</span>
          </el-button>
          <el-button size="small" @click="onPurgeOldLogs">
            <el-icon><Delete /></el-icon>
            <span style="margin-left:4px">清理 90 天前</span>
          </el-button>
        </div>
      </div>
    </div>

    <!-- 总览 -->
    <div class="overview-grid">
      <div class="overview-card">
        <div class="overview-label">总调用</div>
        <div class="overview-value">{{ formatNumber(totalCalls) }}</div>
        <div class="overview-sub">成功率 {{ successRate }}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">总 Token</div>
        <div class="overview-value">{{ formatNumber(totalTokens) }}</div>
        <div class="overview-sub">prompt + completion</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Prompt Tokens</div>
        <div class="overview-value">{{ formatNumber(totalPromptTokens) }}</div>
      </div>
      <div class="overview-card">
        <div class="overview-label">Completion Tokens</div>
        <div class="overview-value">{{ formatNumber(totalCompletionTokens) }}</div>
      </div>
    </div>

    <!-- 配置卡片 -->
    <div class="section-card">
      <div class="section-header">
        <span>按 LLM 配置</span>
        <span class="section-sub">点击或筛选配置查看详情</span>
      </div>
      <div v-if="configSummary.length === 0" class="empty-block">
        <el-empty description="暂无 token 用量数据" />
      </div>
      <div v-else class="config-grid">
        <div
          v-for="cfg in configSummary"
          :key="cfg.group_key"
          class="config-card"
          :class="{ active: configFilter === cfg.config_id }"
          @click="configFilter = cfg.config_id ?? null; page = 1; refresh()"
        >
          <div class="config-name">{{ cfg.config_name || '未保存的临时配置' }}</div>
          <div class="config-provider">{{ cfg.provider || '—' }}</div>
          <div class="config-tokens">
            <span class="big">{{ formatNumber(cfg.total_tokens) }}</span>
            <span class="unit">tokens</span>
          </div>
          <div class="config-meta">
            <span>{{ cfg.call_count }} 次调用</span>
            <span class="dot">·</span>
            <span>{{ cfg.success_count }} 成功</span>
          </div>
          <div class="config-bar">
            <div
              class="config-bar-fill"
              :style="{ width: `${cfg.call_count ? (cfg.success_count / cfg.call_count) * 100 : 0}%` }"
            ></div>
          </div>
          <div class="config-time">最近：{{ formatTime(cfg.last_used_at) }}</div>
        </div>
      </div>
    </div>

    <!-- 按模型聚合 -->
    <div class="section-card">
      <div class="section-header">
        <span>按模型</span>
        <span class="section-sub">同一配置下的不同 model 会单独聚合</span>
      </div>
      <el-table :data="modelSummary" v-loading="loading" empty-text="暂无数据">
        <el-table-column prop="model" label="模型" min-width="200" />
        <el-table-column prop="provider" label="供应商" width="140" />
        <el-table-column label="调用次数" width="110">
          <template #default="{ row }">
            <span>{{ row.call_count }}</span>
            <span v-if="row.success_count !== row.call_count" class="muted">
              （成功 {{ row.success_count }}）
            </span>
          </template>
        </el-table-column>
        <el-table-column label="Prompt" width="120">
          <template #default="{ row }">{{ formatNumber(row.prompt_tokens) }}</template>
        </el-table-column>
        <el-table-column label="Completion" width="120">
          <template #default="{ row }">{{ formatNumber(row.completion_tokens) }}</template>
        </el-table-column>
        <el-table-column label="Total" width="120">
          <template #default="{ row }">
            <strong>{{ formatNumber(row.total_tokens) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="最近使用" min-width="180">
          <template #default="{ row }">{{ formatTime(row.last_used_at) }}</template>
        </el-table-column>
      </el-table>
    </div>

    <!-- 明细 -->
    <div class="section-card">
      <div class="section-header">
        <span>调用明细</span>
        <span class="section-sub">每次 LLM 请求一行，按时间倒序</span>
      </div>
      <el-table :data="entries" v-loading="loading" empty-text="暂无数据">
        <el-table-column label="时间" width="170">
          <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
        </el-table-column>
        <el-table-column prop="agent_name" label="Agent" width="200" />
        <el-table-column prop="model" label="模型" min-width="160" />
        <el-table-column label="配置" width="160">
          <template #default="{ row }">
            <span>{{ row.config_name || '—' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="Round" width="80">
          <template #default="{ row }">
            <el-tag size="small" type="info">#{{ row.round_index }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="Prompt" width="100">
          <template #default="{ row }">{{ formatNumber(row.prompt_tokens) }}</template>
        </el-table-column>
        <el-table-column label="Completion" width="120">
          <template #default="{ row }">{{ formatNumber(row.completion_tokens) }}</template>
        </el-table-column>
        <el-table-column label="Total" width="100">
          <template #default="{ row }">
            <strong>{{ formatNumber(row.total_tokens) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="耗时" width="90">
          <template #default="{ row }">
            {{ row.duration_ms != null ? `${row.duration_ms} ms` : '—' }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.success" type="success" size="small">成功</el-tag>
            <el-tooltip v-else :content="row.error_message || '失败'" placement="top">
              <el-tag type="danger" size="small">失败</el-tag>
            </el-tooltip>
          </template>
        </el-table-column>
        <el-table-column prop="finish_reason" label="结束原因" min-width="120" />
      </el-table>
      <div class="pagination">
        <el-button size="small" :disabled="page <= 1 || loading" @click="onPrevPage">上一页</el-button>
        <span class="page-num">第 {{ page }} 页</span>
        <el-button
          size="small"
          :disabled="entries.length < pageSize || loading"
          @click="onNextPage"
        >下一页</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.usage-stats {
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-4);
}

.filter-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) var(--ms-space-5);
  box-shadow: var(--ms-shadow-sm);
}

.filter-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ms-space-3);
}

.filter-actions {
  display: flex;
  gap: var(--ms-space-2);
  margin-left: auto;
  flex-wrap: wrap;
}

.overview-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(180px, 1fr));
  gap: var(--ms-space-3);
}

@media (max-width: 1100px) {
  .overview-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

.overview-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) var(--ms-space-5);
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-1);
  min-height: 92px;
  box-shadow: var(--ms-shadow-sm);
}

.overview-label {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.overview-value {
  color: var(--ms-text-primary);
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.5px;
}

.overview-sub {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
}

.section-card {
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) var(--ms-space-5);
  box-shadow: var(--ms-shadow-sm);
}

.section-header {
  display: flex;
  align-items: baseline;
  gap: var(--ms-space-3);
  margin-bottom: var(--ms-space-4);
}

.section-header > span:first-child {
  color: var(--ms-text-primary);
  font-weight: 600;
  font-size: var(--ms-text-base);
}

.section-sub {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
}

.empty-block {
  padding: var(--ms-space-6) 0;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--ms-space-3);
}

.config-card {
  background: var(--ms-bg-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-lg);
  padding: var(--ms-space-4) var(--ms-space-5);
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-2);
}

.config-card:hover {
  border-color: var(--ms-primary-500);
  background: rgba(99, 102, 241, 0.06);
  transform: translateY(-1px);
}

.config-card.active {
  border-color: var(--ms-primary-500);
  background: rgba(99, 102, 241, 0.12);
  box-shadow: 0 0 0 1px rgba(99, 102, 241, 0.2);
}

.config-name {
  color: var(--ms-text-primary);
  font-weight: 600;
  font-size: var(--ms-text-sm);
}

.config-provider {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
  letter-spacing: 0.02em;
}

.config-tokens {
  display: flex;
  align-items: baseline;
  gap: var(--ms-space-1);
}

.config-tokens .big {
  color: var(--ms-primary-500);
  font-size: 22px;
  font-weight: 700;
}

.config-tokens .unit {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
}

.config-meta {
  color: var(--ms-text-secondary);
  font-size: var(--ms-text-xs);
  display: flex;
  gap: 4px;
}

.config-meta .dot {
  color: var(--ms-text-tertiary);
}

.config-bar {
  height: 4px;
  background: var(--ms-border-subtle);
  border-radius: 999px;
  overflow: hidden;
}

.config-bar-fill {
  height: 100%;
  background: var(--ms-gradient-primary);
  transition: width 0.3s;
}

.config-time {
  color: var(--ms-text-tertiary);
  font-size: 11px;
}

.muted {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
  margin-left: 4px;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: var(--ms-space-3);
  padding-top: var(--ms-space-3);
}

.page-num {
  color: var(--ms-text-secondary);
  font-size: var(--ms-text-sm);
}
</style>
