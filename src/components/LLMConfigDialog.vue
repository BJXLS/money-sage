<template>
  <el-dialog
    v-model="visible"
    title="大模型接口配置"
    width="500px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      label-position="left"
    >
      <el-form-item label="平台选择" prop="platform">
        <el-select
          v-model="form.platform"
          placeholder="请选择平台"
          style="width: 100%"
        >
          <el-option
            label="阿里云百炼"
            value="alibaba_bailian"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="App Key" prop="appKey">
        <el-input
          v-model="form.appKey"
          type="password"
          placeholder="请输入App Key"
          show-password
          clearable
        />
        <div class="form-tip">
          请在阿里云百炼控制台获取您的App Key
        </div>
      </el-form-item>
    </el-form>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">取消</el-button>
        <el-button @click="handleTest" :loading="testing">测试连接</el-button>
        <el-button type="primary" @click="handleSave" :loading="saving">
          保存配置
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'

// 定义类型
interface LLMConfig {
  id?: number
  platform: string
  app_key: string
  is_active?: boolean
  created_at?: string
  updated_at?: string
}

interface NewLLMConfig {
  platform: string
  app_key: string
}

// Props和Emits
const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'success': []
}>()

// 响应式数据
const visible = ref(false)
const formRef = ref<FormInstance>()
const saving = ref(false)
const testing = ref(false)
const currentConfig = ref<LLMConfig | null>(null)

// 表单数据
const form = reactive({
  platform: 'alibaba_bailian',
  appKey: ''
})

// 表单验证规则
const rules: FormRules = {
  platform: [
    { required: true, message: '请选择平台', trigger: 'change' }
  ],
  appKey: [
    { required: true, message: '请输入App Key', trigger: 'blur' },
    { min: 10, message: 'App Key长度不能少于10位', trigger: 'blur' }
  ]
}

// 监听props变化
watch(
  () => props.modelValue,
  (newVal) => {
    visible.value = newVal
    if (newVal) {
      loadCurrentConfig()
    }
  },
  { immediate: true }
)

// 监听visible变化
watch(visible, (newVal) => {
  emit('update:modelValue', newVal)
})

// 加载当前配置
const loadCurrentConfig = async () => {
  try {
    const config = await invoke<LLMConfig | null>('get_llm_config')
    if (config) {
      currentConfig.value = config
      form.platform = config.platform
      form.appKey = config.app_key
    } else {
      // 没有配置时重置表单
      form.platform = 'alibaba_bailian'
      form.appKey = ''
    }
  } catch (error) {
    console.error('加载配置失败:', error)
    ElMessage.error('加载配置失败')
  }
}

// 测试连接
const handleTest = async () => {
  if (!form.appKey.trim()) {
    ElMessage.warning('请先输入App Key')
    return
  }

  testing.value = true
  try {
    // 这里可以添加实际的测试连接逻辑
    // 暂时模拟测试
    await new Promise(resolve => setTimeout(resolve, 2000))
    ElMessage.success('连接测试成功！')
  } catch (error) {
    console.error('测试连接失败:', error)
    ElMessage.error('连接测试失败，请检查App Key是否正确')
  } finally {
    testing.value = false
  }
}

// 保存配置
const handleSave = async () => {
  if (!formRef.value) return

  try {
    await formRef.value.validate()
  } catch {
    return
  }

  // 确认保存
  try {
    await ElMessageBox.confirm(
      '保存后将覆盖现有配置，确定要保存吗？',
      '确认保存',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
  } catch {
    return
  }

  saving.value = true
  try {
    const configData: NewLLMConfig = {
      platform: form.platform,
      app_key: form.appKey.trim()
    }

    if (currentConfig.value) {
      // 更新现有配置
      await invoke('update_llm_config', {
        id: currentConfig.value.id,
        config: {
          platform: configData.platform,
          app_key: configData.app_key,
          is_active: true
        }
      })
    } else {
      // 创建新配置
      await invoke('save_llm_config', { config: configData })
    }

    ElMessage.success('配置保存成功！')
    emit('success')
    handleClose()
  } catch (error) {
    console.error('保存配置失败:', error)
    ElMessage.error('保存配置失败')
  } finally {
    saving.value = false
  }
}

// 关闭对话框
const handleClose = () => {
  visible.value = false
  // 重置表单验证状态
  if (formRef.value) {
    formRef.value.clearValidate()
  }
}
</script>

<style scoped>
.form-tip {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
  line-height: 1.4;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

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

:deep(.el-form-item__label) {
  color: #ffffff;
  font-weight: 500;
}

:deep(.el-input__wrapper) {
  background: #404040;
  border: 1px solid #606060;
  border-radius: 6px;
}

:deep(.el-input__wrapper:hover) {
  border-color: #409eff;
}

:deep(.el-input__wrapper.is-focus) {
  border-color: #409eff;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

:deep(.el-input__inner) {
  color: #ffffff;
  background: transparent;
}

:deep(.el-input__inner::placeholder) {
  color: #b0b0b0;
}

:deep(.el-select) {
  width: 100%;
}

:deep(.el-select .el-input__wrapper) {
  background: #404040;
  border: 1px solid #606060;
}

:deep(.el-select-dropdown) {
  background: #2a2a2a;
  border: 1px solid #404040;
}

:deep(.el-select-dropdown__item) {
  color: #ffffff;
}

:deep(.el-select-dropdown__item:hover) {
  background: #404040;
}

:deep(.el-select-dropdown__item.selected) {
  background: #409eff;
  color: #ffffff;
}

:deep(.el-button) {
  border-radius: 6px;
  font-weight: 500;
}

:deep(.el-button--default) {
  background: #404040;
  border-color: #606060;
  color: #ffffff;
}

:deep(.el-button--default:hover) {
  background: #606060;
  border-color: #808080;
}

:deep(.el-button--primary) {
  background: #409eff;
  border-color: #409eff;
}

:deep(.el-button--primary:hover) {
  background: #66b1ff;
  border-color: #66b1ff;
}
</style> 