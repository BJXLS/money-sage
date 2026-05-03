import type { MemoryHistoryEntry } from '../stores'

const opMap: Record<string, string> = {
  insert: '新增',
  update: '修改',
  retire: '退役',
  auto_merge: '自动合并',
  supersede: '被取代',
  preset_apply: '应用预设',
  undo: '撤销',
  rejected: '被拒绝',
  auto_promote: '自动晋升',
}

const typeMap: Record<string, string> = {
  classification_rule: '分类规则',
  recurring_event: '固定事件',
  financial_goal: '财务目标',
  user_profile: '用户画像',
  agent_role: '人格设定',
}

export function formatHistoryOp(op: string, factType?: string): string {
  return `${opMap[op] || op} ${typeMap[factType || ''] || factType || '记忆'}`
}

export function extractChangeSummary(entry: MemoryHistoryEntry): string {
  if (entry.op === 'rejected') {
    const reason = entry.after_json?.reason || ''
    return `因 ${reason} 被拒绝`
  }

  if (entry.op === 'insert' || entry.op === 'preset_apply') {
    const factType = entry.after_json?.fact_type || ''
    const key = entry.after_json?.key || ''
    return `新增：${key || typeMap[factType] || factType}`
  }

  if (entry.op === 'retire') {
    return '该记忆已退役'
  }

  if (entry.op === 'auto_merge') {
    return '相同内容被引用，置信度提升'
  }

  if (entry.op === 'auto_promote') {
    return `从待确认晋升为生效（引用 ${entry.before_json?.usage_count || '?'} 次）`
  }

  // update / supersede / undo
  const before = entry.before_json || {}
  const after = entry.after_json || {}

  if (entry.op === 'supersede') {
    return '被新版本取代'
  }

  if (entry.op === 'undo') {
    const undoneOp = after.history_id ? `撤销历史记录 #${after.history_id}` : '撤销操作'
    return undoneOp
  }

  // update: 找出变化的顶层 key
  const changedKeys = Object.keys(after).filter(
    (k) => JSON.stringify(before[k]) !== JSON.stringify(after[k])
  )

  if (changedKeys.length === 0) {
    return '无实质变化'
  }

  const keyLabels: Record<string, string> = {
    key: '键',
    value_json: '内容',
    status: '状态',
    confidence: '置信度',
    pattern: '匹配模式',
    target_category_path: '目标分类',
    title: '标题',
    cron: '周期',
    amount: '金额',
    metric: '指标',
    text: '文本',
    display_name: '显示名称',
    user_address: '称呼',
    tone: '语气',
    traits: '特质',
    do: '应该做',
    dont: '不要做',
  }

  const labels = changedKeys.map((k) => keyLabels[k] || k).join('、')
  return `修改了 ${labels}`
}
