<script setup lang="ts">
// ClaudeHalo 进度监控卡片（Q2 重做 + 三个修复）
//
// 1. 提示未适配：state="unknown" 时显示「未配 hook」+「一键安装 hook」按钮
// 2. 会话累积：探测时自动清理死会话（claudehalo_get_status 内置）
// 3. 会话标题：claudehalo_get_status 返回 title 字段（EnumWindows 拿到的窗口标题）

import { computed, onMounted, ref } from 'vue'
import { Sparkles, Download } from 'lucide-vue-next'
import { tauriApi } from '../api/tauri'
import { useStore } from '../stores/workbench'
import { useAdaptivePolling } from '../composables/useAdaptivePolling'

defineProps<{ title?: string; hideTitle?: boolean }>()

const store = useStore()
const cardRef = ref<HTMLElement | null>(null)

useAdaptivePolling(() => store.refreshClaudeHalo(), {
  activeMs: 2000,
  idleMs: 5000,
  viewport: cardRef,
})

const sessions = computed(() => store.state.claudehaloSessions)

// 至少有一个 session state=unknown → 提示未适配
const needsHookSetup = computed(() =>
  sessions.value.some((s) => s.state === 'unknown'),
)

const installing = ref(false)
const installMsg = ref<string | null>(null)

async function installHook() {
  if (installing.value) return
  installing.value = true
  installMsg.value = null
  try {
    const r = await tauriApi.claudehaloInstallHook(null)
    installMsg.value = r.already_configured ? '已配置（幂等跳过）' : '已写入 ~/.claude/settings.json'
    // 触发一次刷新让卡片从 unknown 切到真实状态
    await store.refreshClaudeHalo()
  } catch (e) {
    installMsg.value = `安装失败: ${(e as Error)?.message ?? e}`
  } finally {
    installing.value = false
    setTimeout(() => { installMsg.value = null }, 3000)
  }
}

function stateClass(state: string) {
  switch (state) {
    case 'idle':
      return 'ch-state-idle'
    case 'thinking':
      return 'ch-state-thinking'
    case 'working':
      return 'ch-state-working'
    case 'waiting_input':
      return 'ch-state-waiting'
    case 'compacting':
      return 'ch-state-compacting'
    case 'completed':
      return 'ch-state-completed'
    default:
      return 'ch-state-unknown'
  }
}

function stateLabel(state: string) {
  switch (state) {
    case 'idle':
      return '空闲'
    case 'thinking':
      return '思考中'
    case 'working':
      return '执行中'
    case 'waiting_input':
      return '等待输入'
    case 'compacting':
      return '压缩中'
    case 'completed':
      return '已完成'
    case 'unknown':
      return '未配 hook'
    default:
      return state
  }
}

onMounted(async () => {
  await store.refreshClaudeHalo()
})
</script>

<template>
  <div ref="cardRef" class="card claudehalo-card" :class="{ 'card-no-title': hideTitle }">
    <header v-if="!hideTitle" class="hd">
      <h3 class="hd-title">
        <Sparkles class="ic" />
        <span>{{ title ?? 'Claude 进度' }}</span>
      </h3>
      <span class="status-dot" :class="sessions.some(s => s.state === 'thinking' || s.state === 'working' || s.state === 'waiting_input') ? 'busy' : 'idle'"></span>
    </header>

    <!-- 未配 hook 时的安装横幅 (问题 1) -->
    <div v-if="needsHookSetup" class="hook-banner">
      <p class="hook-text">检测到 Claude Code 但未配 ClaudeHalo hook</p>
      <button
        class="hook-btn"
        :disabled="installing"
        type="button"
        @click="installHook"
      >
        <Download :size="13" />
        <span>{{ installing ? '安装中…' : '一键安装 hook' }}</span>
      </button>
      <p v-if="installMsg" class="hook-msg">{{ installMsg }}</p>
    </div>

    <div v-if="sessions.length === 0" class="empty">
      <p class="empty-title">未检测到 Claude Code</p>
      <p class="empty-sub">在终端运行 <code>claude</code> 启动 Claude Code</p>
    </div>

    <ul v-else class="session-list">
      <li v-for="s in sessions" :key="s.pid" class="session-row">
        <div class="session-top">
          <span class="session-title" :title="s.title ?? ''">{{ s.title ?? `PID ${s.pid}` }}</span>
          <span class="session-state" :class="stateClass(s.state)">
            {{ stateLabel(s.state) }}
          </span>
        </div>
        <p v-if="s.state_file || s.pid !== undefined" class="session-meta">
          PID {{ s.pid }}<span v-if="s.state_file"> · {{ s.state_file }}</span>
        </p>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.card {
  background: var(--frost-surface);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-lg, 12px);
  box-shadow: var(--shadow-card);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.card-no-title {
  padding-top: 12px;
}
.hd {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.hd-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
  margin: 0;
}
.hd-title .ic {
  width: 14px;
  height: 14px;
  color: var(--brand-500, var(--accent));
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-pill);
  background: var(--text-4);
}
.status-dot.busy {
  background: var(--c-blue);
  box-shadow: 0 0 6px var(--c-blue);
  animation: ch-pulse 1.4s ease-in-out infinite;
}
@keyframes ch-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.hook-banner {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 10px;
  background: var(--c-yellow-soft, rgba(250, 204, 21, 0.16));
  border: 1px solid var(--c-yellow-ink, #806600);
  border-radius: 8px;
}
.hook-text {
  margin: 0;
  font-size: 11px;
  color: var(--c-yellow-ink, #806600);
  line-height: 1.4;
}
.hook-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 4px 10px;
  border: 1px solid var(--c-yellow-ink, #806600);
  border-radius: 6px;
  background: var(--c-yellow-ink, #806600);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
}
.hook-btn:hover:not(:disabled) {
  opacity: 0.85;
}
.hook-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.hook-msg {
  margin: 0;
  font-size: 10px;
  color: var(--c-green-ink, #15803d);
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px 8px;
  color: var(--text-3);
  gap: 4px;
}
.empty-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
  margin: 0;
}
.empty-sub {
  font-size: 11px;
  color: var(--text-3);
  margin: 0;
}
.empty-sub code {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  background: var(--bg-card-soft);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 10px;
}
.session-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
}
.session-row {
  background: var(--bg-card-soft);
  border-radius: 8px;
  padding: 6px 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.session-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.session-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.session-state {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 999px;
  flex-shrink: 0;
}
.ch-state-idle {
  background: var(--bg-card);
  color: var(--text-3);
}
.ch-state-thinking {
  background: var(--c-purple-soft, rgba(167, 139, 250, 0.18));
  color: var(--c-purple-ink, #6d28d9);
}
.ch-state-working {
  background: var(--c-blue-soft, rgba(59, 130, 246, 0.18));
  color: var(--c-blue-ink, #1d4ed8);
}
.ch-state-waiting {
  background: var(--c-orange-soft, rgba(245, 158, 11, 0.18));
  color: var(--c-orange-ink, #b45309);
}
.ch-state-compacting {
  background: var(--c-yellow-soft, rgba(250, 204, 21, 0.18));
  color: var(--c-yellow-ink, #806600);
}
.ch-state-completed {
  background: var(--c-green-soft, rgba(34, 197, 94, 0.16));
  color: var(--c-green-ink, #15803d);
}
.ch-state-unknown {
  background: var(--bg-card);
  color: var(--text-3);
}
.session-meta {
  font-size: 10px;
  color: var(--text-3);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>