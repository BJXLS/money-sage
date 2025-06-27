import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'

// 类型定义
export interface Category {
  id: number
  name: string
  icon?: string
  color?: string
  type: 'income' | 'expense'
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
  description?: string
  note?: string
  created_at: string
  updated_at: string
}

export interface TransactionWithCategory extends Transaction {
  category_name: string
  category_icon?: string
  category_color?: string
}

export interface Budget {
  id: number
  category_id: number
  amount: number
  period_type: 'weekly' | 'monthly' | 'yearly'
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
  
  // Actions
  const fetchCategories = async () => {
    try {
      loading.value = true
      const result = await invoke<Category[]>('get_categories')
      categories.value = result
    } catch (error) {
      console.error('获取分类失败:', error)
      throw error
    } finally {
      loading.value = false
    }
  }
  
  const fetchCategoriesByType = async (type: 'income' | 'expense') => {
    try {
      const result = await invoke<Category[]>('get_categories_by_type', { categoryType: type })
      return result
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
  }) => {
    try {
      const result = await invoke<number>('create_category', { category: categoryData })
      await fetchCategories() // 重新获取分类列表
      return result
    } catch (error) {
      console.error('创建分类失败:', error)
      throw error
    }
  }
  
  const deleteCategory = async (id: number) => {
    try {
      await invoke('delete_category', { id })
      await fetchCategories() // 重新获取分类列表
    } catch (error) {
      console.error('删除分类失败:', error)
      throw error
    }
  }
  
  const fetchTransactions = async (limit?: number, offset?: number) => {
    try {
      loading.value = true
      const result = await invoke<TransactionWithCategory[]>('get_transactions', { 
        limit: limit || pageSize.value, 
        offset: offset || (currentPage.value - 1) * pageSize.value 
      })
      transactions.value = result
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
      return result
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
    description?: string
    note?: string
  }) => {
    try {
      const result = await invoke<number>('create_transaction', { transaction: transactionData })
      await fetchTransactions() // 重新获取交易记录
      await fetchMonthlyStats() // 更新统计数据
      return result
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
    description?: string
    note?: string
  }) => {
    try {
      await invoke('update_transaction', { id, transaction: transactionData })
      await fetchTransactions() // 重新获取交易记录
      await fetchMonthlyStats() // 更新统计数据
    } catch (error) {
      console.error('更新交易记录失败:', error)
      throw error
    }
  }
  
  const deleteTransaction = async (id: number) => {
    try {
      await invoke('delete_transaction', { id })
      await fetchTransactions() // 重新获取交易记录
      await fetchMonthlyStats() // 更新统计数据
    } catch (error) {
      console.error('删除交易记录失败:', error)
      throw error
    }
  }
  
  const fetchMonthlyStats = async (months = 6) => {
    try {
      const result = await invoke<MonthlyStats[]>('get_monthly_stats', { months })
      monthlyStats.value = result
    } catch (error) {
      console.error('获取月度统计失败:', error)
      throw error
    }
  }
  
  const fetchCategoryStats = async (startDate: string, endDate: string, type: 'income' | 'expense') => {
    try {
      const result = await invoke<CategoryStats[]>('get_category_stats', {
        startDate,
        endDate,
        transactionType: type
      })
      categoryStats.value = result
      return result
    } catch (error) {
      console.error('获取分类统计失败:', error)
      throw error
    }
  }
  
  const fetchBudgets = async () => {
    try {
      const result = await invoke<BudgetProgress[]>('get_budgets')
      budgets.value = result
    } catch (error) {
      console.error('获取预算失败:', error)
      throw error
    }
  }
  
  const createBudget = async (budgetData: {
    category_id: number
    amount: number
    period_type: 'weekly' | 'monthly' | 'yearly'
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
    currentMonthIncome,
    currentMonthExpense,
    currentMonthBalance,
    
    // Actions
    fetchCategories,
    fetchCategoriesByType,
    createCategory,
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
    deleteBudget,
    importCSV,
    exportCSV,
    initializeData
  }
}) 