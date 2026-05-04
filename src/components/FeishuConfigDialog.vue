<template>
  <el-dialog
    v-model="visible"
    title="飞书机器人配置"
    width="640px"
    :close-on-click-modal="false"
    @open="handleOpen"
    @close="handleClose"
    class="feishu-dialog"
  >
    <div class="feishu-body">
      <!-- 头部说明 -->
      <div class="hint-block">
        <el-icon class="hint-icon"><InfoFilled /></el-icon>
        <div class="hint-text">
          填入飞书自建应用的 <code>App ID</code> / <code>App Secret</code>，点击"测试连接"会调
          <code>bot/v3/info</code>，成功表示凭据可用且机器人能力已启用。M2 阶段不会自动建立长连接。
        </div>
      </div>

      <!-- 表单 -->
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        class="feishu-form"
      >
        <el-form-item label="App ID" prop="appId">
          <el-input
            v-model="form.appId"
            placeholder="如：cli_a1b2c3d4e5f6g7h8"
            clearable
          >
            <template #prefix>
              <el-icon><User /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item label="App Secret" prop="appSecret">
          <el-input
            v-model="form.appSecret"
            type="password"
            placeholder="飞书开发者后台「凭证与基础信息」中获取"
            show-password
            clearable
          >
            <template #prefix>
              <el-icon><Lock /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="域名" prop="domain">
              <el-select v-model="form.domain" style="width: 100%">
                <el-option label="feishu (open.feishu.cn)" value="feishu" />
                <el-option label="lark (open.larksuite.com)" value="lark" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="启用" prop="enabled">
              <div class="enabled-row">
                <el-switch
                  v-model="form.enabled"
                  active-color="#6366f1"
                  inactive-color="rgba(255,255,255,0.12)"
                />
                <span class="enabled-hint">M2 仅记录开关，M3 起将据此自动建立连接</span>
              </div>
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="绑定 LLM 配置（可选）">
          <el-select
            v-model="form.bindLlmConfigId"
            clearable
            placeholder="未绑定时使用全局活跃 LLM 配置"
            style="width: 100%"
          >
            <el-option
              v-for="cfg in llmConfigs"
              :key="cfg.id"
              :label="`${cfg.config_name || cfg.provider} · ${cfg.model}`"
              :value="cfg.id"
            />
          </el-select>
        </el-form-item>
      </el-form>

      <!-- 测试结果 -->
      <div v-if="testResult" class="test-result" :class="testResult.ok ? 'success' : 'error'">
        <el-icon>
          <CircleCheckFilled v-if="testResult.ok" />
          <CircleCloseFilled v-else />
        </el-icon>
        <div class="test-result-text">
          <div class="test-result-title">{{ testResult.title }}</div>
          <div v-if="testResult.detail" class="test-result-detail">{{ testResult.detail }}</div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">关闭</el-button>
        <el-button @click="handleTest" :loading="testing" :disabled="!canTest">
          <el-icon v-if="!testing"><Checked /></el-icon>
          测试连接
        </el-button>
        <el-button type="primary" @click="handleSave" :loading="saving">
          保存配置
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  InfoFilled,
  User,
  Lock,
  Checked,
  CircleCheckFilled,
  CircleCloseFilled,
} from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'

// ── 类型定义 ──────────────────────────────────────────────────────────

interface FeishuConfig {
  id: number
  name: string
  app_id: string
  app_secret: string
  domain: string
  bind_llm_config_id: number | null
  bind_role_scope: string
  enabled: boolean
  created_at: string
  updated_at: string
}

interface BotIdentity {
  open_id: string
  name: string
}

interface LLMConfigSummary {
  id: number
  config_name: string
  provider: string
  model: string
  is_active: boolean
}

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
const llmConfigs = ref<LLMConfigSummary[]>([])
const testResult = ref<{ ok: boolean; title: string; detail?: string } | null>(null)

const form = reactive({
  appId: '',
  appSecret: '',
  domain: 'feishu',
  bindLlmConfigId: null as number | null,
  bindRoleScope: 'analysis',
  enabled: false,
})

// ── 计算属性 ──────────────────────────────────────────────────────────

const canTest = computed(() =>
  form.appId.trim().length > 0 && form.appSecret.trim().length > 0,
)

// ── 表单验证规则 ──────────────────────────────────────────────────────

const rules: FormRules = {
  appId: [{ required: true, message: '请填写 App ID', trigger: 'blur' }],
  appSecret: [{ required: true, message: '请填写 App Secret', trigger: 'blur' }],
  domain: [{ required: true, message: '请选择域名', trigger: 'change' }],
}

// ── 监听 ──────────────────────────────────────────────────────────────

watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

// ── 业务逻辑 ──────────────────────────────────────────────────────────

async function handleOpen() {
  await Promise.all([loadConfig(), loadLlmConfigs()])
}

async function loadConfig() {
  try {
    const cfg = await invoke<FeishuConfig | null>('feishu_get_config')
    if (cfg) {
      form.appId = cfg.app_id
      form.appSecret = cfg.app_secret
      form.domain = cfg.domain || 'feishu'
      form.bindLlmConfigId = cfg.bind_llm_config_id
      form.bindRoleScope = cfg.bind_role_scope || 'analysis'
      form.enabled = cfg.enabled
    } else {
      form.appId = ''
      form.appSecret = ''
      form.domain = 'feishu'
      form.bindLlmConfigId = null
      form.bindRoleScope = 'analysis'
      form.enabled = false
    }
    testResult.value = null
    formRef.value?.clearValidate()
  } catch (e) {
    ElMessage.error(`加载飞书配置失败: ${e}`)
  }
}

async function loadLlmConfigs() {
  try {
    llmConfigs.value = await invoke<LLMConfigSummary[]>('get_llm_configs')
  } catch (e) {
    // 容错：LLM 配置不可用时只是禁用绑定下拉，不影响保存飞书凭据
    console.error('加载 LLM 配置失败', e)
    llmConfigs.value = []
  }
}

async function handleTest() {
  if (!canTest.value) return
  testing.value = true
  testResult.value = null
  try {
    const id = await invoke<BotIdentity>('feishu_test_credential', {
      appId: form.appId,
      appSecret: form.appSecret,
      domain: form.domain,
    })
    testResult.value = {
      ok: true,
      title: `连接成功：${id.name || '(未命名机器人)'}`,
      detail: `open_id: ${id.open_id || '(空)'}`,
    }
  } catch (e) {
    testResult.value = {
      ok: false,
      title: '连接失败',
      detail: String(e),
    }
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
    await invoke('feishu_save_config', {
      config: {
        app_id: form.appId,
        app_secret: form.appSecret,
        domain: form.domain,
        bind_llm_config_id: form.bindLlmConfigId,
        bind_role_scope: form.bindRoleScope,
        enabled: form.enabled,
      },
    })
    ElMessage.success('飞书配置已保存')
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
.feishu-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.hint-block {
  display: flex;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 10px;
  background: rgba(99, 102, 241, 0.08);
  border: 1px solid rgba(99, 102, 241, 0.25);
}

.hint-icon {
  color: #a5b4fc;
  font-size: 18px;
  flex-shrink: 0;
  margin-top: 2px;
}

.hint-text {
  font-size: 12px;
  line-height: 1.6;
  color: #cbd5e1;
}

.hint-text code {
  font-family: 'Consolas', 'Courier New', monospace;
  background: rgba(99, 102, 241, 0.15);
  border-radius: 3px;
  padding: 1px 5px;
  color: #a5b4fc;
  font-size: 11px;
}

.feishu-form { flex-shrink: 0; }

:deep(.feishu-form .el-form-item) {
  margin-bottom: 14px;
}

:deep(.feishu-form .el-form-item__label) {
  font-size: 12px !important;
  font-weight: 600;
  color: #64748b !important;
  letter-spacing: 0.02em;
  padding-bottom: 4px !important;
  line-height: 1 !important;
  height: auto !important;
}

.enabled-row {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 32px;
}

.enabled-hint {
  font-size: 11px;
  color: #475569;
  line-height: 1.4;
}

/* 测试结果 */
.test-result {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  flex-shrink: 0;
}

.test-result .el-icon {
  font-size: 18px;
  margin-top: 1px;
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

.test-result-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-width: 0;
}

.test-result-title {
  font-weight: 600;
}

.test-result-detail {
  font-size: 11px;
  font-weight: 400;
  font-family: 'Consolas', 'Courier New', monospace;
  word-break: break-all;
  opacity: 0.85;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
