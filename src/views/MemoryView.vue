<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores'
import { ElMessage } from 'element-plus'
import { open as openDialog } from '@tauri-apps/plugin-dialog'

const store = useAppStore()
const editingContent = ref('')
const hasUnsavedChanges = ref(false)

const FILE_LABELS: Record<string, string> = {
  'AGENTS.md': '行为准则',
  'IDENTITY.md': '身份信息',
  'SOUL.md': '性格灵魂',
  'USER.md': '用户画像',
  'MEMORY.md': '记忆',
}

// ── Memory 目录配置 ───────────────────────────────────────────────────
const memoryPathInput = ref('')
const changingPath = ref(false)

const loadMemoryDir = async () => {
  memoryPathInput.value = await store.getMemoryDir()
}

const handleBrowse = async () => {
  try {
    const selected = await openDialog({
      directory: true,
      title: '选择记忆文件目录',
    })
    if (selected && typeof selected === 'string') {
      memoryPathInput.value = selected
    }
  } catch (e) {
    ElMessage.error('打开目录选择器失败: ' + String(e))
  }
}

const handleSetMemoryDir = async (mode: string) => {
  const path = memoryPathInput.value.trim()
  if (!path) {
    ElMessage.warning('请输入目录路径')
    return
  }
  const label = mode === 'copy' ? '复制' : '重置'
  const ok = window.confirm(
    `确定要${label}切换记忆目录到 "${path}" 吗？\n\n` +
    (mode === 'copy' ? '当前记忆内容将被复制到新目录。' : '当前记忆内容将不会保留。') +
    '\n\n配置将在重启后生效。'
  )
  if (!ok) return

  changingPath.value = true
  try {
    const result = await store.setMemoryDir(path, mode)
    ElMessage.success(result)
  } catch (e) {
    ElMessage.error('设置失败: ' + String(e))
  } finally {
    changingPath.value = false
  }
}

const activeFile = ref('AGENTS.md')

const loadFile = async (name: string) => {
  await store.readWorkspaceFile(name)
  editingContent.value = store.currentFileContent
  hasUnsavedChanges.value = false
}

const handleFileSelect = (name: string) => {
  if (hasUnsavedChanges.value) {
    const ok = window.confirm('当前文件有未保存的更改，确定要切换吗？')
    if (!ok) return
  }
  activeFile.value = name
  loadFile(name)
}

const handleSave = async () => {
  try {
    await store.writeWorkspaceFile(activeFile.value, editingContent.value)
    hasUnsavedChanges.value = false
    ElMessage.success('保存成功')
  } catch (e) {
    ElMessage.error('保存失败: ' + String(e))
  }
}

const handleReset = async () => {
  const ok = window.confirm('确定要放弃当前更改并重新加载文件吗？')
  if (!ok) return
  await loadFile(activeFile.value)
}

const currentFileInfo = () => {
  return store.workspaceFiles.find(f => f.name === activeFile.value)
}

const charCount = () => {
  return editingContent.value.length
}

const isOverLimit = () => {
  return charCount() > 2000
}

watch(() => editingContent.value, (newVal) => {
  if (newVal !== store.currentFileContent) {
    hasUnsavedChanges.value = true
  }
})

onMounted(async () => {
  await Promise.all([
    store.fetchWorkspaceFiles(),
    loadMemoryDir(),
  ])
  await loadFile(activeFile.value)
})
</script>

<template>
  <div class="workspace-editor">
    <!-- Memory 目录配置 -->
    <div class="memory-path-bar">
      <div class="path-label">记忆目录</div>
      <el-input
        v-model="memoryPathInput"
        placeholder="默认路径"
        size="small"
        class="path-input"
      />
      <el-button size="small" @click="handleBrowse">浏览</el-button>
      <el-button size="small" type="warning" :loading="changingPath" @click="handleSetMemoryDir('reset')">
        重置切换
      </el-button>
      <el-button size="small" type="primary" :loading="changingPath" @click="handleSetMemoryDir('copy')">
        复制切换
      </el-button>
      <span class="path-hint">修改后需重启应用</span>
    </div>

    <aside class="file-sidebar">
      <div class="sidebar-header">配置文件</div>
      <div
        v-for="file in store.workspaceFiles"
        :key="file.name"
        class="file-item"
        :class="{ active: activeFile === file.name, missing: !file.exists }"
        @click="handleFileSelect(file.name)"
      >
        <span class="file-name">{{ FILE_LABELS[file.name] || file.name }}</span>
        <span class="file-meta">{{ file.char_count }}字</span>
        <span v-if="!file.exists" class="file-badge">未创建</span>
      </div>
    </aside>

    <main class="editor-area">
      <div class="editor-toolbar">
        <div class="toolbar-left">
          <span class="file-title">{{ FILE_LABELS[activeFile] || activeFile }}</span>
          <span v-if="hasUnsavedChanges" class="unsaved-badge">未保存</span>
        </div>
        <div class="toolbar-right">
          <el-button size="small" @click="handleReset">重置</el-button>
          <el-button size="small" type="primary" @click="handleSave">保存</el-button>
        </div>
      </div>

      <div class="editor-body">
        <el-input
          v-model="editingContent"
          type="textarea"
          :rows="28"
          resize="none"
          placeholder="在此编辑文件内容..."
          class="editor-textarea"
        />
      </div>

      <div class="editor-footer">
        <div class="footer-left">
          <span :class="{ warning: isOverLimit() }">
            字符数: {{ charCount() }} / 2000
          </span>
          <span v-if="isOverLimit()" class="limit-warning">
            （超过上限，System Prompt 中会被截断）
          </span>
        </div>
        <div class="footer-right">
          <span v-if="currentFileInfo()?.modified_at">
            最后修改: {{ currentFileInfo()?.modified_at }}
          </span>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.workspace-editor {
  display: flex;
  flex-wrap: wrap;
  height: 100%;
  gap: var(--ms-space-4);
}

.memory-path-bar {
  width: 100%;
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-3) var(--ms-space-5);
  flex-shrink: 0;
  box-shadow: var(--ms-shadow-sm);
}

.path-label {
  font-size: var(--ms-text-sm);
  font-weight: 600;
  color: var(--ms-text-primary);
  white-space: nowrap;
}

.path-input { flex: 1; max-width: 500px; }

.path-hint {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-left: auto;
}

.file-sidebar {
  width: 220px;
  flex-shrink: 0;
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  padding: var(--ms-space-4) 0;
  overflow-y: auto;
  box-shadow: var(--ms-shadow-sm);
}

.file-sidebar .sidebar-header {
  padding: 0 var(--ms-space-4) var(--ms-space-3);
  font-size: var(--ms-text-xs);
  font-weight: 600;
  color: var(--ms-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid var(--ms-border-subtle);
  margin-bottom: var(--ms-space-2);
}

.file-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-2) var(--ms-space-4);
  margin: 2px var(--ms-space-3);
  border-radius: var(--ms-radius-md);
  cursor: pointer;
  transition: all 0.2s ease;
  color: var(--ms-text-secondary);
  font-size: var(--ms-text-sm);
  border: 1px solid transparent;
}

.file-item:hover {
  background: var(--ms-surface-hover);
  color: var(--ms-text-primary);
}

.file-item.active {
  background: rgba(99, 102, 241, 0.12);
  color: var(--ms-primary-500);
  border-color: rgba(99, 102, 241, 0.25);
}

.file-item.missing { opacity: 0.6; }

.file-name { font-weight: 500; }

.file-meta {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  margin-left: auto;
  margin-right: var(--ms-space-2);
}

.file-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.12);
  color: var(--ms-warning);
}

.editor-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  overflow: hidden;
  box-shadow: var(--ms-shadow-sm);
  min-width: 0;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-3) var(--ms-space-5);
  border-bottom: 1px solid var(--ms-border-subtle);
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
}

.file-title {
  font-size: var(--ms-text-base);
  font-weight: 600;
  color: var(--ms-text-primary);
}

.unsaved-badge {
  font-size: var(--ms-text-xs);
  padding: 2px 8px;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.12);
  color: var(--ms-warning);
}

.toolbar-right {
  display: flex;
  gap: var(--ms-space-2);
}

.editor-body {
  flex: 1;
  padding: var(--ms-space-4) var(--ms-space-5);
  overflow: hidden;
}

.editor-textarea { height: 100%; }
.editor-textarea :deep(.el-textarea__inner) {
  background: var(--ms-bg-secondary);
  border: 1px solid var(--ms-border-subtle);
  color: var(--ms-text-primary);
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: var(--ms-text-sm);
  line-height: 1.7;
  border-radius: var(--ms-radius-lg);
  padding: var(--ms-space-4);
  height: 100%;
}

.editor-textarea :deep(.el-textarea__inner:focus) {
  border-color: var(--ms-primary-500);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.12);
}

.editor-textarea :deep(.el-textarea__inner::placeholder) {
  color: var(--ms-text-tertiary);
}

.editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-2) var(--ms-space-5);
  border-top: 1px solid var(--ms-border-subtle);
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}

.footer-left {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
}

.warning {
  color: var(--ms-warning);
  font-weight: 600;
}

.limit-warning {
  color: var(--ms-warning);
}
</style>
