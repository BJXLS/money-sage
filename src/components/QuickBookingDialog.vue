<template>
  <el-dialog
    v-model="visible"
    title="快速记账"
    width="600px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <div class="quick-booking-content">
      <div class="input-section">
        <div class="input-label">
          <el-icon class="label-icon"><EditPen /></el-icon>
          <span>请描述您的收支情况</span>
        </div>
        <el-input
          v-model="inputText"
          type="textarea"
          :rows="8"
          placeholder="例如：今天中午在餐厅花了38元吃午饭&#10;昨天收到工资5000元&#10;买了一本编程书籍89元"
          class="text-input"
          maxlength="500"
          show-word-limit
          clearable
        />
        <div class="input-tips">
          <div class="tip-item">
            <el-icon><InfoFilled /></el-icon>
            <span>支持多条记录，每行一条</span>
          </div>
          <div class="tip-item">
            <el-icon><InfoFilled /></el-icon>
            <span>AI将自动识别金额、分类和时间</span>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose" class="cancel-btn">
          取消
        </el-button>
        <el-button @click="handleClear" class="clear-btn">
          清空
        </el-button>
        <el-button 
          type="primary" 
          @click="handleSubmit" 
          :loading="processing"
          :disabled="!inputText.trim()"
          class="submit-btn"
        >
          <el-icon v-if="!processing"><Lightning /></el-icon>
          {{ processing ? '处理中...' : '智能记账' }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { InfoFilled, EditPen, Lightning } from '@element-plus/icons-vue'

// Props和Emits
const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'success': [data: any]
}>()

// 响应式数据
const visible = ref(false)
const inputText = ref('')
const processing = ref(false)

// 监听props变化
watch(
  () => props.modelValue,
  (newVal) => {
    visible.value = newVal
    if (newVal) {
      // 打开对话框时清空内容
      inputText.value = ''
      processing.value = false
    }
  },
  { immediate: true }
)

// 监听visible变化
watch(visible, (newVal) => {
  emit('update:modelValue', newVal)
})

// 清空输入
const handleClear = () => {
  inputText.value = ''
}

// 提交处理
const handleSubmit = async () => {
  if (!inputText.value.trim()) {
    ElMessage.warning('请输入记账信息')
    return
  }

  processing.value = true
  try {
    // 调用后端API进行文本处理
    const result = await invoke('process_quick_booking_text', {
      text: inputText.value.trim()
    })

    ElMessage.success('记账信息处理成功！')
    emit('success', result)
    handleClose()
  } catch (error) {
    console.error('处理记账信息失败:', error)
    ElMessage.error('处理失败，请检查输入格式或稍后重试')
  } finally {
    processing.value = false
  }
}

// 关闭对话框
const handleClose = () => {
  if (inputText.value.trim() && !processing.value) {
    ElMessageBox.confirm(
      '确定要关闭吗？输入的内容将会丢失。',
      '确认关闭',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    ).then(() => {
      visible.value = false
      inputText.value = ''
    }).catch(() => {
      // 用户取消关闭
    })
  } else {
    visible.value = false
    inputText.value = ''
  }
}
</script>

<style scoped>
.quick-booking-content {
  padding: 8px 0;
}

.input-section {
  margin-bottom: 16px;
}

.input-label {
  display: flex;
  align-items: center;
  margin-bottom: 12px;
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
}

.label-icon {
  margin-right: 8px;
  color: #409eff;
  font-size: 18px;
}

.text-input {
  margin-bottom: 16px;
}

.input-tips {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tip-item {
  display: flex;
  align-items: center;
  font-size: 13px;
  color: #b0b0b0;
}

.tip-item .el-icon {
  margin-right: 6px;
  color: #409eff;
  font-size: 14px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.cancel-btn,
.clear-btn {
  padding: 8px 20px;
}

.submit-btn {
  padding: 8px 24px;
  font-weight: 600;
}

.submit-btn .el-icon {
  margin-right: 6px;
}

/* 深度样式 */
:deep(.el-dialog) {
  background: #2a2a2a;
  border: 1px solid #404040;
}

:deep(.el-dialog__header) {
  background: #2a2a2a;
  border-bottom: 1px solid #404040;
  padding: 20px 24px 16px;
}

:deep(.el-dialog__title) {
  color: #ffffff;
  font-size: 18px;
  font-weight: 600;
}

:deep(.el-dialog__headerbtn .el-dialog__close) {
  color: #b0b0b0;
}

:deep(.el-dialog__headerbtn .el-dialog__close:hover) {
  color: #ffffff;
}

:deep(.el-dialog__body) {
  background: #2a2a2a;
  color: #ffffff;
  padding: 24px;
}

:deep(.el-dialog__footer) {
  background: #2a2a2a;
  border-top: 1px solid #404040;
  padding: 16px 24px 20px;
}

:deep(.text-input .el-textarea__inner) {
  background: #404040;
  border: 1px solid #606060;
  border-radius: 8px;
  color: #ffffff;
  font-size: 14px;
  line-height: 1.6;
  padding: 16px;
  min-height: 160px;
  resize: vertical;
}

:deep(.text-input .el-textarea__inner:focus) {
  border-color: #409eff;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

:deep(.text-input .el-textarea__inner::placeholder) {
  color: #909399;
  font-size: 13px;
  line-height: 1.5;
}

:deep(.el-input__count) {
  background: transparent;
  color: #b0b0b0;
  font-size: 12px;
}

:deep(.el-button) {
  border-radius: 6px;
  font-weight: 500;
}

:deep(.cancel-btn) {
  background: #404040;
  border-color: #606060;
  color: #ffffff;
}

:deep(.cancel-btn:hover) {
  background: #606060;
  border-color: #808080;
}

:deep(.clear-btn) {
  background: #606060;
  border-color: #707070;
  color: #ffffff;
}

:deep(.clear-btn:hover) {
  background: #707070;
  border-color: #909090;
}

:deep(.submit-btn) {
  background: #409eff;
  border-color: #409eff;
}

:deep(.submit-btn:hover) {
  background: #66b1ff;
  border-color: #66b1ff;
}

:deep(.submit-btn:disabled) {
  background: #404040;
  border-color: #606060;
  color: #909399;
}
</style> 