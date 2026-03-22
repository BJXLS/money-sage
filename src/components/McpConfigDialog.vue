<template>
  <el-dialog
    v-model="visible"
    title="MCP 工具服务器"
    width="780px"
    :close-on-click-modal="false"
    @open="handleOpen"
    @close="handleClose"
    class="mcp-dialog"
  >
    <div class="dialog-body">
      <!-- 左侧：服务器列表 -->
      <div class="server-list-panel">
        <div class="panel-title">已配置服务器</div>

        <div v-if="servers.length === 0" class="empty-servers">
          <div class="empty-icon">🔌</div>
          <div class="empty-text">还没有服务器</div>
          <div class="empty-sub">添加 MCP 服务器来扩展 AI 能力</div>
        </div>

        <div class="server-items" v-else>
          <div
            v-for="srv in servers"
            :key="srv.id"
            class="server-item"
            :class="{ 'is-selected': selectedId === srv.id }"
            @click="selectServer(srv)"
          >
            <div class="server-item-header">
              <span class="server-status-dot" :class="getStatusClass(srv)" />
              <span class="server-item-name">{{ srv.name || '未命名' }}</span>
            </div>
            <div class="server-item-cmd">{{ srv.command }}</div>
            <div class="server-item-tools" v-if="getServerTools(srv.id).length > 0">
              {{ getServerTools(srv.id).length }} 个工具可用
            </div>
            <div class="server-item-actions">
              <el-tooltip :content="isConnected(srv.id) ? '停止' : '启动'" placement="top">
                <el-button
                  size="small"
                  circle
                  class="action-btn"
                  :class="isConnected(srv.id) ? 'stop-btn' : 'start-btn'"
                  @click.stop="toggleServer(srv)"
                  :loading="serverLoading[srv.id]"
                >
                  <el-icon v-if="!serverLoading[srv.id]">
                    <VideoPause v-if="isConnected(srv.id)" />
                    <VideoPlay v-else />
                  </el-icon>
                </el-button>
              </el-tooltip>
              <el-tooltip content="删除" placement="top">
                <el-button
                  size="small"
                  circle
                  class="action-btn delete-btn"
                  @click.stop="handleDelete(srv.id)"
                >
                  <el-icon><Delete /></el-icon>
                </el-button>
              </el-tooltip>
            </div>
          </div>
        </div>

        <el-button class="new-server-btn" @click="startNewServer">
          <el-icon><Plus /></el-icon>
          添加服务器
        </el-button>
      </div>

      <!-- 右侧：编辑/详情 -->
      <div class="server-detail-panel">
        <!-- 预设 MCP 服务器 -->
        <div class="preset-section">
          <div class="preset-title">常用 MCP 服务器</div>
          <div class="preset-grid">
            <div
              v-for="preset in presets"
              :key="preset.name"
              class="preset-card"
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
          class="server-form"
        >
          <el-form-item label="服务器名称" prop="name">
            <el-input v-model="form.name" placeholder="如：文件系统" clearable />
          </el-form-item>

          <el-form-item label="启动命令" prop="command">
            <el-input v-model="form.command" placeholder="如：npx, uvx, node" clearable>
              <template #prefix>
                <el-icon><Monitor /></el-icon>
              </template>
            </el-input>
          </el-form-item>

          <el-form-item label="命令参数 (JSON 数组)">
            <el-input
              v-model="form.args"
              type="textarea"
              :rows="2"
              placeholder='如：["-y", "@modelcontextprotocol/server-filesystem", "/path"]'
            />
          </el-form-item>

          <el-form-item label="环境变量 (JSON 对象)">
            <el-input
              v-model="form.env"
              type="textarea"
              :rows="2"
              placeholder='如：{"API_KEY": "your-key"}'
            />
          </el-form-item>

          <el-form-item>
            <el-switch
              v-model="form.enabled"
              active-text="启用"
              inactive-text="禁用"
              active-color="#6366f1"
            />
          </el-form-item>
        </el-form>

        <!-- 工具列表 -->
        <div v-if="selectedServerTools.length > 0" class="tools-section">
          <div class="tools-title">已发现的工具 ({{ selectedServerTools.length }})</div>
          <div class="tools-list">
            <div v-for="tool in selectedServerTools" :key="tool.name" class="tool-item">
              <div class="tool-name">{{ tool.name }}</div>
              <div class="tool-desc" v-if="tool.description">{{ tool.description }}</div>
            </div>
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-if="selectedServerError" class="server-error">
          <el-icon><CircleCloseFilled /></el-icon>
          <span>{{ selectedServerError }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">关闭</el-button>
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

interface McpServerConfig {
  id: number
  name: string
  command: string
  args: string
  env: string
  enabled: boolean
}

interface McpServerStatus {
  id: number
  name: string
  command: string
  enabled: boolean
  connected: boolean
  tools: McpTool[]
  error?: string
}

interface McpTool {
  name: string
  description?: string
  inputSchema: any
}

interface PresetServer {
  name: string
  label: string
  icon: string
  command: string
  args: string
  env: string
}

const presets: PresetServer[] = [
  {
    name: 'filesystem',
    label: '文件系统',
    icon: '📁',
    command: 'npx',
    args: '["-y", "@modelcontextprotocol/server-filesystem", "."]',
    env: '{}',
  },
  {
    name: 'fetch',
    label: '网页抓取',
    icon: '🌐',
    command: 'uvx',
    args: '["mcp-server-fetch"]',
    env: '{}',
  },
  {
    name: 'sqlite',
    label: 'SQLite',
    icon: '🗄️',
    command: 'uvx',
    args: '["mcp-server-sqlite", "--db-path", "data.db"]',
    env: '{}',
  },
  {
    name: 'memory',
    label: '知识图谱',
    icon: '🧠',
    command: 'npx',
    args: '["-y", "@modelcontextprotocol/server-memory"]',
    env: '{}',
  },
  {
    name: 'github',
    label: 'GitHub',
    icon: '🐙',
    command: 'npx',
    args: '["-y", "@modelcontextprotocol/server-github"]',
    env: '{"GITHUB_PERSONAL_ACCESS_TOKEN": "your-token"}',
  },
  {
    name: 'custom',
    label: '自定义',
    icon: '⚙️',
    command: '',
    args: '[]',
    env: '{}',
  },
]

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'change': []
}>()

const visible = ref(false)
const formRef = ref<FormInstance>()
const saving = ref(false)
const servers = ref<McpServerConfig[]>([])
const serverStatus = ref<McpServerStatus[]>([])
const selectedId = ref<number | null>(null)
const editingId = ref<number | null>(null)
const serverLoading = reactive<Record<number, boolean>>({})

const form = reactive({
  name: '',
  command: '',
  args: '[]',
  env: '{}',
  enabled: true,
})

const rules: FormRules = {
  name: [{ required: true, message: '请填写服务器名称', trigger: 'blur' }],
  command: [{ required: true, message: '请填写启动命令', trigger: 'blur' }],
}

watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const selectedServerTools = computed<McpTool[]>(() => {
  if (!selectedId.value) return []
  return getServerTools(selectedId.value)
})

const selectedServerError = computed<string | null>(() => {
  if (!selectedId.value) return null
  const status = serverStatus.value.find(s => s.id === selectedId.value)
  return status?.error ?? null
})

function getServerTools(serverId: number): McpTool[] {
  const status = serverStatus.value.find(s => s.id === serverId)
  return status?.tools ?? []
}

function isConnected(serverId: number): boolean {
  const status = serverStatus.value.find(s => s.id === serverId)
  return status?.connected ?? false
}

function getStatusClass(srv: McpServerConfig): string {
  const status = serverStatus.value.find(s => s.id === srv.id)
  if (status?.connected) return 'is-connected'
  if (status?.error) return 'is-error'
  return 'is-disconnected'
}

async function handleOpen() {
  await loadServers()
  await refreshStatus()
  if (servers.value.length > 0) {
    selectServer(servers.value[0])
  } else {
    startNewServer()
  }
}

async function loadServers() {
  try {
    servers.value = await invoke<McpServerConfig[]>('get_mcp_servers')
  } catch (e) {
    ElMessage.error(`加载 MCP 配置失败: ${e}`)
  }
}

async function refreshStatus() {
  try {
    serverStatus.value = await invoke<McpServerStatus[]>('get_mcp_server_status')
  } catch (e) {
    console.warn('获取 MCP 状态失败:', e)
  }
}

function selectServer(srv: McpServerConfig) {
  selectedId.value = srv.id
  editingId.value = srv.id
  form.name = srv.name
  form.command = srv.command
  form.args = srv.args || '[]'
  form.env = srv.env || '{}'
  form.enabled = srv.enabled
}

function startNewServer() {
  selectedId.value = null
  editingId.value = null
  form.name = ''
  form.command = ''
  form.args = '[]'
  form.env = '{}'
  form.enabled = true
  formRef.value?.clearValidate()
}

function applyPreset(preset: PresetServer) {
  form.name = preset.name === 'custom' ? '' : preset.label
  form.command = preset.command
  form.args = preset.args
  form.env = preset.env
  form.enabled = true
}

async function toggleServer(srv: McpServerConfig) {
  serverLoading[srv.id] = true
  try {
    if (isConnected(srv.id)) {
      await invoke('stop_mcp_server', { id: srv.id })
      ElMessage.success(`已停止 ${srv.name}`)
    } else {
      await invoke('start_mcp_server', { id: srv.id })
      ElMessage.success(`已启动 ${srv.name}`)
    }
    await refreshStatus()
  } catch (e) {
    ElMessage.error(`操作失败: ${e}`)
  } finally {
    serverLoading[srv.id] = false
  }
}

async function handleDelete(id: number) {
  try {
    await ElMessageBox.confirm('确定删除这个 MCP 服务器配置吗？', '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await invoke('delete_mcp_server', { id })
    ElMessage.success('已删除')
    await loadServers()
    await refreshStatus()
    if (editingId.value === id) startNewServer()
    emit('change')
  } catch (e) {
    if (e !== 'cancel') ElMessage.error(`删除失败: ${e}`)
  }
}

async function handleSave() {
  try {
    await formRef.value?.validate()
  } catch { return }

  saving.value = true
  try {
    const payload = {
      name: form.name,
      command: form.command,
      args: form.args,
      env: form.env,
      enabled: form.enabled,
    }

    if (editingId.value) {
      await invoke('update_mcp_server', { id: editingId.value, config: payload })
      ElMessage.success('配置已更新')
    } else {
      await invoke('create_mcp_server', { config: payload })
      ElMessage.success('配置已保存')
    }

    await loadServers()
    await refreshStatus()
    emit('change')
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

/* ── 左侧服务器列表 ── */
.server-list-panel {
  width: 210px;
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

.empty-servers {
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
.empty-sub  { font-size: 11px; text-align: center; line-height: 1.5; }

.server-items {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.server-item {
  padding: 10px 10px 8px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: all 0.2s;
}

.server-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.12);
}

.server-item.is-selected {
  background: rgba(99, 102, 241, 0.12);
  border-color: rgba(99, 102, 241, 0.35);
}

.server-item-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.server-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.server-status-dot.is-connected {
  background: #10b981;
  box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
}

.server-status-dot.is-disconnected {
  background: #475569;
}

.server-status-dot.is-error {
  background: #ef4444;
  box-shadow: 0 0 6px rgba(239, 68, 68, 0.5);
}

.server-item-name {
  font-size: 13px;
  font-weight: 600;
  color: #cbd5e1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.server-item-cmd {
  font-size: 11px;
  color: #475569;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.server-item-tools {
  font-size: 10px;
  color: #10b981;
  margin-top: 2px;
}

.server-item-actions {
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

.start-btn:hover {
  background: rgba(16, 185, 129, 0.15) !important;
  border-color: rgba(16, 185, 129, 0.4) !important;
  color: #10b981 !important;
}

.stop-btn:hover {
  background: rgba(245, 158, 11, 0.15) !important;
  border-color: rgba(245, 158, 11, 0.4) !important;
  color: #f59e0b !important;
}

.delete-btn:hover {
  background: rgba(239, 68, 68, 0.15) !important;
  border-color: rgba(239, 68, 68, 0.4) !important;
  color: #ef4444 !important;
}

.new-server-btn {
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

.new-server-btn:hover {
  background: rgba(99, 102, 241, 0.18) !important;
  border-color: rgba(99, 102, 241, 0.6) !important;
}

/* ── 右侧详情面板 ── */
.server-detail-panel {
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
  grid-template-columns: repeat(3, 1fr);
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

.preset-icon { font-size: 18px; line-height: 1; }

.preset-name {
  font-size: 11px;
  font-weight: 500;
  color: #94a3b8;
  text-align: center;
}

/* 表单 */
.server-form { flex-shrink: 0; }

:deep(.server-form .el-form-item) {
  margin-bottom: 14px;
}

:deep(.server-form .el-form-item__label) {
  font-size: 12px !important;
  font-weight: 600;
  color: #64748b !important;
  letter-spacing: 0.02em;
  padding-bottom: 4px !important;
  line-height: 1 !important;
  height: auto !important;
}

/* 工具列表 */
.tools-section {
  flex-shrink: 0;
}

.tools-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #10b981;
  margin-bottom: 8px;
}

.tools-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 160px;
  overflow-y: auto;
}

.tool-item {
  padding: 8px 10px;
  background: rgba(16, 185, 129, 0.06);
  border: 1px solid rgba(16, 185, 129, 0.15);
  border-radius: 8px;
}

.tool-name {
  font-size: 12px;
  font-weight: 600;
  color: #a7f3d0;
  font-family: 'Consolas', 'Courier New', monospace;
}

.tool-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
  line-height: 1.4;
}

/* 错误提示 */
.server-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 10px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: #f87171;
  font-size: 13px;
}

/* 底部按钮 */
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

/* 滚动条 */
.server-items::-webkit-scrollbar,
.server-detail-panel::-webkit-scrollbar,
.tools-list::-webkit-scrollbar { width: 4px; }
.server-items::-webkit-scrollbar-thumb,
.server-detail-panel::-webkit-scrollbar-thumb,
.tools-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}
</style>
