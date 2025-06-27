<template>
  <div class="transactions-view">
    <!-- 搜索筛选区域 -->
    <el-card class="filter-card">
      <el-row :gutter="20">
        <el-col :xs="24" :sm="12" :md="6">
          <el-select v-model="filters.type" placeholder="选择类型" clearable @change="handleSearch">
            <el-option label="全部" value="" />
            <el-option label="收入" value="income" />
            <el-option label="支出" value="expense" />
          </el-select>
        </el-col>
        
        <el-col :xs="24" :sm="12" :md="6">
          <el-select v-model="filters.categoryId" placeholder="选择分类" clearable @change="handleSearch">
            <el-option
              v-for="category in store.categories"
              :key="category.id"
              :label="category.name"
              :value="category.id"
            >
              <div class="category-option">
                <span class="category-icon">{{ category.icon || '💰' }}</span>
                <span>{{ category.name }}</span>
              </div>
            </el-option>
          </el-select>
        </el-col>
        
        <el-col :xs="24" :sm="12" :md="8">
          <el-date-picker
            v-model="dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            format="YYYY-MM-DD"
            value-format="YYYY-MM-DD"
            @change="handleDateRangeChange"
          />
        </el-col>
        
        <el-col :xs="24" :sm="12" :md="4">
          <el-button type="primary" @click="handleSearch">
            <el-icon><Search /></el-icon>
            搜索
          </el-button>
        </el-col>
      </el-row>
    </el-card>
    
    <!-- 交易记录表格 -->
    <el-card class="table-card">
      <template #header>
        <div class="table-header">
          <div class="header-left">
            <span>交易记录</span>
            <el-tag v-if="filteredTransactions.length > 0" type="info" class="count-tag">
              共 {{ filteredTransactions.length }} 条
            </el-tag>
          </div>
          <div class="header-right">
            <el-button @click="showAddDialog = true">
              <el-icon><Plus /></el-icon>
              添加记录
            </el-button>
            <el-button @click="handleExport">
              <el-icon><Download /></el-icon>
              导出
            </el-button>
          </div>
        </div>
      </template>
      
      <el-table
        :data="paginatedTransactions"
        :loading="store.loading"
        stripe
        class="transactions-table"
        @sort-change="handleSortChange"
      >
        <el-table-column prop="date" label="日期" width="120" sortable>
          <template #default="{ row }">
            {{ formatDate(row.date) }}
          </template>
        </el-table-column>
        
        <el-table-column prop="type" label="类型" width="80">
          <template #default="{ row }">
            <el-tag :type="row.type === 'income' ? 'success' : 'danger'" size="small">
              {{ row.type === 'income' ? '收入' : '支出' }}
            </el-tag>
          </template>
        </el-table-column>
        
        <el-table-column prop="category_name" label="分类" width="120">
          <template #default="{ row }">
            <div class="category-cell">
              <span class="category-icon" :style="{ color: row.category_color }">
                {{ row.category_icon || '💰' }}
              </span>
              <span>{{ row.category_name }}</span>
            </div>
          </template>
        </el-table-column>
        
        <el-table-column prop="amount" label="金额" width="120" sortable>
          <template #default="{ row }">
            <span :class="['amount', row.type]">
              {{ row.type === 'income' ? '+' : '-' }}¥{{ formatAmount(row.amount) }}
            </span>
          </template>
        </el-table-column>
        
        <el-table-column prop="description" label="描述" min-width="150">
          <template #default="{ row }">
            <span>{{ row.description || '-' }}</span>
          </template>
        </el-table-column>
        
        <el-table-column prop="note" label="备注" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
            <span>{{ row.note || '-' }}</span>
          </template>
        </el-table-column>
        
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              size="small"
              text
              @click="handleEdit(row)"
            >
              <el-icon><Edit /></el-icon>
            </el-button>
            <el-button
              type="danger"
              size="small"
              text
              @click="handleDelete(row)"
            >
              <el-icon><Delete /></el-icon>
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      
      <!-- 分页 -->
      <div class="pagination-container">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredTransactions.length"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>
    
    <!-- 添加交易对话框 -->
    <AddTransactionDialog 
      v-model="showAddDialog" 
      @success="handleTransactionAdded"
    />
    
    <!-- 编辑交易对话框 -->
    <EditTransactionDialog
      v-model="showEditDialog"
      :transaction="editingTransaction"
      @success="handleTransactionEdited"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useAppStore, type TransactionWithCategory } from '../stores'
import dayjs from 'dayjs'
import AddTransactionDialog from '../components/AddTransactionDialog.vue'
import EditTransactionDialog from '../components/EditTransactionDialog.vue'

const store = useAppStore()

// 响应式数据
const showAddDialog = ref(false)
const showEditDialog = ref(false)
const editingTransaction = ref<TransactionWithCategory | null>(null)
const currentPage = ref(1)
const pageSize = ref(20)
const dateRange = ref<[string, string] | null>(null)

// 筛选条件
const filters = ref({
  type: '',
  categoryId: null as number | null,
  startDate: '',
  endDate: '',
  keyword: ''
})

// 排序条件
const sortConfig = ref({
  prop: 'date',
  order: 'descending'
})

// 计算属性
const filteredTransactions = computed(() => {
  let transactions = [...store.transactions]
  
  // 类型筛选
  if (filters.value.type) {
    transactions = transactions.filter(t => t.type === filters.value.type)
  }
  
  // 分类筛选
  if (filters.value.categoryId) {
    transactions = transactions.filter(t => t.category_id === filters.value.categoryId)
  }
  
  // 日期范围筛选
  if (filters.value.startDate && filters.value.endDate) {
    transactions = transactions.filter(t => 
      t.date >= filters.value.startDate && t.date <= filters.value.endDate
    )
  }
  
  // 排序
  if (sortConfig.value.prop) {
    transactions.sort((a, b) => {
      const aValue = a[sortConfig.value.prop as keyof TransactionWithCategory]
      const bValue = b[sortConfig.value.prop as keyof TransactionWithCategory]
       if (sortConfig.value.order === 'ascending') {
        return aValue > bValue ? 1 : -1
      } else {
        return aValue < bValue ? 1 : -1
      }
    })
  }
  
  return transactions
})

const paginatedTransactions = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return filteredTransactions.value.slice(start, end)
})

// 方法
const formatDate = (dateStr: string) => {
  return dayjs(dateStr).format('YYYY-MM-DD')
}

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const handleSearch = () => {
  currentPage.value = 1
}

const handleDateRangeChange = (dates: [string, string] | null) => {
  if (dates) {
    filters.value.startDate = dates[0]
    filters.value.endDate = dates[1]
  } else {
    filters.value.startDate = ''
    filters.value.endDate = ''
  }
  handleSearch()
}

const handleSortChange = ({ prop, order }: { prop: string; order: string }) => {
  sortConfig.value.prop = prop
  sortConfig.value.order = order
}

const handleSizeChange = (size: number) => {
  pageSize.value = size
  currentPage.value = 1
}

const handleCurrentChange = (page: number) => {
  currentPage.value = page
}

const handleEdit = (transaction: TransactionWithCategory) => {
  editingTransaction.value = transaction
  showEditDialog.value = true
}

const handleDelete = async (transaction: TransactionWithCategory) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除这条交易记录吗？`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    
    await store.deleteTransaction(transaction.id)
    ElMessage.success('删除成功')
  } catch (error) {
    if (error !== 'cancel') {
      console.error('删除交易记录失败:', error)
      ElMessage.error('删除失败，请重试')
    }
  }
}

const handleExport = async () => {
  try {
    // 这里可以添加导出逻辑
    ElMessage.info('导出功能开发中...')
  } catch (error) {
    console.error('导出失败:', error)
    ElMessage.error('导出失败，请重试')
  }
}

const handleTransactionAdded = () => {
  showAddDialog.value = false
  store.fetchTransactions()
}

const handleTransactionEdited = () => {
  showEditDialog.value = false
  editingTransaction.value = null
  store.fetchTransactions()
}

// 监听器
watch(() => store.transactions, () => {
  // 当交易记录更新时，重置到第一页
  currentPage.value = 1
})

onMounted(() => {
  // 确保分类数据已加载
  if (store.categories.length === 0) {
    store.fetchCategories()
  }
})
</script>

<style scoped>
.transactions-view {
  width: 100%;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-card :deep(.el-card__body) {
  padding: 20px;
}

.table-card {
  border-radius: 8px;
}

.table-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-right {
  display: flex;
  gap: 8px;
}

.count-tag {
  font-size: 12px;
}

.transactions-table {
  margin-bottom: 20px;
}

.category-cell {
  display: flex;
  align-items: center;
}

.category-icon {
  margin-right: 8px;
  font-size: 16px;
}

.category-option {
  display: flex;
  align-items: center;
}

.amount {
  font-weight: 600;
}

.amount.income {
  color: #67C23A;
}

.amount.expense {
  color: #F56C6C;
}

.pagination-container {
  display: flex;
  justify-content: center;
  padding: 20px 0;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .table-header {
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
  }
  
  .header-right {
    justify-content: center;
  }
  
  .transactions-table {
    font-size: 12px;
  }
  
  .pagination-container {
    overflow-x: auto;
  }
}
</style> 