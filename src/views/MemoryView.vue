<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores'
import { ElMessage } from 'element-plus'

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
  await store.fetchWorkspaceFiles()
  await loadFile(activeFile.value)
})
</script>

<template>
  <div class="workspace-editor">
    <div class="file-sidebar">
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
    </div>

    <div class="editor-area">
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
    </div>
  </div>
</template>

<style scoped>
.workspace-editor {
  display: flex;
  height: calc(100vh - 112px);
  gap: 16px;
}

.file-sidebar {
  width: 220px;
  flex-shrink: 0;
  background: #151520;
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 14px;
  padding: 16px 0;
  overflow-y: auto;
}

.sidebar-header {
  padding: 0 16px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  margin-bottom: 8px;
}

.file-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  margin: 2px 10px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  color: #94a3b8;
  font-size: 14px;
}

.file-item:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #cbd5e1;
}

.file-item.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(139, 92, 246, 0.15));
  color: #a5b4fc;
  border: 1px solid rgba(99, 102, 241, 0.2);
}

.file-item.missing {
  opacity: 0.6;
}

.file-name {
  font-weight: 500;
}

.file-meta {
  font-size: 11px;
  color: #64748b;
  margin-left: auto;
  margin-right: 8px;
}

.file-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.editor-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #151520;
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 14px;
  overflow: hidden;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.file-title {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.unsaved-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.toolbar-right {
  display: flex;
  gap: 8px;
}

.editor-body {
  flex: 1;
  padding: 16px 20px;
  overflow: hidden;
}

.editor-textarea :deep(.el-textarea__inner) {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #e2e8f0;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 13px;
  line-height: 1.7;
  border-radius: 10px;
  padding: 16px;
  height: 100%;
}

.editor-textarea :deep(.el-textarea__inner:focus) {
  border-color: rgba(99, 102, 241, 0.5);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1);
}

.editor-textarea :deep(.el-textarea__inner::placeholder) {
  color: #475569;
}

.editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px;
  color: #64748b;
}

.footer-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.warning {
  color: #f59e0b;
  font-weight: 600;
}

.limit-warning {
  color: #f59e0b;
}
</style>
