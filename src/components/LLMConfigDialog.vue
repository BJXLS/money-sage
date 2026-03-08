<template>
  <el-dialog
    v-model="visible"
    title="大模型接口配置"
    width="760px"
    :close-on-click-modal="false"
    @open="handleOpen"
    @close="handleClose"
    class="llm-dialog"
  >
    <div class="dialog-body">
      <!-- 左侧：已保存配置列表 -->
      <div class="config-list-panel">
        <div class="panel-title">已保存配置</div>

        <div v-if="configs.length === 0" class="empty-configs">
          <div class="empty-icon">🤖</div>
          <div class="empty-text">还没有配置</div>
          <div class="empty-sub">点击右侧"新建"开始</div>
        </div>

        <div class="config-items" v-else>
          <div
            v-for="cfg in configs"
            :key="cfg.id"
            class="config-item"
            :class="{ 'is-selected': selectedId === cfg.id, 'is-active': cfg.is_active }"
            @click="selectConfig(cfg)"
          >
            <div class="config-item-header">
              <span class="config-provider-badge" :style="{ background: getProviderColor(cfg.provider) }">
                {{ getProviderShort(cfg.provider) }}
              </span>
              <span class="config-item-name">{{ cfg.config_name || cfg.provider }}</span>
              <span v-if="cfg.is_active" class="active-badge">活跃</span>
            </div>
            <div class="config-item-model">{{ cfg.model || '未设置模型' }}</div>
            <div class="config-item-actions">
              <el-tooltip content="设为活跃" placement="top">
                <el-button
                  v-if="!cfg.is_active"
                  size="small"
                  circle
                  class="action-btn activate-btn"
                  @click.stop="handleActivate(Number(cfg.id))"
                >
                  <el-icon><Select /></el-icon>
                </el-button>
              </el-tooltip>
              <el-tooltip content="删除" placement="top">
                <el-button
                  size="small"
                  circle
                  class="action-btn delete-btn"
                  @click.stop="handleDelete(Number(cfg.id))"
                >
                  <el-icon><Delete /></el-icon>
                </el-button>
              </el-tooltip>
            </div>
          </div>
        </div>

        <el-button class="new-config-btn" @click="startNewConfig">
          <el-icon><Plus /></el-icon>
          新建配置
        </el-button>
      </div>

      <!-- 右侧：编辑表单 -->
      <div class="config-form-panel">
        <!-- 预设供应商快捷选择 -->
        <div class="preset-section">
          <div class="preset-title">快速选择供应商</div>
          <div class="preset-grid">
            <div
              v-for="preset in presets"
              :key="preset.provider"
              class="preset-card"
              :class="{ 'is-selected': form.provider === preset.provider }"
              @click="applyPreset(preset)"
            >
              <span class="preset-icon">{{ preset.icon }}</span>
              <span class="preset-name">{{ preset.label }}</span>
            </div>
          </div>
        </div>

        <!-- 表单 -->
        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          label-position="top"
          class="config-form"
        >
          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item label="配置名称" prop="configName">
                <el-input v-model="form.configName" placeholder="如：我的 GPT-4" clearable />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="供应商" prop="provider">
                <el-input v-model="form.provider" placeholder="如：OpenAI" clearable />
              </el-form-item>
            </el-col>
          </el-row>

          <el-form-item label="API Base URL" prop="baseUrl">
            <el-input
              v-model="form.baseUrl"
              placeholder="如：https://api.openai.com/v1"
              clearable
            >
              <template #prefix>
                <el-icon><Promotion /></el-icon>
              </template>
            </el-input>
          </el-form-item>

          <el-form-item label="API Key" prop="apiKey">
            <el-input
              v-model="form.apiKey"
              type="password"
              placeholder="本地服务（Ollama/LM Studio）可留空"
              show-password
              clearable
            >
              <template #prefix>
                <el-icon><Lock /></el-icon>
              </template>
            </el-input>
          </el-form-item>

          <el-form-item label="模型名称" prop="model">
            <el-input v-model="form.model" :placeholder="currentModelPlaceholder" clearable>
              <template #prefix>
                <el-icon><Monitor /></el-icon>
              </template>
            </el-input>
            <div class="field-hint" v-if="currentPreset?.models?.length">
              推荐：
              <span
                v-for="m in currentPreset.models"
                :key="m"
                class="model-tag"
                @click="form.model = m"
              >{{ m }}</span>
            </div>
          </el-form-item>

          <!-- 高级设置 -->
          <div class="advanced-toggle" @click="showAdvanced = !showAdvanced">
            <el-icon :class="{ 'is-rotate': showAdvanced }"><ArrowRight /></el-icon>
            高级设置
          </div>

          <div v-show="showAdvanced" class="advanced-fields">
            <el-form-item label="温度 (Temperature)">
              <el-input-number
                v-model="form.temperature"
                :min="0"
                :max="2"
                :step="0.1"
                :precision="1"
                controls-position="right"
                style="width: 100%"
              />
            </el-form-item>
            <el-form-item label="最大 Token">
              <el-input-number
                v-model="form.maxTokens"
                :min="256"
                :max="128000"
                :step="256"
                controls-position="right"
                style="width: 100%"
              />
            </el-form-item>

            <!-- 深度思考开关：仅阿里百炼供应商显示 -->
            <div v-if="form.provider === '阿里百炼'" class="thinking-row">
              <div class="thinking-label">
                <span class="thinking-title">深度思考 (Thinking)</span>
                <span class="thinking-desc">开启后请求携带 <code>enable_thinking: true</code>，仅支持 QwQ / Qwen3 等思考模型</span>
              </div>
              <el-switch
                v-model="form.enableThinking"
                active-color="#6366f1"
                inactive-color="rgba(255,255,255,0.12)"
              />
            </div>
          </div>
        </el-form>

        <!-- 测试结果 -->
        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'success' : 'error'">
          <el-icon>
            <CircleCheckFilled v-if="testResult.ok" />
            <CircleCloseFilled v-else />
          </el-icon>
          <span>{{ testResult.message }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">取消</el-button>
        <el-button @click="handleTest" :loading="testing" :disabled="!canTest">
          <el-icon v-if="!testing"><Checked /></el-icon>
          测试连接
        </el-button>
        <el-button type="primary" @click="handleSave" :loading="saving">
          {{ editingId ? '更新配置' : '保存配置' }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'

// ── 类型定义 ──────────────────────────────────────────────────────────

interface LLMConfig {
  id?: number
  config_name: string
  provider: string
  base_url: string
  api_key: string
  model: string
  temperature: number
  max_tokens: number
  enable_thinking: boolean
  is_active: boolean
  created_at?: string
  updated_at?: string
}

interface PresetProvider {
  provider: string
  label: string
  icon: string
  base_url: string
  models: string[]
  no_key?: boolean
  color: string
}

// ── 预设供应商列表 ────────────────────────────────────────────────────

const presets: PresetProvider[] = [
  {
    provider: 'OpenAI',
    label: 'OpenAI',
    icon: '🤖',
    base_url: 'https://api.openai.com/v1',
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    color: '#10a37f',
  },
  {
    provider: 'DeepSeek',
    label: 'DeepSeek',
    icon: '🌊',
    base_url: 'https://api.deepseek.com/v1',
    models: ['deepseek-chat', 'deepseek-reasoner'],
    color: '#4d6bfe',
  },
  {
    provider: '阿里百炼',
    label: '阿里百炼',
    icon: '☁️',
    base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    models: ['qwen-plus', 'qwen-turbo', 'qwen-max', 'qwen3-235b-a22b'],
    color: '#ff6a00',
  },
  {
    provider: 'Moonshot',
    label: 'Moonshot',
    icon: '🌙',
    base_url: 'https://api.moonshot.cn/v1',
    models: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k'],
    color: '#7c3aed',
  },
  {
    provider: '智谱AI',
    label: '智谱 AI',
    icon: '🧠',
    base_url: 'https://open.bigmodel.cn/api/paas/v4',
    models: ['glm-4-flash', 'glm-4', 'glm-4v'],
    color: '#0ea5e9',
  },
  {
    provider: 'Groq',
    label: 'Groq',
    icon: '⚡',
    base_url: 'https://api.groq.com/openai/v1',
    models: ['llama-3.3-70b-versatile', 'mixtral-8x7b-32768', 'gemma2-9b-it'],
    color: '#f59e0b',
  },
  {
    provider: 'Ollama',
    label: 'Ollama',
    icon: '🦙',
    base_url: 'http://localhost:11434/v1',
    models: ['qwen2.5', 'llama3.2', 'deepseek-r1', 'gemma3'],
    no_key: true,
    color: '#059669',
  },
  {
    provider: '自定义',
    label: '自定义',
    icon: '⚙️',
    base_url: '',
    models: [],
    color: '#6366f1',
  },
]

// ── Props & Emits ─────────────────────────────────────────────────────

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'success': []
}>()

// ── 响应式状态 ────────────────────────────────────────────────────────

const visible = ref(false)
const formRef = ref<FormInstance>()
const saving = ref(false)
const testing = ref(false)
const showAdvanced = ref(false)
const configs = ref<LLMConfig[]>([])
const selectedId = ref<number | null>(null)
const editingId = ref<number | null>(null)
const testResult = ref<{ ok: boolean; message: string } | null>(null)

const form = reactive({
  configName: '',
  provider: '',
  baseUrl: '',
  apiKey: '',
  model: '',
  temperature: 0.3,
  maxTokens: 2048,
  enableThinking: false,
})

// ── 计算属性 ──────────────────────────────────────────────────────────

const currentPreset = computed(() =>
  presets.find(p => p.provider === form.provider) ?? null
)

const currentModelPlaceholder = computed(() => {
  if (currentPreset.value?.models.length) {
    return `如：${currentPreset.value.models[0]}`
  }
  return '输入模型名称'
})

const canTest = computed(() => !!form.baseUrl && !!form.model)

// ── 表单验证规则 ──────────────────────────────────────────────────────

const rules: FormRules = {
  baseUrl: [{ required: true, message: '请填写 API Base URL', trigger: 'blur' }],
  model:   [{ required: true, message: '请填写模型名称', trigger: 'blur' }],
}

// ── 监听 ──────────────────────────────────────────────────────────────

watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

// ── 工具函数 ──────────────────────────────────────────────────────────

function getProviderColor(provider: string): string {
  return presets.find(p => p.provider === provider)?.color ?? '#6366f1'
}

function getProviderShort(provider: string): string {
  if (!provider) return '?'
  // 取首字母或前两个汉字
  return provider.replace(/[^a-zA-Z\u4e00-\u9fa5]/g, '').slice(0, 2) || provider[0]
}

// ── 业务逻辑 ──────────────────────────────────────────────────────────

async function handleOpen() {
  await loadConfigs()
  // 默认选中活跃配置
  const active = configs.value.find(c => c.is_active)
  if (active) selectConfig(active)
  else startNewConfig()
}

async function loadConfigs() {
  try {
    configs.value = await invoke<LLMConfig[]>('get_llm_configs')
  } catch (e) {
    ElMessage.error(`加载配置失败: ${e}`)
  }
}

function selectConfig(cfg: LLMConfig) {
  selectedId.value = cfg.id ?? null
  editingId.value = cfg.id ?? null
  form.configName  = cfg.config_name
  form.provider    = cfg.provider
  form.baseUrl     = cfg.base_url
  form.apiKey      = cfg.api_key
  form.model       = cfg.model
  form.temperature    = cfg.temperature    ?? 0.3
  form.maxTokens      = cfg.max_tokens     ?? 2048
  form.enableThinking = cfg.enable_thinking ?? false
  testResult.value = null
}

function startNewConfig() {
  selectedId.value = null
  editingId.value  = null
  form.configName  = ''
  form.provider    = ''
  form.baseUrl     = ''
  form.apiKey      = ''
  form.model       = ''
  form.temperature    = 0.3
  form.maxTokens      = 2048
  form.enableThinking = false
  testResult.value = null
  formRef.value?.clearValidate()
}

function applyPreset(preset: PresetProvider) {
  form.provider = preset.provider
  form.baseUrl  = preset.base_url
  if (preset.no_key) form.apiKey = 'ollama'   // Ollama 需要非空值
  if (preset.models.length) form.model = preset.models[0]
  if (!form.configName) form.configName = preset.label
  testResult.value = null
}

async function handleActivate(id: number) {
  try {
    await invoke('set_active_llm_config', { id })
    ElMessage.success('已切换为活跃配置')
    await loadConfigs()
  } catch (e) {
    ElMessage.error(`切换失败: ${e}`)
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm('确定删除这个配置吗？', '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await invoke('delete_llm_config', { id })
    ElMessage.success('已删除')
    await loadConfigs()
    if (editingId.value === id) startNewConfig()
  } catch (e) {
    if (e !== 'cancel') ElMessage.error(`删除失败: ${e}`)
  }
}

async function handleTest() {
  if (!form.baseUrl || !form.model) return
  testing.value  = true
  testResult.value = null
  try {
    const msg = await invoke<string>('test_llm_connection', {
      config: {
        base_url: form.baseUrl,
        api_key:  form.apiKey,
        model:    form.model,
      },
    })
    testResult.value = { ok: true, message: msg }
  } catch (e) {
    testResult.value = { ok: false, message: String(e) }
  } finally {
    testing.value = false
  }
}

async function handleSave() {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }

  saving.value = true
  try {
    const payload = {
      config_name:     form.configName || form.provider || '未命名配置',
      provider:        form.provider,
      base_url:        form.baseUrl,
      api_key:         form.apiKey,
      model:           form.model,
      temperature:     form.temperature,
      max_tokens:      form.maxTokens,
      enable_thinking: form.enableThinking,
    }

    if (editingId.value) {
      await invoke('update_llm_config', {
        id: editingId.value,
        config: { ...payload, is_active: configs.value.find(c => c.id === editingId.value)?.is_active },
      })
      ElMessage.success('配置已更新')
    } else {
      await invoke('save_llm_config', { config: payload })
      ElMessage.success('配置已保存并设为活跃')
    }

    await loadConfigs()
    emit('success')
  } catch (e) {
    ElMessage.error(`保存失败: ${e}`)
  } finally {
    saving.value = false
  }
}

function handleClose() {
  visible.value = false
}
</script>

<style scoped>
.dialog-body {
  display: flex;
  gap: 0;
  height: 520px;
  overflow: hidden;
}

/* ── 左侧配置列表 ── */
.config-list-panel {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-right: 16px;
  overflow: hidden;
}

.panel-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #475569;
  padding-bottom: 4px;
}

.empty-configs {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: #475569;
}

.empty-icon { font-size: 28px; }
.empty-text { font-size: 13px; font-weight: 500; }
.empty-sub  { font-size: 11px; }

.config-items {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.config-item {
  padding: 10px 10px 8px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
}

.config-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.12);
}

.config-item.is-selected {
  background: rgba(99, 102, 241, 0.12);
  border-color: rgba(99, 102, 241, 0.35);
}

.config-item.is-active {
  border-color: rgba(16, 185, 129, 0.3);
}

.config-item-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.config-provider-badge {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.config-item-name {
  font-size: 13px;
  font-weight: 600;
  color: #cbd5e1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.active-badge {
  font-size: 9px;
  font-weight: 700;
  color: #10b981;
  background: rgba(16, 185, 129, 0.15);
  border: 1px solid rgba(16, 185, 129, 0.3);
  border-radius: 4px;
  padding: 1px 5px;
  flex-shrink: 0;
}

.config-item-model {
  font-size: 11px;
  color: #475569;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.config-item-actions {
  display: flex;
  gap: 4px;
  margin-top: 8px;
}

.action-btn {
  width: 24px !important;
  height: 24px !important;
  padding: 0 !important;
  border: 1px solid rgba(255, 255, 255, 0.08) !important;
  background: rgba(255, 255, 255, 0.04) !important;
}

.activate-btn:hover {
  background: rgba(16, 185, 129, 0.15) !important;
  border-color: rgba(16, 185, 129, 0.4) !important;
  color: #10b981 !important;
}

.delete-btn:hover {
  background: rgba(239, 68, 68, 0.15) !important;
  border-color: rgba(239, 68, 68, 0.4) !important;
  color: #ef4444 !important;
}

.new-config-btn {
  background: rgba(99, 102, 241, 0.1) !important;
  border: 1px dashed rgba(99, 102, 241, 0.4) !important;
  color: #a5b4fc !important;
  width: 100%;
  border-radius: 10px !important;
  font-size: 13px !important;
  font-weight: 500 !important;
  height: 36px !important;
  transition: all 0.2s !important;
}

.new-config-btn:hover {
  background: rgba(99, 102, 241, 0.18) !important;
  border-color: rgba(99, 102, 241, 0.6) !important;
}

/* ── 右侧编辑表单 ── */
.config-form-panel {
  flex: 1;
  padding-left: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.preset-section { flex-shrink: 0; }

.preset-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #475569;
  margin-bottom: 8px;
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
}

.preset-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: all 0.2s;
}

.preset-card:hover {
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}

.preset-card.is-selected {
  background: rgba(99, 102, 241, 0.12);
  border-color: rgba(99, 102, 241, 0.4);
}

.preset-icon { font-size: 18px; line-height: 1; }

.preset-name {
  font-size: 11px;
  font-weight: 500;
  color: #94a3b8;
  text-align: center;
  line-height: 1.2;
}

.preset-card.is-selected .preset-name {
  color: #a5b4fc;
}

/* 表单 */
.config-form { flex-shrink: 0; }

:deep(.config-form .el-form-item) {
  margin-bottom: 14px;
}

:deep(.config-form .el-form-item__label) {
  font-size: 12px !important;
  font-weight: 600;
  color: #64748b !important;
  letter-spacing: 0.02em;
  padding-bottom: 4px !important;
  line-height: 1 !important;
  height: auto !important;
}

.field-hint {
  font-size: 11px;
  color: #475569;
  margin-top: 5px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.model-tag {
  display: inline-block;
  padding: 1px 7px;
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.25);
  border-radius: 5px;
  color: #a5b4fc;
  cursor: pointer;
  font-size: 11px;
  transition: all 0.15s;
}

.model-tag:hover {
  background: rgba(99, 102, 241, 0.2);
  border-color: rgba(99, 102, 241, 0.5);
}

/* 高级设置 */
.advanced-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: #475569;
  cursor: pointer;
  user-select: none;
  transition: color 0.2s;
  margin-bottom: 10px;
}

.advanced-toggle:hover { color: #94a3b8; }

.advanced-toggle .el-icon {
  transition: transform 0.25s;
}

.advanced-toggle .el-icon.is-rotate {
  transform: rotate(90deg);
}

.advanced-fields {
  padding: 12px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  margin-bottom: 4px;
}


/* 测试结果 */
.test-result {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  flex-shrink: 0;
}

.test-result.success {
  background: rgba(16, 185, 129, 0.1);
  border: 1px solid rgba(16, 185, 129, 0.25);
  color: #34d399;
}

.test-result.error {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: #f87171;
}

/* 底部按钮 */
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

/* 深度思考开关行 */
.thinking-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  background: rgba(99, 102, 241, 0.06);
  border: 1px solid rgba(99, 102, 241, 0.2);
  border-radius: 10px;
  margin-top: 4px;
}

.thinking-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.thinking-title {
  font-size: 13px;
  font-weight: 600;
  color: #c7d2fe;
}

.thinking-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.5;
}

.thinking-desc code {
  font-family: 'Consolas', 'Courier New', monospace;
  background: rgba(99, 102, 241, 0.15);
  border-radius: 3px;
  padding: 1px 4px;
  color: #a5b4fc;
  font-size: 10px;
}

/* 滚动条 */
.config-items::-webkit-scrollbar,
.config-form-panel::-webkit-scrollbar { width: 4px; }
.config-items::-webkit-scrollbar-thumb,
.config-form-panel::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}
</style>
