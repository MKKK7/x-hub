<script setup lang="ts">
// ClaudeHalo 进度监控卡片（Q2）
//
// 数据源：tauriApi.claudehaloGetStatus() 探测本地 7700..7707 端口的 HTTP /status
// 多 session 通过 port + hostname 区分
// 通知状态切换由 notify.rs 自绘右下角通知窗（详见后续薄片）

import { computed, onMounted } from 'vue'
import { Sparkles } from 'lucide-vue-next'
import { useStore } from '../stores/workbench'
import { useAdaptivePolling } from '../composables/useAdaptivePolling'

defineProps<{ title?: string; hideTitle?: boolean }>()

const store = useStore()
const cardRef = ref<HTMLElement | null>(null)

// 1s 轮询探测（探测本身异步 + 超时 200ms，对前端无压力）
useAdaptivePolling(() => store.refreshClaudeHalo(), {
  activeMs: 1000,
  idleMs: 3000,
  viewport: cardRef,
})

const sessions = computed(() => store.state.claudehaloSessions)

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
    default:
      return state
  }
}

import { ref } from 'vue'
onMounted(async () => {
  await store.refreshClaudeHalo()
})
</script>

<template>
  <div ref="cardRef" class="card claudehalo-card" :class="{ 'card-no-title': hideTitle }">
    <header v-if="!hideTitle" class="hd hd-split">
      <h3 class="hd-title">
        <Sparkles class="ic" />
        <span>{{ title ?? 'Claude 进度' }}</span>
      </h3>
    </header>

    <div v-if="sessions.length === 0" class="empty">
      <p class="empty-title">未检测到 ClaudeHalo</p>
      <p class="empty-sub">启动 ClaudeHaloManage 后将自动出现</p>
    </div>

    <ul v-else class="session-list">
      <li v-for="s in sessions" :key="s.port" class="session-row">
        <div class="session-top">
          <span class="session-port">:{{ s.port }}</span>
          <span class="session-state" :class="stateClass(s.state)">
            {{ stateLabel(s.state) }}
          </span>
        </div>
        <p v-if="s.hostname || s.session_id" class="session-meta">
          {{ s.hostname ?? '' }}<span v-if="s.session_id"> · {{ s.session_id }}</span>
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
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px 8px;
  color: var(--text-3);
}
.empty-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
  margin: 0 0 2px;
}
.empty-sub {
  font-size: 11px;
  color: var(--text-3);
  margin: 0;
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
}
.session-port {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
}
.session-state {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 999px;
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