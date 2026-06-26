<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, ArrowRight, ArrowDown, Money, Edit, Delete } from '@element-plus/icons-vue'
import { useAppStore } from '../stores'
import type { ImportConflictStrategy, ImportPreviewResult } from '../stores'
import dayjs from 'dayjs'
import { open, save } from '@tauri-apps/plugin-dialog'
import MoneyCard from '../components/ui/MoneyCard.vue'
import MoneyButton from '../components/ui/MoneyButton.vue'

const store = useAppStore()

const currentDate = ref(dayjs())
const selectedDate = ref(dayjs().format('YYYY-MM-DD'))
const showRecordDialog = ref(false)
const showImportDialog = ref(false)
const importing = ref(false)
const importFilePath = ref('')
const importPreview = ref<ImportPreviewResult | null>(null)
const importWarnings = ref<string[]>([])
const importStrategy = ref<ImportConflictStrategy>('upsert')

const showCategoryPanel = ref(false)
const hoveredParentId = ref<number | null>(null)
const categorySelectRef = ref<HTMLElement>()

const editingTransactionId = ref<number | null>(null)

const transactionForm = ref({
  type: 'expense' as 'income' | 'expense',
  amount: '',
  categoryId: null as number | null,
  budgetId: null as number | null,
  account: '现金',
  time: '00:00',
  note: ''
})

const weekdays = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']

const currentMonthText = computed(() => currentDate.value.format('YYYY年M月'))
const selectedDateText = computed(() => dayjs(selectedDate.value).format('YYYY年M月D日'))

const monthIncome = computed(() => {
  return store.transactions
    .filter(t => t.type === 'income')
    .reduce((sum, t) => sum + t.amount, 0)
})

const monthExpense = computed(() => {
  return store.transactions
    .filter(t => t.type === 'expense')
    .reduce((sum, t) => sum + t.amount, 0)
})

const monthBalance = computed(() => monthIncome.value - monthExpense.value)

const availableParentCategories = computed(() => {
  return store.categories.filter(cat => cat.type === transactionForm.value.type && !cat.parent_id)
})

const selectedCategory = computed(() => {
  if (!transactionForm.value.categoryId) return null
  return store.categories.find(cat => cat.id === transactionForm.value.categoryId)
})

const allActiveBudgets = computed(() => {
  return store.budgets.filter(b => b.is_active).sort((a, b) => (b.remaining - a.remaining))
})

const hoveredSubCategories = computed(() => {
  if (!hoveredParentId.value) return []
  return store.categories.filter(cat => cat.parent_id === hoveredParentId.value)
})

const dailyTransactions = computed(() => {
  return store.transactions.filter(t => dayjs(t.date).format('YYYY-MM-DD') === selectedDate.value)
})

const dailyStats = computed(() => {
  const dayTransactions = dailyTransactions.value
  const income = dayTransactions.filter(t => t.type === 'income').reduce((sum, t) => sum + t.amount, 0)
  const expense = dayTransactions.filter(t => t.type === 'expense').reduce((sum, t) => sum + t.amount, 0)
  return { income, expense, balance: income - expense }
})

const calendarDays = computed(() => {
  const year = currentDate.value.year()
  const month = currentDate.value.month()
  const firstDay = dayjs().year(year).month(month).date(1)
  const lastDay = firstDay.endOf('month')
  const startDate = firstDay.startOf('week')
  const endDate = lastDay.endOf('week')

  const days = []
  let current = startDate

  while (current.isBefore(endDate) || current.isSame(endDate)) {
    const dateStr = current.format('YYYY-MM-DD')
    const dayTransactions = store.transactions.filter(t => dayjs(t.date).format('YYYY-MM-DD') === dateStr)
    const totalExpense = dayTransactions.filter(t => t.type === 'expense').reduce((sum, t) => sum + t.amount, 0)
    const totalIncome = dayTransactions.filter(t => t.type === 'income').reduce((sum, t) => sum + t.amount, 0)

    days.push({
      date: dateStr,
      dayNumber: current.date(),
      isOtherMonth: current.month() !== month,
      isToday: current.isSame(dayjs(), 'day'),
      transactions: dayTransactions,
      totalExpense,
      totalIncome,
    })

    current = current.add(1, 'day')
  }

  return days
})

const prevMonth = () => { currentDate.value = currentDate.value.subtract(1, 'month') }
const nextMonth = () => { currentDate.value = currentDate.value.add(1, 'month') }

const selectDate = (date: string) => {
  selectedDate.value = date
  showRecordDialog.value = true
  resetForm()
}

const resetForm = () => {
  editingTransactionId.value = null
  transactionForm.value = {
    type: 'expense',
    amount: '',
    categoryId: null,
    budgetId: null,
    account: '现金',
    time: '00:00',
    note: ''
  }
}

const handleDialogClose = () => {
  showRecordDialog.value = false
  resetForm()
}

const saveTransaction = async () => {
  if (!transactionForm.value.amount || !transactionForm.value.categoryId) {
    ElMessage.warning('请填写完整的交易信息')
    return
  }

  try {
    const data = {
      type: transactionForm.value.type,
      amount: parseFloat(transactionForm.value.amount),
      category_id: transactionForm.value.categoryId,
      date: selectedDate.value,
      budget_id: transactionForm.value.budgetId,
      description: transactionForm.value.note,
      note: transactionForm.value.note
    }

    if (editingTransactionId.value) {
      await store.updateTransaction(editingTransactionId.value, data)
      ElMessage.success('记录更新成功')
    } else {
      await store.createTransaction(data)
      ElMessage.success('记录保存成功')
    }

    const monthStart = currentDate.value.startOf('month').format('YYYY-MM-DD')
    const monthEnd = currentDate.value.endOf('month').format('YYYY-MM-DD')
    await store.fetchTransactionsByDateRange(monthStart, monthEnd)
    handleDialogClose()
  } catch (error) {
    ElMessage.error('保存失败，请重试')
  }
}

const formatAmount = (amount: number) => {
  return Math.abs(amount).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const formatTime = (dateTime: string) => dayjs(dateTime).format('HH:mm')

const normalizeDialogPath = (picked: string | string[] | null) => {
  if (!picked) return null
  if (Array.isArray(picked)) return picked[0] || null
  return picked
}

const handleExportExcel = async () => {
  try {
    const filePath = await save({
      title: '导出 Excel',
      defaultPath: `money-note-export-${dayjs().format('YYYYMMDD-HHmmss')}.xlsx`,
      filters: [{ name: 'Excel 文件', extensions: ['xlsx'] }]
    })
    const path = normalizeDialogPath(filePath)
    if (!path) return
    const result = await store.exportDataFile(path, 'excel')
    ElMessage.success(`导出成功：${result.transactions} 条交易`)
  } catch (e: any) {
    ElMessage.error(`导出失败：${e?.toString?.() || e}`)
  }
}

const handleExportBackup = async () => {
  try {
    const filePath = await save({
      title: '导出 MoneySage 备份',
      defaultPath: `money-note-backup-${dayjs().format('YYYYMMDD-HHmmss')}.moneysage`,
      filters: [{ name: 'MoneySage 备份', extensions: ['moneysage', 'json'] }]
    })
    const path = normalizeDialogPath(filePath)
    if (!path) return
    const result = await store.exportDataFile(path, 'money_sage')
    ElMessage.success(`备份成功：${result.transactions} 条交易，${result.memory_facts} 条记忆`)
  } catch (e: any) {
    ElMessage.error(`备份失败：${e?.toString?.() || e}`)
  }
}

const handlePickImportFile = async () => {
  try {
    const picked = await open({
      title: '选择导入文件',
      multiple: false,
      filters: [
        { name: '支持文件', extensions: ['xlsx', 'moneysage', 'json'] },
        { name: 'Excel 文件', extensions: ['xlsx'] },
        { name: 'MoneySage 文件', extensions: ['moneysage', 'json'] }
      ]
    })
    const path = normalizeDialogPath(picked)
    if (!path) return
    importFilePath.value = path
    const preview = await store.previewImportData(path)
    importPreview.value = preview
    importWarnings.value = preview.warnings || []
    showImportDialog.value = true
  } catch (e: any) {
    ElMessage.error(`读取导入预览失败：${e?.toString?.() || e}`)
  }
}

const confirmImport = async (forceImport = false) => {
  if (!importFilePath.value) {
    ElMessage.warning('请先选择导入文件')
    return
  }

  if (forceImport) {
    try {
      await ElMessageBox.confirm(
        '备份文件校验未通过，强制导入可能引入不一致或损坏的数据。是否仍要继续？',
        '强制导入确认',
        { type: 'warning', confirmButtonText: '继续强制导入', cancelButtonText: '取消' }
      )
    } catch { return }
  }

  if (importStrategy.value === 'replace_all') {
    try {
      await ElMessageBox.confirm(
        '你选择了“清空后全量导入”，会覆盖当前数据。是否继续？',
        '高风险操作确认',
        { type: 'warning', confirmButtonText: '继续导入', cancelButtonText: '取消' }
      )
    } catch { return }
  }

  importing.value = true
  try {
    const result = await store.importDataFile(importFilePath.value, importStrategy.value, forceImport)
    showImportDialog.value = false
    ElMessage.success(`导入完成：新增 ${result.inserted}，更新 ${result.updated}，跳过 ${result.skipped}`)
  } catch (e: any) {
    ElMessage.error(`导入失败：${e?.toString?.() || e}`)
  } finally {
    importing.value = false
  }
}

const toggleCategoryPanel = () => {
  showCategoryPanel.value = !showCategoryPanel.value
  if (!showCategoryPanel.value) hoveredParentId.value = null
}

const setHoveredParent = (parentId: number) => { hoveredParentId.value = parentId }

const selectCategory = (category: any) => {
  transactionForm.value.categoryId = category.id
  showCategoryPanel.value = false
  hoveredParentId.value = null
}

const handleClickOutside = (event: MouseEvent) => {
  if (categorySelectRef.value && !categorySelectRef.value.contains(event.target as Node)) {
    showCategoryPanel.value = false
    hoveredParentId.value = null
  }
}

const editTransaction = (transaction: any) => {
  editingTransactionId.value = transaction.id
  transactionForm.value = {
    type: transaction.type,
    amount: transaction.amount.toString(),
    categoryId: transaction.category_id,
    budgetId: transaction.budget_id ?? null,
    account: '现金',
    time: formatTime(transaction.created_at),
    note: transaction.description || transaction.note || ''
  }
  showRecordDialog.value = true
}

const deleteTransaction = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这条记录吗？', '确认删除', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })
    await store.deleteTransaction(id)
    ElMessage.success('删除成功')
    const monthStart = currentDate.value.startOf('month').format('YYYY-MM-DD')
    const monthEnd = currentDate.value.endOf('month').format('YYYY-MM-DD')
    await store.fetchTransactionsByDateRange(monthStart, monthEnd)
  } catch (error) {
    if (error !== 'cancel') ElMessage.error('删除失败，请重试')
  }
}

onMounted(() => {
  const monthStart = currentDate.value.startOf('month').format('YYYY-MM-DD')
  const monthEnd = currentDate.value.endOf('month').format('YYYY-MM-DD')
  store.fetchTransactionsByDateRange(monthStart, monthEnd)
  store.fetchCategories()
  document.addEventListener('click', handleClickOutside)
})

watch(currentDate, (newVal) => {
  const monthStart = newVal.startOf('month').format('YYYY-MM-DD')
  const monthEnd = newVal.endOf('month').format('YYYY-MM-DD')
  store.fetchTransactionsByDateRange(monthStart, monthEnd)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<template>
  <div class="transactions-view">
    <!-- Month Navigation -->
    <div class="month-toolbar">
      <div class="month-selector">
        <button class="nav-btn" @click="prevMonth">
          <el-icon><ArrowLeft /></el-icon>
        </button>
        <h2 class="month-title">{{ currentMonthText }}</h2>
        <button class="nav-btn" @click="nextMonth">
          <el-icon><ArrowRight /></el-icon>
        </button>
      </div>
      <div class="month-summary">
        <div class="summary-pill income">
          <span class="dot"></span>
          <span class="label">收入</span>
          <span class="value">¥ {{ formatAmount(monthIncome) }}</span>
        </div>
        <div class="summary-pill expense">
          <span class="dot"></span>
          <span class="label">支出</span>
          <span class="value">¥ {{ formatAmount(monthExpense) }}</span>
        </div>
        <div class="summary-pill balance">
          <span class="dot"></span>
          <span class="label">结余</span>
          <span class="value">¥ {{ formatAmount(monthBalance) }}</span>
        </div>
      </div>
      <div class="data-tools">
        <MoneyButton variant="secondary" size="sm" @click="handleExportExcel">导出 Excel</MoneyButton>
        <MoneyButton variant="secondary" size="sm" @click="handleExportBackup">导出备份</MoneyButton>
        <MoneyButton variant="primary" size="sm" @click="handlePickImportFile">导入数据</MoneyButton>
      </div>
    </div>

    <div class="main-grid">
      <!-- Calendar -->
      <MoneyCard no-padding class="calendar-card">
        <div class="calendar-container">
          <div class="calendar-header">
            <div v-for="day in weekdays" :key="day" class="weekday">{{ day }}</div>
          </div>
          <div class="calendar-body">
            <div
              v-for="day in calendarDays"
              :key="day.date"
              class="calendar-day"
              :class="{
                'other-month': day.isOtherMonth,
                'today': day.isToday,
                'selected': day.date === selectedDate,
                'has-expense': day.totalExpense > 0,
                'has-income': day.totalIncome > 0,
              }"
              @click="selectDate(day.date)"
            >
              <div class="day-header">
                <span class="day-number">{{ day.dayNumber }}</span>
                <span v-if="day.isToday" class="today-dot"></span>
              </div>
              <div class="day-summary">
                <div v-if="day.totalExpense > 0" class="expense-amount">-¥{{ formatAmount(day.totalExpense) }}</div>
                <div v-if="day.totalIncome > 0" class="income-amount">+¥{{ formatAmount(day.totalIncome) }}</div>
                <div v-if="day.transactions.length > 0" class="transaction-count">{{ day.transactions.length }} 笔</div>
              </div>
            </div>
          </div>
        </div>
      </MoneyCard>

      <!-- Day Detail Panel -->
      <div class="detail-column">
        <MoneyCard>
          <div class="date-header">
            <h3>{{ selectedDateText }}</h3>
            <span v-if="selectedDate === dayjs().format('YYYY-MM-DD')" class="today-badge">今天</span>
          </div>
          <div class="daily-summary">
            <div class="summary-item income">
              <span class="label">收入</span>
              <span class="amount">¥{{ formatAmount(dailyStats.income) }}</span>
            </div>
            <div class="summary-item expense">
              <span class="label">支出</span>
              <span class="amount">¥{{ formatAmount(dailyStats.expense) }}</span>
            </div>
            <div class="summary-item balance">
              <span class="label">结余</span>
              <span class="amount">¥{{ formatAmount(dailyStats.balance) }}</span>
            </div>
          </div>
        </MoneyCard>

        <MoneyCard title="快速记账">
          <div class="quick-add-form">
            <div class="type-segment">
              <button
                :class="['segment-btn', { active: transactionForm.type === 'expense' }]"
                @click="transactionForm.type = 'expense'"
              >支出</button>
              <button
                :class="['segment-btn', { active: transactionForm.type === 'income' }]"
                @click="transactionForm.type = 'income'"
              >收入</button>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label>金额</label>
                <el-input-number v-model="transactionForm.amount" :min="0.01" :precision="2" :controls="false" placeholder="0.00" />
              </div>
              <div class="form-group">
                <label>分类</label>
                <div class="category-tree-select" ref="categorySelectRef">
                  <div class="category-display" :class="{ active: showCategoryPanel }" @click="toggleCategoryPanel">
                    <div v-if="selectedCategory" class="selected-category">
                      <span class="category-icon" :style="{ color: selectedCategory.color }">{{ selectedCategory.icon || '📁' }}</span>
                      <span>{{ selectedCategory.name }}</span>
                    </div>
                    <span v-else class="placeholder">选择分类</span>
                    <el-icon class="arrow-icon" :class="{ rotate: showCategoryPanel }"><ArrowDown /></el-icon>
                  </div>
                  <div v-if="showCategoryPanel" class="category-panel">
                    <div class="parent-categories">
                      <div
                        v-for="parent in availableParentCategories"
                        :key="parent.id"
                        class="parent-category-item"
                        @mouseenter="setHoveredParent(parent.id)"
                        @click="selectCategory(parent)"
                      >
                        <span class="category-icon" :style="{ color: parent.color }">{{ parent.icon || '📁' }}</span>
                        <span class="category-name">{{ parent.name }}</span>
                        <el-icon class="arrow-right"><ArrowRight /></el-icon>
                      </div>
                    </div>
                    <div class="sub-categories">
                      <template v-if="hoveredSubCategories.length > 0">
                        <div
                          v-for="sub in hoveredSubCategories"
                          :key="sub.id"
                          class="sub-category-item"
                          @click="selectCategory(sub)"
                        >
                          <span class="category-icon" :style="{ color: sub.color }">{{ sub.icon || '📋' }}</span>
                          <span class="category-name">{{ sub.name }}</span>
                        </div>
                      </template>
                      <div v-else class="sub-categories-placeholder">
                        <span class="placeholder-text">{{ hoveredParentId ? '没有小类' : '请选择左侧大类' }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label>预算</label>
                <el-select v-if="transactionForm.type === 'expense'" v-model="transactionForm.budgetId" placeholder="选择预算（可选）" clearable>
                  <el-option
                    v-for="b in allActiveBudgets"
                    :key="b.id"
                    :label="`${b.name}（${b.category_name}）`"
                    :value="b.id"
                  />
                </el-select>
                <el-input v-else disabled value="-"></el-input>
              </div>
              <div class="form-group">
                <label>账户</label>
                <el-select v-model="transactionForm.account">
                  <el-option label="现金" value="现金" />
                  <el-option label="银行卡" value="银行卡" />
                  <el-option label="支付宝" value="支付宝" />
                  <el-option label="微信" value="微信" />
                </el-select>
              </div>
            </div>
            <div class="form-group">
              <label>备注</label>
              <el-input v-model="transactionForm.note" type="textarea" :rows="2" placeholder="添加备注..." maxlength="100" show-word-limit />
            </div>
            <MoneyButton variant="primary" class="save-btn" @click="saveTransaction">
              <el-icon><Money /></el-icon>
              <span>保存</span>
            </MoneyButton>
          </div>
        </MoneyCard>

        <MoneyCard title="当日记录">
          <div class="daily-records">
            <div
              v-for="record in dailyTransactions"
              :key="record.id"
              class="record-item"
            >
              <div class="record-left">
                <div class="record-icon" :style="{ color: record.category_color }">{{ record.category_icon || '💰' }}</div>
                <div class="record-info">
                  <div class="record-desc">{{ record.description || record.category_name }}</div>
                  <div class="record-time">{{ formatTime(record.created_at) }}</div>
                </div>
              </div>
              <div class="record-right">
                <div class="record-amount" :class="record.type">{{ record.type === 'income' ? '+' : '-' }}¥{{ formatAmount(record.amount) }}</div>
                <div class="record-actions">
                  <el-button @click="editTransaction(record)" type="primary" size="small" text><el-icon><Edit /></el-icon></el-button>
                  <el-button @click="deleteTransaction(record.id)" type="danger" size="small" text><el-icon><Delete /></el-icon></el-button>
                </div>
              </div>
            </div>
            <div v-if="dailyTransactions.length === 0" class="no-records">暂无记录</div>
          </div>
        </MoneyCard>
      </div>
    </div>

    <!-- Edit Dialog (for mobile / alternative) -->
    <el-dialog v-model="showRecordDialog" :title="editingTransactionId ? '编辑记录' : '新增记录'" width="520px" :before-close="handleDialogClose">
      <div class="dialog-form">
        <div class="date-header-dialog">
          <h3>{{ selectedDateText }}</h3>
          <div class="daily-summary">
            <div class="summary-item income"><span class="label">收入</span><span class="amount">¥{{ formatAmount(dailyStats.income) }}</span></div>
            <div class="summary-item expense"><span class="label">支出</span><span class="amount">¥{{ formatAmount(dailyStats.expense) }}</span></div>
            <div class="summary-item balance"><span class="label">结余</span><span class="amount">¥{{ formatAmount(dailyStats.balance) }}</span></div>
          </div>
        </div>
        <div class="type-segment">
          <button :class="['segment-btn', { active: transactionForm.type === 'expense' }]" @click="transactionForm.type = 'expense'">支出</button>
          <button :class="['segment-btn', { active: transactionForm.type === 'income' }]" @click="transactionForm.type = 'income'">收入</button>
        </div>
        <el-form label-width="60px">
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="金额"><el-input-number v-model="transactionForm.amount" :min="0.01" :precision="2" :controls="false" placeholder="0.00" style="width:100%" /></el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="分类">
                <div class="category-tree-select" ref="categorySelectRef">
                  <div class="category-display" :class="{ active: showCategoryPanel }" @click="toggleCategoryPanel">
                    <div v-if="selectedCategory" class="selected-category">
                      <span class="category-icon" :style="{ color: selectedCategory.color }">{{ selectedCategory.icon || '📁' }}</span>
                      <span>{{ selectedCategory.name }}</span>
                    </div>
                    <span v-else class="placeholder">选择分类</span>
                    <el-icon class="arrow-icon" :class="{ rotate: showCategoryPanel }"><ArrowDown /></el-icon>
                  </div>
                  <div v-if="showCategoryPanel" class="category-panel">
                    <div class="parent-categories">
                      <div v-for="parent in availableParentCategories" :key="parent.id" class="parent-category-item" @mouseenter="setHoveredParent(parent.id)" @click="selectCategory(parent)">
                        <span class="category-icon" :style="{ color: parent.color }">{{ parent.icon || '📁' }}</span>
                        <span class="category-name">{{ parent.name }}</span>
                        <el-icon class="arrow-right"><ArrowRight /></el-icon>
                      </div>
                    </div>
                    <div class="sub-categories">
                      <template v-if="hoveredSubCategories.length > 0">
                        <div v-for="sub in hoveredSubCategories" :key="sub.id" class="sub-category-item" @click="selectCategory(sub)">
                          <span class="category-icon" :style="{ color: sub.color }">{{ sub.icon || '📋' }}</span>
                          <span class="category-name">{{ sub.name }}</span>
                        </div>
                      </template>
                      <div v-else class="sub-categories-placeholder"><span class="placeholder-text">{{ hoveredParentId ? '没有小类' : '请选择左侧大类' }}</span></div>
                    </div>
                  </div>
                </div>
              </el-form-item>
            </el-col>
          </el-row>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="预算">
                <el-select v-if="transactionForm.type === 'expense'" v-model="transactionForm.budgetId" placeholder="选择预算（可选）" clearable style="width:100%">
                  <el-option v-for="b in allActiveBudgets" :key="b.id" :label="`${b.name}（${b.category_name}）`" :value="b.id" />
                </el-select>
                <el-input v-else disabled value="-"></el-input>
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="账户">
                <el-select v-model="transactionForm.account" style="width:100%">
                  <el-option label="现金" value="现金" />
                  <el-option label="银行卡" value="银行卡" />
                  <el-option label="支付宝" value="支付宝" />
                  <el-option label="微信" value="微信" />
                </el-select>
              </el-form-item>
            </el-col>
          </el-row>
          <el-form-item label="备注"><el-input v-model="transactionForm.note" type="textarea" :rows="2" placeholder="添加备注..." maxlength="100" show-word-limit /></el-form-item>
        </el-form>
        <div class="daily-records">
          <h4>当日记录</h4>
          <div class="records-list">
            <div v-for="record in dailyTransactions" :key="record.id" class="record-item">
              <div class="record-left">
                <div class="record-icon" :style="{ color: record.category_color }">{{ record.category_icon || '💰' }}</div>
                <div class="record-info">
                  <div class="record-desc">{{ record.description || record.category_name }}</div>
                  <div class="record-time">{{ formatTime(record.created_at) }}</div>
                </div>
              </div>
              <div class="record-right">
                <div class="record-amount" :class="record.type">{{ record.type === 'income' ? '+' : '-' }}¥{{ formatAmount(record.amount) }}</div>
                <div class="record-actions">
                  <el-button @click="editTransaction(record)" type="primary" size="small" text><el-icon><Edit /></el-icon></el-button>
                  <el-button @click="deleteTransaction(record.id)" type="danger" size="small" text><el-icon><Delete /></el-icon></el-button>
                </div>
              </div>
            </div>
            <div v-if="dailyTransactions.length === 0" class="no-records">暂无记录</div>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="handleDialogClose">取消</el-button>
        <el-button type="primary" @click="saveTransaction">
          <el-icon><Money /></el-icon>
          {{ editingTransactionId ? '更新记录' : '保存记录' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- Import Dialog -->
    <el-dialog v-model="showImportDialog" title="导入数据" width="640px">
      <div class="import-panel">
        <p v-if="importFilePath"><strong>文件：</strong>{{ importFilePath }}</p>
        <p v-if="importPreview"><strong>类型：</strong>{{ importPreview.file_type }}</p>
        <p v-if="importPreview?.schema_version !== undefined"><strong>Schema：</strong>v{{ importPreview.schema_version }}</p>
        <el-alert v-if="importPreview?.checksum_valid === false" title="备份校验未通过：文件可能已被修改或损坏。如确认来源可信，可点击下方“强制导入”继续。" type="error" :closable="false" style="margin-top: 8px" />
        <el-table v-if="importPreview" :data="importPreview.items" size="small" style="margin-top: 8px">
          <el-table-column prop="table" label="数据表" width="180" />
          <el-table-column prop="rows" label="行数" />
          <el-table-column prop="estimated_insert" label="预计新增" />
          <el-table-column prop="estimated_update" label="预计更新" />
          <el-table-column prop="estimated_skip" label="预计跳过" />
        </el-table>
        <el-alert v-for="(w, idx) in importWarnings" :key="idx" :title="w" type="warning" :closable="false" style="margin-top: 8px" />
        <el-form label-width="120px" style="margin-top: 12px">
          <el-form-item label="冲突策略">
            <el-radio-group v-model="importStrategy">
              <el-radio label="upsert">更新并插入（推荐）</el-radio>
              <el-radio label="skip">冲突跳过</el-radio>
              <el-radio label="replace_all">清空后全量导入</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </div>
      <template #footer>
        <el-button @click="showImportDialog = false">取消</el-button>
        <el-button v-if="importPreview?.checksum_valid !== false" type="primary" :loading="importing" @click="confirmImport(false)">确认导入</el-button>
        <el-button v-else type="danger" :loading="importing" @click="confirmImport(true)">强制导入</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.transactions-view {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.month-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.month-selector {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-btn {
  width: 36px;
  height: 36px;
  border-radius: var(--ms-radius-md);
  border: 1px solid var(--ms-border-subtle);
  background-color: var(--ms-surface-secondary);
  color: var(--ms-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.nav-btn:hover {
  background-color: var(--ms-surface-hover);
  color: var(--ms-text-primary);
}

.month-title {
  margin: 0;
  font-size: var(--ms-text-2xl);
  font-weight: var(--ms-font-bold);
  color: var(--ms-text-primary);
  min-width: 140px;
  text-align: center;
}

.month-summary {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.summary-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: var(--ms-radius-full);
  background-color: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  font-size: var(--ms-text-sm);
}

.summary-pill .dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.summary-pill.income .dot { background-color: var(--ms-income); }
.summary-pill.expense .dot { background-color: var(--ms-expense); }
.summary-pill.balance .dot { background-color: var(--ms-primary-500); }

.summary-pill .label {
  color: var(--ms-text-secondary);
}

.summary-pill .value {
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
  font-variant-numeric: tabular-nums;
}

.data-tools {
  display: flex;
  gap: 8px;
}

.main-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 24px;
  align-items: start;
}

.detail-column {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

/* Calendar */
.calendar-card :deep(.ms-card-body) {
  padding: 0;
}

.calendar-container {
  padding: 16px;
}

.calendar-header {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
  margin-bottom: 4px;
}

.weekday {
  text-align: center;
  padding: 10px 4px;
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.calendar-body {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
}

.calendar-day {
  min-height: 100px;
  padding: 8px;
  border-radius: var(--ms-radius-lg);
  background-color: var(--ms-surface-secondary);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.calendar-day:hover {
  background-color: var(--ms-surface-hover);
  border-color: var(--ms-border-default);
}

.calendar-day.other-month {
  background-color: var(--ms-bg-secondary);
  opacity: 0.6;
}

.calendar-day.today {
  background-color: rgba(99, 102, 241, 0.08);
  border-color: var(--ms-primary-500);
}

.calendar-day.selected {
  background-color: rgba(99, 102, 241, 0.12);
  border-color: var(--ms-primary-500);
  box-shadow: 0 0 0 1px var(--ms-primary-500);
}

.day-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.day-number {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.today-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ms-gradient-primary);
}

.day-summary {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.expense-amount {
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-expense);
  font-variant-numeric: tabular-nums;
}

.income-amount {
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-income);
  font-variant-numeric: tabular-nums;
}

.transaction-count {
  font-size: 10px;
  color: var(--ms-text-tertiary);
}

/* Day detail */
.date-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.date-header h3 {
  margin: 0;
  font-size: var(--ms-text-lg);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.today-badge {
  padding: 2px 8px;
  border-radius: var(--ms-radius-full);
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-medium);
  background-color: rgba(99, 102, 241, 0.1);
  color: var(--ms-primary-500);
}

.daily-summary {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  text-align: center;
}

.daily-summary .summary-item {
  padding: 12px 8px;
  border-radius: var(--ms-radius-lg);
  background-color: var(--ms-bg-secondary);
}

.daily-summary .summary-item .label {
  display: block;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-bottom: 4px;
}

.daily-summary .summary-item .amount {
  display: block;
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  font-variant-numeric: tabular-nums;
}

.daily-summary .summary-item.income .amount { color: var(--ms-income); }
.daily-summary .summary-item.expense .amount { color: var(--ms-expense); }
.daily-summary .summary-item.balance .amount { color: var(--ms-text-primary); }

/* Quick add form */
.quick-add-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.type-segment {
  display: flex;
  padding: 4px;
  border-radius: var(--ms-radius-md);
  background-color: var(--ms-bg-tertiary);
  border: 1px solid var(--ms-border-subtle);
}

.segment-btn {
  flex: 1;
  padding: 6px 16px;
  border-radius: var(--ms-radius-md);
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.segment-btn.active {
  color: white;
  background: var(--ms-gradient-primary);
  box-shadow: var(--ms-shadow-sm);
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-group label {
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-secondary);
}

.save-btn {
  width: 100%;
  justify-content: center;
}

/* Category tree select */
.category-tree-select {
  position: relative;
  width: 100%;
}

.category-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  height: 32px;
  background-color: var(--ms-bg-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  cursor: pointer;
  transition: all 0.2s ease;
}

.category-display:hover {
  border-color: var(--ms-border-default);
}

.category-display.active {
  border-color: var(--ms-border-focus);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

.selected-category {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ms-text-primary);
  font-size: var(--ms-text-sm);
}

.placeholder {
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-sm);
}

.arrow-icon {
  transition: transform 0.25s ease;
  color: var(--ms-text-tertiary);
}

.arrow-icon.rotate {
  transform: rotate(180deg);
  color: var(--ms-primary-500);
}

.category-panel {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 1000;
  background-color: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-lg);
  box-shadow: var(--ms-shadow-lg);
  display: flex;
  max-height: 280px;
  overflow: hidden;
  width: 360px;
}

.parent-categories {
  width: 180px;
  border-right: 1px solid var(--ms-border-subtle);
  overflow-y: auto;
}

.sub-categories {
  width: 180px;
  overflow-y: auto;
}

.parent-category-item,
.sub-category-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  cursor: pointer;
  transition: background-color 0.15s ease;
  gap: 8px;
}

.parent-category-item:hover,
.sub-category-item:hover {
  background-color: var(--ms-surface-hover);
}

.parent-category-item .category-name,
.sub-category-item .category-name {
  flex: 1;
  color: var(--ms-text-secondary);
  font-size: var(--ms-text-sm);
}

.parent-category-item:hover .category-name,
.sub-category-item:hover .category-name {
  color: var(--ms-text-primary);
}

.arrow-right {
  color: var(--ms-text-tertiary);
  font-size: 12px;
}

.sub-categories-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100px;
}

.placeholder-text {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-tertiary);
}

/* Records */
.daily-records h4 {
  margin: 0 0 14px 0;
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
  font-weight: var(--ms-font-semibold);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.records-list {
  max-height: 240px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.record-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  border-radius: var(--ms-radius-lg);
  transition: background-color 0.15s ease;
}

.record-item:hover {
  background-color: var(--ms-surface-hover);
}

.record-item:hover .record-actions {
  opacity: 1;
}

.record-left {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}

.record-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.record-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--ms-bg-tertiary);
  border-radius: var(--ms-radius-md);
  font-size: 16px;
  flex-shrink: 0;
}

.record-info {
  min-width: 0;
}

.record-desc {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.record-time {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.record-amount {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-semibold);
  font-variant-numeric: tabular-nums;
}

.record-amount.income { color: var(--ms-income); }
.record-amount.expense { color: var(--ms-expense); }

.record-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.no-records {
  text-align: center;
  color: var(--ms-text-tertiary);
  padding: 24px;
  font-size: var(--ms-text-sm);
}

/* Dialog */
.dialog-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.date-header-dialog h3 {
  margin: 0 0 12px 0;
  font-size: var(--ms-text-lg);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.date-header-dialog .daily-summary {
  margin-bottom: 8px;
}

.import-panel {
  color: var(--ms-text-secondary);
}

@media (max-width: 1024px) {
  .main-grid {
    grid-template-columns: 1fr;
  }

  .detail-column {
    display: none;
  }

  .month-toolbar {
    flex-direction: column;
    align-items: flex-start;
  }
}

@media (max-width: 768px) {
  .calendar-day {
    min-height: 72px;
  }

  .record-actions {
    opacity: 1;
  }
}
</style>
