<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { marked } from 'marked'

marked.setOptions({ breaks: true, gfm: true })

// ─── 类型定义 ───────────────────────────────────────────────────────────────

interface LLMConfig {
  id: number
  config_name: string
  provider: string
  model: string
  is_active: boolean
}

interface AnalysisSession {
  id: string
  title: string
  config_id: number | null
  created_at: string
  updated_at: string
}

interface AnalysisMessageRecord {
  id: number
  session_id: string
  role: string
  content: string
  created_at: string
}

interface StreamChunkPayload {
  session_id: string
  chunk: string
  done: boolean
  error: string | null
}

interface Message {
  id: number
  role: 'user' | 'assistant' | 'system'
  content: string
  loading?: boolean
  error?: boolean
}

// ─── 常量 ────────────────────────────────────────────────────────────────────

const QUICK_QUESTIONS = [
  '分析我本月的支出结构',
  '过去3个月的收支趋势如何',
  '我的消费有哪些需要注意的地方',
  '哪个分类支出最多？给我具体数据',
]

// ─── 状态 ────────────────────────────────────────────────────────────────────

const sessionId = ref(crypto.randomUUID())
const messages = ref<Message[]>([])
const inputText = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement>()
let msgIdCounter = 0

// 历史会话
const historySessions = ref<AnalysisSession[]>([])
const showHistoryPanel = ref(false)
const activeSessionId = ref<string | null>(null)
const deleteConfirmId = ref<string | null>(null)

// LLM 配置
const llmConfigs = ref<LLMConfig[]>([])
const selectedConfigId = ref<number | null>(null)

// 流式监听
let currentUnlisten: UnlistenFn | null = null

// ─── 加载后端数据 ─────────────────────────────────────────────────────────────

const loadHistory = async () => {
  try {
    historySessions.value = await invoke<AnalysisSession[]>('get_analysis_sessions')
  } catch (_) {
    historySessions.value = []
  }
}

const loadLLMConfigs = async () => {
  try {
    llmConfigs.value = await invoke<LLMConfig[]>('get_llm_configs')
    const active = llmConfigs.value.find(c => c.is_active)
    if (active) selectedConfigId.value = active.id
    else if (llmConfigs.value.length > 0) selectedConfigId.value = llmConfigs.value[0].id
  } catch (_) {
    llmConfigs.value = []
  }
}

// ─── 格式化时间 ───────────────────────────────────────────────────────────────

const formatTime = (ts: string) => {
  try {
    const d = new Date(ts + (ts.includes('Z') || ts.includes('+') ? '' : 'Z'))
    const now = Date.now()
    const diff = now - d.getTime()
    if (diff < 60000) return '刚刚'
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`
    return `${d.getMonth() + 1}/${d.getDate()}`
  } catch {
    return ts
  }
}

// ─── 滚动到底部 ───────────────────────────────────────────────────────────────

const scrollToBottom = async () => {
  await nextTick()
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

// ─── Markdown 渲染 ───────────────────────────────────────────────────────────

const renderMarkdown = (text: string) => {
  if (!text) return ''
  try {
    return marked.parse(text, { async: false }) as string
  } catch {
    return text.replace(/\n/g, '<br/>')
  }
}

// ─── 发送消息（流式） ────────────────────────────────────────────────────────

const sendMessage = async (text?: string) => {
  const content = (text ?? inputText.value).trim()
  if (!content || isLoading.value) return

  inputText.value = ''

  // 添加用户消息
  messages.value.push({ id: ++msgIdCounter, role: 'user', content })

  // 添加 AI loading 占位
  const aiMsgId = ++msgIdCounter
  messages.value.push({ id: aiMsgId, role: 'assistant', content: '', loading: true })
  isLoading.value = true
  await scrollToBottom()

  // 注册流式事件监听
  currentUnlisten = await listen<StreamChunkPayload>('analysis-stream-chunk', (event) => {
    const payload = event.payload
    if (payload.session_id !== sessionId.value) return

    const idx = messages.value.findIndex(m => m.id === aiMsgId)
    if (idx === -1) return

    if (payload.error) {
      messages.value[idx] = {
        ...messages.value[idx],
        content: payload.error,
        error: true,
        loading: false,
      }
      isLoading.value = false
      currentUnlisten?.()
      currentUnlisten = null
      loadHistory()
      return
    }

    if (payload.done) {
      messages.value[idx] = { ...messages.value[idx], loading: false }
      isLoading.value = false
      currentUnlisten?.()
      currentUnlisten = null
      loadHistory()
      return
    }

    // 追加 chunk
    messages.value[idx] = {
      ...messages.value[idx],
      content: messages.value[idx].content + payload.chunk,
      loading: false,
    }
    scrollToBottom()
  })

  // 发起流式请求
  try {
    await invoke('send_analysis_message_stream', {
      request: {
        message: content,
        session_id: sessionId.value,
        config_id: selectedConfigId.value,
      }
    })
  } catch (err) {
    const idx = messages.value.findIndex(m => m.id === aiMsgId)
    if (idx !== -1) {
      messages.value[idx] = {
        ...messages.value[idx],
        content: `请求失败：${err}`,
        error: true,
        loading: false,
      }
    }
    isLoading.value = false
    currentUnlisten?.()
    currentUnlisten = null
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

// ─── 新建对话 ────────────────────────────────────────────────────────────────

const newConversation = () => {
  sessionId.value = crypto.randomUUID()
  activeSessionId.value = null
  messages.value = []
  msgIdCounter = 0
  showWelcome()
}

// ─── 恢复历史会话 ─────────────────────────────────────────────────────────────

const restoreSession = async (session: AnalysisSession) => {
  if (activeSessionId.value === session.id) return

  try {
    const records = await invoke<AnalysisMessageRecord[]>('get_analysis_messages', {
      sessionId: session.id,
    })

    sessionId.value = session.id
    activeSessionId.value = session.id
    messages.value = []
    msgIdCounter = 0

    showWelcome()

    for (const r of records) {
      messages.value.push({
        id: ++msgIdCounter,
        role: r.role as Message['role'],
        content: r.content,
      })
    }

    if (session.config_id) selectedConfigId.value = session.config_id

    await scrollToBottom()
  } catch (err) {
    console.error('恢复会话失败:', err)
  }
}

// ─── 删除历史会话 ─────────────────────────────────────────────────────────────

const deleteSession = async (id: string) => {
  try {
    await invoke('delete_analysis_session', { sessionId: id })
  } catch (_) {}

  if (id === sessionId.value) {
    sessionId.value = crypto.randomUUID()
    activeSessionId.value = null
    messages.value = []
    msgIdCounter = 0
    showWelcome()
  }
  historySessions.value = historySessions.value.filter(s => s.id !== id)
  deleteConfirmId.value = null
}

// ─── 欢迎消息 ────────────────────────────────────────────────────────────────

const showWelcome = () => {
  messages.value.push({
    id: ++msgIdCounter,
    role: 'system',
    content: '你好！我是您的财务分析助手，可以用自然语言向我提问，例如：\n- 帮我分析本月的支出结构\n- 过去3个月收支趋势怎样\n- 哪个分类花费最多？',
  })
}

// ─── 生命周期 ────────────────────────────────────────────────────────────────

onMounted(async () => {
  showWelcome()
  await Promise.all([loadHistory(), loadLLMConfigs()])
})

onUnmounted(() => {
  currentUnlisten?.()
})
</script>

<template>
  <div class="analysis-container">
    <!-- 历史会话侧边栏 -->
    <transition name="sidebar-slide">
      <div v-if="showHistoryPanel" class="history-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">历史会话</span>
          <el-button class="sidebar-close-btn" size="small" text @click="showHistoryPanel = false">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>
        <div class="sidebar-list">
          <div v-if="historySessions.length === 0" class="sidebar-empty">
            <el-icon class="empty-icon"><ChatDotRound /></el-icon>
            <span>暂无历史会话</span>
          </div>
          <div
            v-for="session in historySessions"
            :key="session.id"
            :class="['session-item', { 'session-active': activeSessionId === session.id }]"
            @click="restoreSession(session)"
          >
            <div class="session-item-body">
              <div class="session-title">{{ session.title }}</div>
              <div class="session-meta">{{ formatTime(session.updated_at) }}</div>
            </div>
            <div class="session-actions" @click.stop>
              <template v-if="deleteConfirmId === session.id">
                <el-button size="small" class="confirm-del-btn" @click="deleteSession(session.id)">确认</el-button>
                <el-button size="small" class="cancel-del-btn" @click="deleteConfirmId = null">取消</el-button>
              </template>
              <el-button v-else size="small" class="del-btn" text @click="deleteConfirmId = session.id">
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
        </div>
      </div>
    </transition>

    <!-- 主聊天区域 -->
    <div class="chat-main">
      <!-- 顶部工具栏 -->
      <div class="analysis-header">
        <div class="header-left">
          <el-button
            size="small"
            class="history-toggle-btn"
            :class="{ 'history-toggle-active': showHistoryPanel }"
            @click="showHistoryPanel = !showHistoryPanel"
          >
            <el-icon><Clock /></el-icon>
            历史
            <span v-if="historySessions.length > 0" class="history-badge">{{ historySessions.length }}</span>
          </el-button>
          <el-divider direction="vertical" class="header-divider" />
          <el-icon class="header-icon"><DataAnalysis /></el-icon>
          <span class="header-title">智能分析</span>
          <span class="header-subtitle">ChatBI · 对话式财务分析</span>
        </div>
        <div class="header-right">
          <!-- 模型选择 -->
          <el-select
            v-if="llmConfigs.length > 0"
            v-model="selectedConfigId"
            class="model-select"
            size="small"
            placeholder="选择模型"
          >
            <el-option
              v-for="cfg in llmConfigs"
              :key="cfg.id"
              :label="`${cfg.config_name || cfg.model}`"
              :value="cfg.id"
            >
              <div class="model-option">
                <span class="model-option-name">{{ cfg.config_name || cfg.model }}</span>
                <span class="model-option-provider">{{ cfg.provider }}</span>
              </div>
            </el-option>
          </el-select>
          <el-button size="small" class="new-chat-btn" @click="newConversation">
            <el-icon><Plus /></el-icon>
            新建对话
          </el-button>
        </div>
      </div>

      <!-- 消息列表 -->
      <div class="messages-area" ref="messagesContainer">
        <div v-for="msg in messages" :key="msg.id" :class="['msg-row', `msg-${msg.role}`]">
          <!-- 系统欢迎消息 -->
          <div v-if="msg.role === 'system'" class="msg-welcome">
            <div class="welcome-icon">🤖</div>
            <div class="welcome-text" v-html="renderMarkdown(msg.content)" />
          </div>

          <!-- 用户消息 -->
          <div v-else-if="msg.role === 'user'" class="msg-bubble msg-user-bubble">
            <span>{{ msg.content }}</span>
          </div>

          <!-- AI 消息 -->
          <div v-else class="msg-ai-wrapper">
            <div class="ai-avatar">AI</div>
            <div class="msg-bubble msg-ai-bubble" :class="{ 'msg-error': msg.error }">
              <div v-if="msg.loading && !msg.content" class="loading-dots">
                <span /><span /><span />
              </div>
              <template v-else>
                <div class="ai-text markdown-body" v-html="renderMarkdown(msg.content)" />
                <div v-if="msg.loading" class="streaming-cursor" />
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- 快捷问题 -->
      <div class="quick-questions">
        <span class="quick-label">快捷提问：</span>
        <el-tag
          v-for="q in QUICK_QUESTIONS"
          :key="q"
          class="quick-tag"
          :class="{ 'disabled': isLoading }"
          @click="!isLoading && sendMessage(q)"
        >{{ q }}</el-tag>
      </div>

      <!-- 输入区域 -->
      <div class="input-area">
        <el-input
          v-model="inputText"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 5 }"
          placeholder="输入您的财务问题，例如：上个月花最多的是哪个分类？（Shift+Enter 换行，Enter 发送）"
          class="chat-input"
          :disabled="isLoading"
          @keydown="handleKeydown"
        />
        <el-button
          class="send-btn"
          :loading="isLoading"
          :disabled="!inputText.trim()"
          @click="sendMessage()"
        >
          <el-icon v-if="!isLoading"><Promotion /></el-icon>
          {{ isLoading ? '分析中...' : '发送' }}
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.analysis-container {
  display: flex;
  flex-direction: row;
  height: 100%;
  background: #0d0d14;
  overflow: hidden;
}

/* ── 历史会话侧边栏 ── */
.history-sidebar {
  width: 240px;
  min-width: 240px;
  background: #0b0b12;
  border-right: 1px solid #1e1e30;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex-shrink: 0;
}
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 14px 10px;
  border-bottom: 1px solid #1e1e30;
  flex-shrink: 0;
}
.sidebar-title {
  font-size: 13px;
  font-weight: 600;
  color: #9090c0;
  letter-spacing: 0.5px;
}
.sidebar-close-btn { color: #5050a0 !important; padding: 2px !important; }
.sidebar-close-btn:hover { color: #a0a0d0 !important; }

.sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  scrollbar-width: thin;
  scrollbar-color: #2a2a40 transparent;
}
.sidebar-list::-webkit-scrollbar { width: 3px; }
.sidebar-list::-webkit-scrollbar-thumb { background: #2a2a40; border-radius: 2px; }

.sidebar-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 16px;
  color: #3a3a60;
  font-size: 12px;
}
.empty-icon { font-size: 28px; color: #2a2a50; }

.session-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
  border: 1px solid transparent;
}
.session-item:hover { background: #141422; border-color: #2a2a44; }
.session-item:hover .del-btn { opacity: 1; }
.session-active { background: #16162a !important; border-color: #4040a0 !important; }

.session-item-body { flex: 1; min-width: 0; }
.session-title {
  font-size: 13px;
  color: #b0b0d0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.4;
}
.session-active .session-title { color: #a5b4fc; }
.session-meta { font-size: 11px; color: #404060; margin-top: 2px; }
.session-active .session-meta { color: #5050a0; }
.session-actions { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }

.del-btn {
  color: #3a3a60 !important;
  padding: 2px !important;
  opacity: 0;
  transition: opacity 0.15s, color 0.15s;
}
.del-btn:hover { color: #ef4444 !important; }
.confirm-del-btn {
  font-size: 11px !important; height: 22px !important; padding: 0 6px !important;
  background: #3f1515 !important; border-color: #6b2020 !important; color: #fca5a5 !important;
}
.cancel-del-btn {
  font-size: 11px !important; height: 22px !important; padding: 0 6px !important;
  background: #1a1a2e !important; border-color: #3a3a5c !important; color: #9090c0 !important;
}

.sidebar-slide-enter-active,
.sidebar-slide-leave-active {
  transition: width 0.25s ease, min-width 0.25s ease, opacity 0.2s ease;
}
.sidebar-slide-enter-from,
.sidebar-slide-leave-to {
  width: 0 !important; min-width: 0 !important; opacity: 0;
}

/* ── 主聊天区域 ── */
.chat-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

/* ── 顶部 ── */
.analysis-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid #1e1e30;
  flex-shrink: 0;
  gap: 10px;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.header-divider { border-color: #2a2a44 !important; margin: 0 2px; }
.header-icon { font-size: 20px; color: #8b5cf6; }
.header-title { font-size: 16px; font-weight: 600; color: #e0e0ef; }
.header-subtitle {
  font-size: 12px; color: #6060a0;
  padding: 2px 8px; background: #1a1a2e; border-radius: 10px;
}
.history-toggle-btn {
  display: flex; align-items: center; gap: 5px;
  background: #1e1e30 !important; border-color: #3a3a5c !important;
  color: #7070a0 !important; font-size: 12px;
  height: 30px; padding: 0 10px !important;
}
.history-toggle-btn:hover,
.history-toggle-active {
  border-color: #6366f1 !important; color: #a5b4fc !important;
  background: #1a1a38 !important;
}
.history-badge {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 16px; height: 16px; padding: 0 4px;
  border-radius: 8px; background: #6366f1;
  color: #fff; font-size: 10px; font-weight: 600; line-height: 1;
}

/* 模型选择 */
.model-select {
  width: 160px;
}
.model-select :deep(.el-input__wrapper) {
  background: #1e1e30 !important;
  border-color: #3a3a5c !important;
  box-shadow: none !important;
  border-radius: 6px;
}
.model-select :deep(.el-input__inner) {
  color: #a0a0c0 !important;
  font-size: 12px;
}
.model-select :deep(.el-select__suffix) {
  color: #5050a0 !important;
}
.model-option {
  display: flex; align-items: center; justify-content: space-between; gap: 8px;
}
.model-option-name { font-size: 13px; }
.model-option-provider { font-size: 11px; color: #8080b0; }

.new-chat-btn {
  background: #1e1e30 !important; border-color: #3a3a5c !important;
  color: #a0a0c0 !important; font-size: 12px;
}
.new-chat-btn:hover { border-color: #6366f1 !important; color: #6366f1 !important; }

/* ── 消息列表 ── */
.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  scrollbar-width: thin;
  scrollbar-color: #2a2a40 transparent;
}
.messages-area::-webkit-scrollbar { width: 4px; }
.messages-area::-webkit-scrollbar-track { background: transparent; }
.messages-area::-webkit-scrollbar-thumb { background: #2a2a40; border-radius: 2px; }

.msg-welcome {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 16px 20px;
  background: linear-gradient(135deg, #1a1a2e, #16162a);
  border: 1px solid #2a2a50; border-radius: 12px;
}
.welcome-icon { font-size: 24px; flex-shrink: 0; }
.welcome-text { font-size: 14px; color: #b0b0d0; line-height: 1.8; }

.msg-user { display: flex; justify-content: flex-end; }
.msg-user-bubble {
  max-width: 65%; padding: 10px 16px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border-radius: 16px 16px 4px 16px;
  color: #fff; font-size: 14px; line-height: 1.6; word-break: break-word;
}

.msg-ai-wrapper { display: flex; align-items: flex-start; gap: 10px; }
.ai-avatar {
  width: 32px; height: 32px; border-radius: 50%;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 700; color: #fff; flex-shrink: 0;
}
.msg-ai-bubble {
  max-width: 75%; padding: 12px 16px;
  background: #16162a; border: 1px solid #2a2a44;
  border-radius: 4px 16px 16px 16px;
  color: #d0d0e8; font-size: 14px; line-height: 1.7; word-break: break-word;
}
.msg-error { border-color: #ef4444 !important; color: #fca5a5 !important; }

/* 流式光标 */
.streaming-cursor {
  display: inline-block;
  width: 2px;
  height: 14px;
  background: #6366f1;
  margin-left: 2px;
  animation: cursor-blink 0.8s infinite;
  vertical-align: text-bottom;
}
@keyframes cursor-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

/* Loading 动画 */
.loading-dots { display: flex; gap: 5px; align-items: center; padding: 4px 0; }
.loading-dots span {
  width: 7px; height: 7px; border-radius: 50%;
  background: #6366f1;
  animation: dot-bounce 1.2s infinite ease-in-out;
}
.loading-dots span:nth-child(1) { animation-delay: 0s; }
.loading-dots span:nth-child(2) { animation-delay: 0.2s; }
.loading-dots span:nth-child(3) { animation-delay: 0.4s; }
@keyframes dot-bounce {
  0%, 80%, 100% { transform: scale(0.6); opacity: 0.4; }
  40% { transform: scale(1); opacity: 1; }
}

/* ── Markdown 样式 ── */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  color: #c8c8e8;
  margin: 12px 0 6px;
  font-weight: 600;
}
.markdown-body :deep(h1) { font-size: 18px; }
.markdown-body :deep(h2) { font-size: 16px; }
.markdown-body :deep(h3) { font-size: 15px; }
.markdown-body :deep(p) { margin: 4px 0; }
.markdown-body :deep(strong) { color: #a5b4fc; font-weight: 600; }
.markdown-body :deep(em) { color: #c4b5fd; }
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 6px 0;
  padding-left: 20px;
}
.markdown-body :deep(li) { margin: 2px 0; }
.markdown-body :deep(code) {
  background: #1e1e30;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 13px;
  color: #e0b0ff;
}
.markdown-body :deep(pre) {
  background: #12121f;
  border: 1px solid #2a2a44;
  border-radius: 8px;
  padding: 12px;
  margin: 8px 0;
  overflow-x: auto;
}
.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  font-size: 13px;
  color: #d0d0e8;
}
.markdown-body :deep(table) {
  border-collapse: collapse;
  margin: 8px 0;
  width: 100%;
}
.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid #2a2a44;
  padding: 6px 10px;
  font-size: 13px;
}
.markdown-body :deep(th) {
  background: #1a1a2e;
  color: #a0a0d0;
  font-weight: 600;
}
.markdown-body :deep(td) { color: #c0c0e0; }
.markdown-body :deep(blockquote) {
  border-left: 3px solid #6366f1;
  padding-left: 12px;
  margin: 8px 0;
  color: #9090b0;
}
.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid #2a2a44;
  margin: 12px 0;
}
.markdown-body :deep(a) {
  color: #818cf8;
  text-decoration: underline;
}

/* ── 快捷问题 ── */
.quick-questions {
  padding: 10px 20px;
  display: flex; flex-wrap: wrap; gap: 8px; align-items: center;
  border-top: 1px solid #1e1e30; flex-shrink: 0;
}
.quick-label { font-size: 12px; color: #505070; flex-shrink: 0; }
.quick-tag {
  cursor: pointer;
  background: #1a1a2e !important; border-color: #3a3a5c !important;
  color: #8080b0 !important; font-size: 12px; transition: all 0.2s;
}
.quick-tag:hover:not(.disabled) {
  border-color: #6366f1 !important; color: #a5b4fc !important;
  background: #1e1e3a !important;
}
.quick-tag.disabled { cursor: not-allowed; opacity: 0.5; }

/* ── 输入区域 ── */
.input-area {
  display: flex; align-items: flex-end; gap: 10px;
  padding: 12px 20px 16px;
  border-top: 1px solid #1e1e30; flex-shrink: 0;
}
.chat-input { flex: 1; }
.chat-input :deep(.el-textarea__inner) {
  background: #12121f !important; border-color: #2a2a44 !important;
  color: #d0d0e8 !important; font-size: 14px;
  resize: none; border-radius: 10px; padding: 10px 14px;
}
.chat-input :deep(.el-textarea__inner:focus) {
  border-color: #6366f1 !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}
.send-btn {
  height: 42px; padding: 0 20px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6) !important;
  border: none !important; color: #fff !important;
  font-size: 14px; border-radius: 10px; flex-shrink: 0;
  transition: opacity 0.2s;
}
.send-btn:hover:not(:disabled) { opacity: 0.88; }
.send-btn:disabled { opacity: 0.4 !important; }
</style>
