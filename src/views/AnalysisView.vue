<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ElMessageBox } from 'element-plus'
import { marked } from 'marked'
import QuickNoteInlineConfirmCard from '../components/analysis/QuickNoteInlineConfirmCard.vue'
import { useAppStore, type QuickNoteDraft } from '../stores'

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
  message_type: string
  tool_calls_json: string | null
  tool_call_id: string | null
  tool_name: string | null
}

interface ToolStatusPayload {
  tool_name: string
  status: string
  description?: string
  tool_input?: string
  tool_output?: string
}

interface StreamChunkPayload {
  session_id: string
  chunk: string
  done: boolean
  error: string | null
  tool_status?: ToolStatusPayload
  prompt_tokens?: number
  completion_tokens?: number
  total_tokens?: number
  duration_ms?: number
}

interface Message {
  id: number
  role: 'user' | 'assistant' | 'system' | 'tool'
  content: string
  loading?: boolean
  error?: boolean
  toolStatus?: string
  messageType?: 'text' | 'tool_call' | 'tool_result' | 'quick_note_draft'
  toolName?: string
  toolInput?: string
  toolOutput?: string
  collapsed?: boolean
  draft?: QuickNoteDraft
  promptTokens?: number
  completionTokens?: number
  totalTokens?: number
  durationMs?: number
}

// ─── 常量 ────────────────────────────────────────────────────────────────────

// ─── 状态 ────────────────────────────────────────────────────────────────────

const sessionId = ref<string>(crypto.randomUUID())
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
const appStore = useAppStore()

// 输入法组合状态（用于区分输入法回车和真实发送回车）
const isInputComposing = ref(false)

// 流式监听
let currentUnlisten: UnlistenFn | null = null
let tempFilesUnlisten: UnlistenFn | null = null

// 图片缓存：相对路径 -> base64 data URL
const imageCache = ref<Record<string, string>>({})

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

// ─── 图片加载 ─────────────────────────────────────────────────────────────────

const IMAGE_PATH_REGEX = /!\[.*?\]\(([^)]+)\)/g

const extractImagePaths = (text: string): string[] => {
  const paths: string[] = []
  let match: RegExpExecArray | null
  while ((match = IMAGE_PATH_REGEX.exec(text)) !== null) {
    const path = match[1]
    if (path.startsWith('.query_temp/') && !path.startsWith('data:')) {
      paths.push(path)
    }
  }
  return [...new Set(paths)]
}

const loadImages = async (text: string) => {
  const paths = extractImagePaths(text)
  for (const path of paths) {
    if (imageCache.value[path]) continue
    try {
      const dataUrl = await invoke<string>('read_workspace_image', { relativePath: path })
      imageCache.value[path] = dataUrl
    } catch (e) {
      console.error('加载图片失败:', path, e)
    }
  }
}

const renderMarkdownWithImages = (text: string) => {
  let html = renderMarkdown(text)
  for (const [path, dataUrl] of Object.entries(imageCache.value)) {
    html = html.replace(new RegExp(`src="${path}"`, 'g'), `src="${dataUrl}"`)
  }
  return html
}

// 监听所有消息内容变化，自动加载其中引用的图片
watch(
  () => messages.value.map(m => m.content).join('\n'),
  (allText) => {
    loadImages(allText)
  },
  { immediate: true }
)

// ─── 发送消息（流式） ────────────────────────────────────────────────────────

const parseToolInput = (toolCallsJson: string): string => {
  try {
    const arr = JSON.parse(toolCallsJson)
    if (Array.isArray(arr) && arr.length > 0) {
      return typeof arr[0].arguments === 'string' ? arr[0].arguments : JSON.stringify(arr[0].arguments, null, 2)
    }
  } catch { /* ignore */ }
  return toolCallsJson
}

const sendMessage = async (text?: string) => {
  const content = (text ?? inputText.value).trim()
  if (!content || isLoading.value) return

  inputText.value = ''

  // 添加用户消息
  messages.value.push({ id: ++msgIdCounter, role: 'user', content, messageType: 'text' })

  // 当前 AI 文本 bubble 的 id（每次新的文本流开始时创建）
  let currentTextBubbleId: number | null = null
  // 初始 loading 占位
  const loadingMsgId = ++msgIdCounter
  messages.value.push({ id: loadingMsgId, role: 'assistant', content: '', loading: true, messageType: 'text' })
  isLoading.value = true
  await scrollToBottom()

  // 注册流式事件监听
  currentUnlisten = await listen<StreamChunkPayload>('analysis-stream-chunk', (event) => {
    const payload = event.payload
    if (payload.session_id !== sessionId.value) return

    if (payload.error) {
      // 找到当前 loading 的 bubble 或最后一个 assistant bubble
      const targetId = currentTextBubbleId ?? loadingMsgId
      const idx = messages.value.findIndex(m => m.id === targetId)
      if (idx !== -1) {
        messages.value[idx] = { ...messages.value[idx], content: payload.error, error: true, loading: false }
      }
      isLoading.value = false
      currentUnlisten?.()
      currentUnlisten = null
      tempFilesUnlisten?.()
      tempFilesUnlisten = null
      loadHistory()
      return
    }

    if (payload.done) {
      const targetId = currentTextBubbleId ?? loadingMsgId
      const idx = messages.value.findIndex(m => m.id === targetId)
      if (idx !== -1) {
        messages.value[idx] = {
          ...messages.value[idx],
          loading: false,
          toolStatus: undefined,
          promptTokens: payload.prompt_tokens,
          completionTokens: payload.completion_tokens,
          totalTokens: payload.total_tokens,
          durationMs: payload.duration_ms,
        }
      }
      // 移除空的 loading 占位（如果没有任何文本内容）
      if (!currentTextBubbleId) {
        const loadIdx = messages.value.findIndex(m => m.id === loadingMsgId)
        if (loadIdx !== -1 && !messages.value[loadIdx].content) {
          messages.value.splice(loadIdx, 1)
        }
      }
      isLoading.value = false
      currentUnlisten?.()
      currentUnlisten = null
      loadHistory()
      return
    }

    // 工具调用状态
    if (payload.tool_status) {
      const ts = payload.tool_status

      if (ts.status === 'calling') {
        // 移除初始 loading 占位（如果还在）
        if (!currentTextBubbleId) {
          const loadIdx = messages.value.findIndex(m => m.id === loadingMsgId)
          if (loadIdx !== -1 && !messages.value[loadIdx].content) {
            messages.value.splice(loadIdx, 1)
          }
        } else {
          // 结束当前文本 bubble 的 loading
          const idx = messages.value.findIndex(m => m.id === currentTextBubbleId)
          if (idx !== -1) messages.value[idx] = { ...messages.value[idx], loading: false }
          currentTextBubbleId = null
        }

        // 插入 tool_call bubble
        messages.value.push({
          id: ++msgIdCounter,
          role: 'assistant',
          content: '',
          messageType: 'tool_call',
          toolName: ts.tool_name,
          toolInput: ts.tool_input,
          loading: true,
        })
        scrollToBottom()
        return
      }

      if (ts.status === 'result') {
        // 结束上一个 tool_call bubble 的 loading
        const lastToolCall = [...messages.value].reverse().find(m => m.messageType === 'tool_call' && m.loading)
        if (lastToolCall) {
          const idx = messages.value.findIndex(m => m.id === lastToolCall.id)
          if (idx !== -1) messages.value[idx] = { ...messages.value[idx], loading: false }
        }

        // 插入 tool_result bubble
        messages.value.push({
          id: ++msgIdCounter,
          role: 'tool',
          content: ts.tool_output ?? '',
          messageType: 'tool_result',
          toolName: ts.tool_name,
          toolOutput: ts.tool_output,
          collapsed: true,
        })
        if (ts.tool_name === 'quick_note_parse') {
          void loadPendingDraftMessages(sessionId.value)
        }
        scrollToBottom()
        return
      }

      return
    }

    // 文本 chunk — 确保有一个 assistant text bubble
    if (!currentTextBubbleId) {
      // 看看初始 loading 占位是否还在
      const loadIdx = messages.value.findIndex(m => m.id === loadingMsgId)
      if (loadIdx !== -1 && !messages.value[loadIdx].content && messages.value[loadIdx].messageType === 'text') {
        currentTextBubbleId = loadingMsgId
      } else {
        // 创建新的文本 bubble
        currentTextBubbleId = ++msgIdCounter
        messages.value.push({ id: currentTextBubbleId, role: 'assistant', content: '', loading: true, messageType: 'text' })
      }
    }

    const idx = messages.value.findIndex(m => m.id === currentTextBubbleId)
    if (idx !== -1) {
      messages.value[idx] = {
        ...messages.value[idx],
        content: messages.value[idx].content + payload.chunk,
        loading: true,
        toolStatus: undefined,
      }
    }
    scrollToBottom()
  })

  // 注册临时文件删除确认事件监听
  tempFilesUnlisten = await listen('analysis-session-temp-files', (event) => {
    const payload = event.payload as { session_id: string; file_count: number; files: string[] }
    if (payload.session_id !== sessionId.value) return

    ElMessageBox.confirm(
      `本次对话生成了 ${payload.file_count} 个临时 CSV 查询结果文件，是否删除？`,
      '删除临时文件',
      {
        confirmButtonText: '删除',
        cancelButtonText: '保留',
        type: 'warning',
      }
    )
      .then(() => {
        invoke('cleanup_session_temp_files', { sessionId: sessionId.value })
      })
      .catch(() => {
        // 用户选择保留，不做任何操作
      })
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
    const targetId = currentTextBubbleId ?? loadingMsgId
    const idx = messages.value.findIndex(m => m.id === targetId)
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
    tempFilesUnlisten?.()
    tempFilesUnlisten = null
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey && !isInputComposing.value && e.keyCode !== 229) {
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
      const msgType = (r.message_type || 'text') as Message['messageType']
      messages.value.push({
        id: ++msgIdCounter,
        role: r.role as Message['role'],
        content: r.content,
        messageType: msgType,
        toolName: r.tool_name ?? undefined,
        toolInput: r.tool_calls_json ? parseToolInput(r.tool_calls_json) : undefined,
        toolOutput: msgType === 'tool_result' ? r.content : undefined,
        collapsed: msgType === 'tool_result',
      })
    }

    if (session.config_id) selectedConfigId.value = session.config_id

    await loadPendingDraftMessages(session.id)

    await scrollToBottom()
  } catch (err) {
    console.error('恢复会话失败:', err)
  }
}

const loadPendingDraftMessages = async (sid: string) => {
  const drafts = await appStore.fetchSessionPendingDrafts(sid)
  if (sid !== sessionId.value) return
  const tokens = await Promise.all(
    drafts.map(draft =>
      invoke<string>('get_quick_note_draft_token', { draftId: draft.draft_id }).catch(() => '')
    )
  )
  if (sid !== sessionId.value) return
  for (let i = 0; i < drafts.length; i++) {
    const draft = drafts[i]
    const token = tokens[i] || ''
    const mergedDraft = { ...draft, confirmation_token: token || draft.confirmation_token }
    const existingIndex = messages.value.findIndex(m => m.messageType === 'quick_note_draft' && m.draft?.draft_id === draft.draft_id)
    if (existingIndex >= 0) {
      messages.value[existingIndex] = {
        ...messages.value[existingIndex],
        draft: mergedDraft
      }
    } else {
      messages.value.push({
        id: ++msgIdCounter,
        role: 'assistant',
        content: '',
        messageType: 'quick_note_draft',
        draft: mergedDraft,
      })
    }
  }
}

const handleConfirmDraft = async (payload: { draftId: string; token: string; items: any[] }) => {
  await appStore.confirmQuickNoteDraft(payload.draftId, payload.token, payload.items)
  messages.value = messages.value.filter(m => m.draft?.draft_id !== payload.draftId)
}

const handleCancelDraft = async (draftId: string) => {
  await appStore.cancelQuickNoteDraft(draftId)
  messages.value = messages.value.filter(m => m.draft?.draft_id !== draftId)
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
  await Promise.all([loadHistory(), loadLLMConfigs()])
  if (historySessions.value.length > 0) {
    await restoreSession(historySessions.value[0])
  } else {
    showWelcome()
  }
})

onUnmounted(() => {
  currentUnlisten?.()
  tempFilesUnlisten?.()
})
</script>

<template>
  <div class="analysis-container">
    <!-- 历史会话侧边栏 -->
    <transition name="sidebar-slide">
      <aside v-if="showHistoryPanel" class="history-sidebar">
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
      </aside>
    </transition>

    <!-- 主聊天区域 -->
    <main class="chat-main">
      <!-- 顶部工具栏 -->
      <header class="analysis-header">
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
          <div class="header-brand">
            <el-icon class="header-icon"><DataAnalysis /></el-icon>
            <span class="header-title">智能分析</span>
            <span class="header-subtitle">ChatBI · 对话式财务分析</span>
          </div>
        </div>
        <div class="header-right">
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
      </header>

      <!-- 消息列表 -->
      <div class="messages-area" ref="messagesContainer">
        <div v-for="msg in messages" :key="msg.id" :class="['msg-row', `msg-${msg.role}`]">
          <!-- 系统欢迎消息 -->
          <div v-if="msg.role === 'system'" class="msg-welcome">
            <div class="welcome-icon">🤖</div>
            <div class="welcome-text" v-html="renderMarkdownWithImages(msg.content)" />
          </div>

          <!-- 用户消息 -->
          <div v-else-if="msg.role === 'user'" class="msg-bubble msg-user-bubble">
            <span>{{ msg.content }}</span>
          </div>

          <!-- Tool Call 消息 -->
          <div v-else-if="msg.messageType === 'tool_call'" class="msg-ai-wrapper">
            <div class="ai-avatar tool-avatar">🔧</div>
            <div class="tool-call-card">
              <div class="tool-card-header">
                <span class="tool-card-icon">🔧</span>
                <span class="tool-card-name">{{ msg.toolName }}</span>
                <span v-if="msg.loading" class="tool-card-status calling">调用中...</span>
                <span v-else class="tool-card-status done">已调用</span>
              </div>
              <div v-if="msg.toolInput" class="tool-card-body">
                <pre class="tool-card-code">{{ msg.toolInput }}</pre>
              </div>
            </div>
          </div>

          <!-- Tool Result 消息 -->
          <div v-else-if="msg.messageType === 'tool_result'" class="msg-ai-wrapper">
            <div class="ai-avatar result-avatar">📊</div>
            <div class="tool-result-card">
              <div class="tool-result-header" @click="msg.collapsed = !msg.collapsed">
                <span class="tool-card-icon">📊</span>
                <span class="tool-card-name">{{ msg.toolName }} 结果</span>
                <span class="tool-result-toggle">{{ msg.collapsed ? '展开 ▸' : '收起 ▾' }}</span>
              </div>
              <div v-if="!msg.collapsed" class="tool-result-body">
                <pre class="tool-result-code">{{ msg.toolOutput || msg.content }}</pre>
              </div>
            </div>
          </div>

          <!-- 快捷记账草稿 -->
          <div v-else-if="msg.messageType === 'quick_note_draft'" class="msg-ai-wrapper">
            <div class="ai-avatar draft-avatar">🧾</div>
            <QuickNoteInlineConfirmCard
              v-if="msg.draft"
              :draft="msg.draft"
              @confirm="handleConfirmDraft"
              @cancel="handleCancelDraft"
            />
          </div>

          <!-- AI 文本消息 -->
          <div v-else-if="msg.role === 'assistant'" class="msg-ai-wrapper">
            <div class="ai-avatar">AI</div>
            <div class="msg-bubble msg-ai-bubble" :class="{ 'msg-error': msg.error }">
              <div v-if="msg.loading && !msg.content && !msg.toolStatus" class="loading-dots">
                <span /><span /><span />
              </div>
              <template v-else>
                <div v-if="msg.content" class="ai-text markdown-body" v-html="renderMarkdownWithImages(msg.content)" />
                <div v-if="msg.toolStatus" class="tool-status-indicator">
                  <span class="tool-status-icon">&#128269;</span>
                  <span class="tool-status-text">{{ msg.toolStatus }}</span>
                  <span class="tool-status-dots"><span /><span /><span /></span>
                </div>
                <div v-if="msg.loading && !msg.toolStatus" class="streaming-cursor" />
              </template>
              <div
                v-if="msg.role === 'assistant' && msg.messageType === 'text' && (msg.totalTokens != null || msg.durationMs != null)"
                class="msg-meta"
              >
                <span v-if="msg.totalTokens != null" class="meta-item">
                  <el-icon><Coin /></el-icon>
                  {{ msg.totalTokens.toLocaleString() }} tokens
                </span>
                <span v-if="msg.durationMs != null" class="meta-item">
                  <el-icon><Timer /></el-icon>
                  {{ msg.durationMs >= 1000 ? `${(msg.durationMs / 1000).toFixed(2)}s` : `${msg.durationMs}ms` }}
                </span>
              </div>
            </div>
          </div>
        </div>
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
          @compositionstart="isInputComposing = true"
          @compositionend="isInputComposing = false"
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
    </main>
  </div>
</template>

<style scoped>
.analysis-container {
  display: flex;
  flex-direction: row;
  height: 100%;
  background: var(--ms-bg-primary);
  overflow: hidden;
}

/* ── 历史会话侧边栏 ── */
.history-sidebar {
  width: 260px;
  min-width: 260px;
  background: var(--ms-bg-secondary);
  border-right: 1px solid var(--ms-border-subtle);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex-shrink: 0;
}
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--ms-space-4) var(--ms-space-4) var(--ms-space-3);
  border-bottom: 1px solid var(--ms-border-subtle);
  flex-shrink: 0;
}
.sidebar-title {
  font-size: var(--ms-text-sm);
  font-weight: 600;
  color: var(--ms-text-tertiary);
  letter-spacing: 0.5px;
}
.sidebar-close-btn { color: var(--ms-text-tertiary) !important; padding: 2px !important; }
.sidebar-close-btn:hover { color: var(--ms-primary-500) !important; }

.sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--ms-space-2);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sidebar-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--ms-space-2);
  padding: var(--ms-space-10) var(--ms-space-4);
  color: var(--ms-text-tertiary);
  font-size: var(--ms-text-xs);
}
.empty-icon { font-size: 28px; color: var(--ms-text-tertiary); opacity: 0.6; }

.session-item {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
  padding: var(--ms-space-2) var(--ms-space-3);
  border-radius: var(--ms-radius-md);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
  border: 1px solid transparent;
}
.session-item:hover { background: var(--ms-surface-hover); border-color: var(--ms-border-subtle); }
.session-item:hover .del-btn { opacity: 1; }
.session-active { background: var(--ms-surface-active) !important; border-color: var(--ms-primary-500) !important; }

.session-item-body { flex: 1; min-width: 0; }
.session-title {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.4;
}
.session-active .session-title { color: var(--ms-text-primary); font-weight: 500; }
.session-meta { font-size: var(--ms-text-xs); color: var(--ms-text-tertiary); margin-top: 2px; }
.session-active .session-meta { color: var(--ms-primary-400); }
.session-actions { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }

.del-btn {
  color: var(--ms-text-tertiary) !important;
  padding: 2px !important;
  opacity: 0;
  transition: opacity 0.15s, color 0.15s;
}
.del-btn:hover { color: var(--ms-expense) !important; }
.confirm-del-btn {
  font-size: var(--ms-text-xs) !important; height: 22px !important; padding: 0 6px !important;
  background: rgba(244, 63, 94, 0.12) !important; border-color: var(--ms-expense) !important; color: var(--ms-expense) !important;
}
.cancel-del-btn {
  font-size: var(--ms-text-xs) !important; height: 22px !important; padding: 0 6px !important;
  background: var(--ms-surface-hover) !important; border-color: var(--ms-border-subtle) !important; color: var(--ms-text-secondary) !important;
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
  padding: var(--ms-space-3) var(--ms-space-5);
  border-bottom: 1px solid var(--ms-border-subtle);
  flex-shrink: 0;
  gap: var(--ms-space-3);
}
.header-left, .header-right {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
  flex-shrink: 0;
}
.header-brand {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
}
.header-divider { border-color: var(--ms-border-subtle) !important; margin: 0 2px; }
.header-icon { font-size: 20px; color: var(--ms-primary-500); }
.header-title { font-size: var(--ms-text-lg); font-weight: 600; color: var(--ms-text-primary); }
.header-subtitle {
  font-size: var(--ms-text-xs); color: var(--ms-text-tertiary);
  padding: 2px 8px; background: var(--ms-bg-tertiary); border-radius: 10px;
}
.history-toggle-btn {
  display: flex; align-items: center; gap: 5px;
  background: var(--ms-surface-secondary) !important; border-color: var(--ms-border-subtle) !important;
  color: var(--ms-text-secondary) !important; font-size: var(--ms-text-xs);
  height: 30px; padding: 0 10px !important;
}
.history-toggle-btn:hover,
.history-toggle-active {
  border-color: var(--ms-primary-500) !important; color: var(--ms-primary-500) !important;
  background: var(--ms-bg-tertiary) !important;
}
.history-badge {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 16px; height: 16px; padding: 0 4px;
  border-radius: 8px; background: var(--ms-primary-500);
  color: #fff; font-size: 10px; font-weight: 600; line-height: 1;
}

/* 模型选择 */
.model-select { width: 160px; }
.model-select :deep(.el-input__wrapper) {
  background: var(--ms-surface-secondary) !important;
  border-color: var(--ms-border-subtle) !important;
  box-shadow: none !important;
  border-radius: var(--ms-radius-md);
}
.model-select :deep(.el-input__inner) {
  color: var(--ms-text-secondary) !important;
  font-size: var(--ms-text-xs);
}
.model-select :deep(.el-select__suffix) {
  color: var(--ms-text-tertiary) !important;
}
.model-option {
  display: flex; align-items: center; justify-content: space-between; gap: 8px;
}
.model-option-name { font-size: var(--ms-text-sm); color: var(--ms-text-secondary); }
.model-option-provider { font-size: var(--ms-text-xs); color: var(--ms-text-tertiary); }

.new-chat-btn {
  background: var(--ms-surface-secondary) !important; border-color: var(--ms-border-subtle) !important;
  color: var(--ms-text-secondary) !important; font-size: var(--ms-text-xs);
}
.new-chat-btn:hover { border-color: var(--ms-primary-500) !important; color: var(--ms-primary-500) !important; }

/* ── 消息列表 ── */
.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: var(--ms-space-5);
  display: flex;
  flex-direction: column;
  gap: var(--ms-space-4);
}

.msg-welcome {
  display: flex; align-items: flex-start; gap: var(--ms-space-3);
  padding: var(--ms-space-4) var(--ms-space-5);
  background: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle); border-radius: var(--ms-radius-lg);
}
.welcome-icon { font-size: 24px; flex-shrink: 0; }
.welcome-text { font-size: var(--ms-text-sm); color: var(--ms-text-secondary); line-height: 1.8; }

.msg-user { display: flex; justify-content: flex-end; }
.msg-user-bubble {
  max-width: 70%; padding: var(--ms-space-2) var(--ms-space-4);
  background: var(--ms-gradient-primary);
  border-radius: var(--ms-radius-xl) var(--ms-radius-xl) 4px var(--ms-radius-xl);
  color: #fff; font-size: var(--ms-text-sm); line-height: 1.6; word-break: break-word;
}

.msg-ai-wrapper { display: flex; align-items: flex-start; gap: var(--ms-space-3); }
.ai-avatar {
  width: 32px; height: 32px; border-radius: 50%;
  background: var(--ms-gradient-primary);
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 700; color: #fff; flex-shrink: 0;
}
.msg-ai-bubble {
  max-width: 80%; padding: var(--ms-space-3) var(--ms-space-4);
  background: var(--ms-surface-secondary); border: 1px solid var(--ms-border-subtle);
  border-radius: 4px var(--ms-radius-xl) var(--ms-radius-xl) var(--ms-radius-xl);
  color: var(--ms-text-secondary); font-size: var(--ms-text-sm); line-height: 1.7; word-break: break-word;
}
.msg-error { border-color: var(--ms-expense) !important; color: var(--ms-expense) !important; }

/* ── Tool Call 卡片 ── */
.tool-avatar { background: var(--ms-gradient-expense) !important; font-size: 14px !important; }
.result-avatar { background: var(--ms-gradient-income) !important; font-size: 14px !important; }
.draft-avatar { background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%) !important; font-size: 14px !important; }

.tool-call-card {
  max-width: 80%;
  background: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  overflow: hidden;
}
.tool-card-header {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
  padding: var(--ms-space-2) var(--ms-space-3);
  background: var(--ms-bg-tertiary);
  border-bottom: 1px solid var(--ms-border-subtle);
}
.tool-card-icon { font-size: 14px; flex-shrink: 0; }
.tool-card-name { font-size: var(--ms-text-sm); font-weight: 600; color: var(--ms-text-primary); }
.tool-card-status {
  margin-left: auto;
  font-size: var(--ms-text-xs);
  padding: 1px 8px;
  border-radius: 10px;
}
.tool-card-status.calling {
  background: rgba(245, 158, 11, 0.12);
  color: var(--ms-warning);
  border: 1px solid rgba(245, 158, 11, 0.3);
}
.tool-card-status.done {
  background: rgba(16, 185, 129, 0.12);
  color: var(--ms-success);
  border: 1px solid rgba(16, 185, 129, 0.3);
}
.tool-card-body { padding: var(--ms-space-2) var(--ms-space-3); }
.tool-card-code {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-secondary);
  background: var(--ms-bg-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  padding: var(--ms-space-2) var(--ms-space-3);
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 120px;
  overflow-y: auto;
}

/* ── Tool Result 卡片 ── */
.tool-result-card {
  max-width: 80%;
  background: var(--ms-surface-secondary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  overflow: hidden;
}
.tool-result-header {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
  padding: var(--ms-space-2) var(--ms-space-3);
  background: var(--ms-bg-tertiary);
  cursor: pointer;
  user-select: none;
  transition: background 0.15s;
}
.tool-result-header:hover { background: var(--ms-surface-hover); }
.tool-result-toggle {
  margin-left: auto;
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
  transition: color 0.15s;
}
.tool-result-header:hover .tool-result-toggle { color: var(--ms-primary-500); }
.tool-result-body { padding: var(--ms-space-2) var(--ms-space-3); }
.tool-result-code {
  font-size: var(--ms-text-xs);
  color: var(--ms-text-secondary);
  background: var(--ms-bg-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  padding: var(--ms-space-2) var(--ms-space-3);
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
}

/* msg-tool row alignment */
.msg-tool { display: flex; justify-content: flex-start; }

/* 工具调用状态 */
.tool-status-indicator {
  display: flex;
  align-items: center;
  gap: var(--ms-space-2);
  padding: var(--ms-space-2) var(--ms-space-3);
  background: var(--ms-bg-tertiary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  margin-top: 6px;
}
.tool-status-icon { font-size: 14px; flex-shrink: 0; }
.tool-status-text { font-size: var(--ms-text-xs); color: var(--ms-primary-500); }
.tool-status-dots { display: inline-flex; gap: 3px; align-items: center; }
.tool-status-dots span {
  width: 5px; height: 5px; border-radius: 50%;
  background: var(--ms-primary-500);
  animation: dot-bounce 1.2s infinite ease-in-out;
}
.tool-status-dots span:nth-child(1) { animation-delay: 0s; }
.tool-status-dots span:nth-child(2) { animation-delay: 0.2s; }
.tool-status-dots span:nth-child(3) { animation-delay: 0.4s; }

.msg-meta {
  display: flex;
  align-items: center;
  gap: var(--ms-space-3);
  margin-top: var(--ms-space-2);
  padding-top: var(--ms-space-2);
  border-top: 1px solid var(--ms-border-subtle);
  font-size: var(--ms-text-xs);
  color: var(--ms-text-tertiary);
}
.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* 流式光标 */
.streaming-cursor {
  display: inline-block;
  width: 2px;
  height: 14px;
  background: var(--ms-primary-500);
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
  background: var(--ms-primary-500);
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
  color: var(--ms-text-primary);
  margin: var(--ms-space-3) 0 var(--ms-space-2);
  font-weight: 600;
}
.markdown-body :deep(h1) { font-size: var(--ms-text-xl); }
.markdown-body :deep(h2) { font-size: var(--ms-text-lg); }
.markdown-body :deep(h3) { font-size: var(--ms-text-base); }
.markdown-body :deep(p) { margin: var(--ms-space-1) 0; }
.markdown-body :deep(strong) { color: var(--ms-text-primary); font-weight: 600; }
.markdown-body :deep(em) { color: var(--ms-primary-400); }
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: var(--ms-space-2) 0;
  padding-left: 20px;
}
.markdown-body :deep(li) { margin: 2px 0; }
.markdown-body :deep(code) {
  background: var(--ms-bg-tertiary);
  padding: 1px 5px;
  border-radius: 3px;
  font-size: var(--ms-text-sm);
  color: var(--ms-primary-500);
}
.markdown-body :deep(pre) {
  background: var(--ms-bg-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-md);
  padding: var(--ms-space-3);
  margin: var(--ms-space-2) 0;
  overflow-x: auto;
}
.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
}
.markdown-body :deep(table) {
  border-collapse: collapse;
  margin: var(--ms-space-2) 0;
  width: 100%;
}
.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid var(--ms-border-subtle);
  padding: var(--ms-space-2) var(--ms-space-3);
  font-size: var(--ms-text-sm);
}
.markdown-body :deep(th) {
  background: var(--ms-bg-tertiary);
  color: var(--ms-text-secondary);
  font-weight: 600;
}
.markdown-body :deep(td) { color: var(--ms-text-secondary); }
.markdown-body :deep(blockquote) {
  border-left: 3px solid var(--ms-primary-500);
  padding-left: var(--ms-space-3);
  margin: var(--ms-space-2) 0;
  color: var(--ms-text-tertiary);
}
.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid var(--ms-border-subtle);
  margin: var(--ms-space-3) 0;
}
.markdown-body :deep(a) {
  color: var(--ms-primary-500);
  text-decoration: underline;
}
.markdown-body :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: var(--ms-radius-md);
  display: block;
  margin: var(--ms-space-2) 0;
}

/* ── 输入区域 ── */
.input-area {
  display: flex; align-items: flex-end; gap: var(--ms-space-3);
  padding: var(--ms-space-3) var(--ms-space-5) var(--ms-space-4);
  border-top: 1px solid var(--ms-border-subtle); flex-shrink: 0;
}
.chat-input { flex: 1; }
.chat-input :deep(.el-textarea__inner) {
  background: var(--ms-surface-secondary) !important; border-color: var(--ms-border-subtle) !important;
  color: var(--ms-text-primary) !important; font-size: var(--ms-text-sm);
  resize: none; border-radius: var(--ms-radius-lg); padding: var(--ms-space-2) var(--ms-space-4);
}
.chat-input :deep(.el-textarea__inner:focus) {
  border-color: var(--ms-primary-500) !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.15);
}
.send-btn {
  height: 42px; padding: 0 var(--ms-space-5);
  background: var(--ms-gradient-primary) !important;
  border: none !important; color: #fff !important;
  font-size: var(--ms-text-sm); border-radius: var(--ms-radius-lg); flex-shrink: 0;
  transition: opacity 0.2s;
}
.send-btn:hover:not(:disabled) { opacity: 0.88; }
.send-btn:disabled { opacity: 0.4 !important; }
</style>
