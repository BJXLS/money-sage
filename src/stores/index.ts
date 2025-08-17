import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'

// 类型声明
declare global {
  interface Window {
    __TAURI__?: any;
  }
}

// 安全的 invoke 包装函数
const safeInvoke = async <T = any>(command: string, args?: any): Promise<T | null> => {
  try {
    // 直接尝试调用 invoke，如果失败了再处理
    return await invoke<T>(command, args)
  } catch (error) {
    console.warn(`Failed to invoke ${command}:`, error)
    return null
  }
}

// 类型定义
export interface Category {
  id: number
  name: string
  icon?: string
  color?: string
  type: 'income' | 'expense'
  parent_id?: number | null
  is_system: boolean
  created_at: string
  updated_at: string
}

export interface Transaction {
  id: number
  date: string
  type: 'income' | 'expense'
  amount: number
  category_id: number
  budget_id?: number | null
  description?: string
  note?: string
  created_at: string
  updated_at: string
}

export interface TransactionWithCategory extends Transaction {
  category_name: string
  category_icon?: string
  category_color?: string
  budget_name?: string
}

export interface Budget {
  id: number
  name: string
  category_id: number
  amount: number
  budget_type: 'time' | 'event'
  period_type: 'daily' | 'weekly' | 'monthly' | 'yearly'
  start_date: string
  end_date?: string
  is_active: boolean
  created_at: string
  updated_at: string
}

export interface BudgetProgress extends Budget {
  category_name: string
  category_icon?: string
  category_color?: string
  spent: number
  remaining: number
  percentage: number
}

export interface MonthlyStats {
  month: string
  income: number
  expense: number
  balance: number
}

export interface CategoryStats {
  category_id: number
  category_name: string
  category_icon?: string
  category_color?: string
  amount: number
  percentage: number
}

export const useAppStore = defineStore('app', () => {
  // 状态
  const categories = ref<Category[]>([])
  const transactions = ref<TransactionWithCategory[]>([])
  const budgets = ref<BudgetProgress[]>([])
  const monthlyStats = ref<MonthlyStats[]>([])
  const categoryStats = ref<CategoryStats[]>([])
  const loading = ref(false)
  const currentPage = ref(1)
  const pageSize = ref(20)
  const totalTransactions = ref(0)
  
  // 计算属性
  const incomeCategories = computed(() => 
    categories.value.filter(cat => cat.type === 'income')
  )
  
  const expenseCategories = computed(() => 
    categories.value.filter(cat => cat.type === 'expense')
  )

  // 大类（顶级分类）
  const parentCategories = computed(() => 
    categories.value.filter(cat => !cat.parent_id)
  )

  // 收入大类
  const parentIncomeCategories = computed(() => 
    categories.value.filter(cat => cat.type === 'income' && !cat.parent_id)
  )

  // 支出大类
  const parentExpenseCategories = computed(() => 
    categories.value.filter(cat => cat.type === 'expense' && !cat.parent_id)
  )

  // 获取指定大类下的小类
  const getSubCategories = (parentId: number) => {
    return categories.value.filter(cat => cat.parent_id === parentId)
  }
  
  const currentMonthIncome = computed(() => {
    const currentMonth = dayjs().format('YYYY-MM')
    const currentData = monthlyStats.value.find(stat => stat.month === currentMonth)
    return currentData?.income || 0
  })
  
  const currentMonthExpense = computed(() => {
    const currentMonth = dayjs().format('YYYY-MM')
    const currentData = monthlyStats.value.find(stat => stat.month === currentMonth)
    return currentData?.expense || 0
  })
  
  const currentMonthBalance = computed(() => {
    return currentMonthIncome.value - currentMonthExpense.value
  })

  // 获取事件预算列表
  const eventBudgets = computed(() => {
    return budgets.value.filter(budget => budget.budget_type === 'event' && budget.is_active)
  })
  
  // Actions
  const fetchCategories = async () => {
    try {
      loading.value = true
      console.log('正在获取分类数据...')
      const result = await invoke<Category[]>('get_categories')
      console.log('获取到的分类数据:', result)
      categories.value = result || []
    } catch (error) {
      console.error('获取分类失败:', error)
      throw error
    } finally {
      loading.value = false
    }
  }
  
  const fetchCategoriesByType = async (type: 'income' | 'expense') => {
    try {
      const result = await safeInvoke<Category[]>('get_categories_by_type', { categoryType: type })
      return result || []
    } catch (error) {
      console.error('获取分类失败:', error)
      throw error
    }
  }
  
  const createCategory = async (categoryData: {
    name: string
    icon?: string
    color?: string
    type: 'income' | 'expense'
    parent_id?: number | null
  }) => {
    try {
      console.log('创建分类数据:', categoryData)
      const result = await safeInvoke<number>('create_category', { category: categoryData })
      console.log('创建分类结果:', result)
      // 总是刷新数据，无论是否在 Tauri 环境中
      await fetchCategories()
      return result || 0
    } catch (error) {
      console.error('创建分类失败:', error)
      throw error
    }
  }

  const updateCategory = async (id: number, categoryData: {
    name?: string
    icon?: string
    color?: string
    parent_id?: number | null
  }) => {
    try {
      await safeInvoke('update_category', { id, category: categoryData })
      // 总是刷新数据，无论是否在 Tauri 环境中
      await fetchCategories()
    } catch (error) {
      console.error('更新分类失败:', error)
      throw error
    }
  }
  
  const deleteCategory = async (id: number) => {
    try {
      await safeInvoke('delete_category', { id })
      // 总是刷新数据，无论是否在 Tauri 环境中
      await fetchCategories()
    } catch (error) {
      console.error('删除分类失败:', error)
      throw error
    }
  }
  
  const fetchTransactions = async (_limit?: number, _offset?: number) => {
    try {
      loading.value = true
      // 改为每次获取当月的所有记录
      const base = dayjs()
      const startDate = base.startOf('month').format('YYYY-MM-DD')
      const endDate = base.endOf('month').format('YYYY-MM-DD')
      console.log('正在获取当月交易记录...', startDate, endDate)
      const result = await invoke<TransactionWithCategory[]>('get_transactions_by_date_range', {
        startDate,
        endDate
      })
      console.log('获取到的当月交易记录:', result)
      transactions.value = result || []
    } catch (error) {
      console.error('获取交易记录失败:', error)
      throw error
    } finally {
      loading.value = false
    }
  }
  
  const fetchTransactionsByDateRange = async (startDate: string, endDate: string) => {
    try {
      loading.value = true
      const result = await invoke<TransactionWithCategory[]>('get_transactions_by_date_range', {
        startDate,
        endDate
      })
      transactions.value = result || []
      return result || []
    } catch (error) {
      console.error('获取交易记录失败:', error)
      throw error
    } finally {
      loading.value = false
    }
  }
  
  const createTransaction = async (transactionData: {
    date: string
    type: 'income' | 'expense'
    amount: number
    category_id: number
    budget_id?: number | null
    description?: string
    note?: string
  }) => {
    try {
      const result = await safeInvoke<number>('create_transaction', { transaction: transactionData })
      if (result !== null) {
        await Promise.all([
          fetchTransactions(), // 重新获取交易记录
          fetchMonthlyStats(), // 更新统计数据
          fetchBudgets()       // 同步预算进度
        ])
      }
      return result || 0
    } catch (error) {
      console.error('创建交易记录失败:', error)
      throw error
    }
  }
  
  const updateTransaction = async (id: number, transactionData: {
    date?: string
    type?: 'income' | 'expense'
    amount?: number
    category_id?: number
    budget_id?: number | null
    description?: string
    note?: string
  }) => {
    try {
      await invoke('update_transaction', { id, transaction: transactionData })
      await Promise.all([
        fetchTransactions(), // 重新获取交易记录
        fetchMonthlyStats(), // 更新统计数据
        fetchBudgets()       // 同步预算进度
      ])
    } catch (error) {
      console.error('更新交易记录失败:', error)
      throw error
    }
  }
  
  const deleteTransaction = async (id: number) => {
    try {
      await invoke('delete_transaction', { id })
      await Promise.all([
        fetchTransactions(), // 重新获取交易记录
        fetchMonthlyStats(), // 更新统计数据
        fetchBudgets()       // 同步预算进度
      ])
    } catch (error) {
      console.error('删除交易记录失败:', error)
      throw error
    }
  }
  
  const fetchMonthlyStats = async (months = 6) => {
    try {
      const result = await safeInvoke<MonthlyStats[]>('get_monthly_stats', { months })
      monthlyStats.value = result || []
    } catch (error) {
      console.error('获取月度统计失败:', error)
      throw error
    }
  }
  
  const fetchCategoryStats = async (startDate: string, endDate: string, type: 'income' | 'expense') => {
    try {
      const result = await safeInvoke<CategoryStats[]>('get_category_stats', {
        startDate,
        endDate,
        transactionType: type
      })
      categoryStats.value = result || []
      return result || []
    } catch (error) {
      console.error('获取分类统计失败:', error)
      throw error
    }
  }
  
  const fetchBudgets = async () => {
    try {
      const result = await safeInvoke<BudgetProgress[]>('get_budgets')
      budgets.value = result || []
    } catch (error) {
      console.error('获取预算失败:', error)
      throw error
    }
  }
  
  const createBudget = async (budgetData: {
    name: string
    category_id: number
    amount: number
    budget_type: 'time' | 'event'
    period_type: 'daily' | 'weekly' | 'monthly' | 'yearly'
    start_date: string
    end_date?: string
  }) => {
    try {
      const result = await invoke<number>('create_budget', { budget: budgetData })
      await fetchBudgets() // 重新获取预算列表
      return result
    } catch (error) {
      console.error('创建预算失败:', error)
      throw error
    }
  }
  
  const updateBudget = async (id: number, budgetData: {
    name?: string
    category_id?: number
    amount?: number
    budget_type?: 'time' | 'event'
    period_type?: 'weekly' | 'monthly' | 'yearly'
    start_date?: string
    end_date?: string
  }) => {
    try {
      await invoke('update_budget', { id, budget: budgetData })
      await fetchBudgets() // 重新获取预算列表
    } catch (error) {
      console.error('更新预算失败:', error)
      throw error
    }
  }
  
  const deleteBudget = async (id: number) => {
    try {
      await invoke('delete_budget', { id })
      await fetchBudgets() // 重新获取预算列表
    } catch (error) {
      console.error('删除预算失败:', error)
      throw error
    }
  }
  
  const importCSV = async (filePath: string) => {
    try {
      const result = await invoke<number[]>('import_csv_transactions', { filePath })
      await fetchTransactions() // 重新获取交易记录
      await fetchMonthlyStats() // 更新统计数据
      return result
    } catch (error) {
      console.error('导入CSV失败:', error)
      throw error
    }
  }
  
  const exportCSV = async (filePath: string, startDate: string, endDate: string) => {
    try {
      await invoke('export_csv_transactions', { filePath, startDate, endDate })
    } catch (error) {
      console.error('导出CSV失败:', error)
      throw error
    }
  }
  
  // 初始化数据
  const initializeData = async () => {
    try {
      loading.value = true
      await Promise.all([
        fetchCategories(),
        fetchTransactions(),
        fetchMonthlyStats(),
        fetchBudgets()
      ])
    } catch (error) {
      console.error('初始化数据失败:', error)
    } finally {
      loading.value = false
    }
  }
  
  return {
    // 状态
    categories,
    transactions,
    budgets,
    monthlyStats,
    categoryStats,
    loading,
    currentPage,
    pageSize,
    totalTransactions,
    
    // 计算属性
    incomeCategories,
    expenseCategories,
    parentCategories,
    parentIncomeCategories,
    parentExpenseCategories,
    getSubCategories,
    currentMonthIncome,
    currentMonthExpense,
    currentMonthBalance,
    eventBudgets,
    
    // Actions
    fetchCategories,
    fetchCategoriesByType,
    createCategory,
    updateCategory,
    deleteCategory,
    fetchTransactions,
    fetchTransactionsByDateRange,
    createTransaction,
    updateTransaction,
    deleteTransaction,
    fetchMonthlyStats,
    fetchCategoryStats,
    fetchBudgets,
    createBudget,
    updateBudget,
    deleteBudget,
    importCSV,
    exportCSV,
    initializeData
  }
}) 