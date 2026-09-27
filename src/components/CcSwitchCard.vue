<script setup lang="ts">
// CC Switch 探测卡片（Q5）
//
// 数据源：tauriApi.ccswitchGetStatus() 探测本地 cc-switch 进程 + ~/.cc-switch/settings.json
// 不修改 cc-switch 配置, 仅展示状态（用户原话: 不在 x-hub 里塞 CC Switch）
// 30s 重试（用户原话: 入口一直可见, 30s 后台轮询重试）

import { onMounted, ref } from 'vue'
import { Plug } from 'lucide-vue-next'
import { useStore } from '../stores/workbench'
import { useAdaptivePolling } from '../composables/useAdaptivePolling'
import type { CcSwitchStatus } from '../api/tauri'

defineProps<{ title?: string; hideTitle?: boolean }>()

const store = useStore()
const cardRef = ref<HTMLElement | null>(null)

// store 字段（避免大改 StoreState, Q5 数据只在卡片用）
const status = ref<CcSwitchStatus | null>(null)

// 30s 重试 + 5s 慢速空闲, 卡片常驻可见一直跑
useAdaptivePolling(async () => {
  status.value = await store.refreshCcSwitch()
}, {
  activeMs: 30000,
  idleMs: 30000, // 即便 idle 也持续重试, 入口一直可见
  viewport: cardRef,
})

onMounted(async () => {
  status.value = await store.refreshCcSwitch()
})
</script>

<template>
  <div ref="cardRef" class="card ccswitch-card" :class="{ 'card-no-title': hideTitle }">
    <header v-if="!hideTitle" class="hd">
      <h3 class="hd-title">
        <Plug class="ic" />
        <span>{{ title ?? 'CC Switch' }}</span>
      </h3>
      <span class="status-dot" :class="status?.running ? 'on' : 'off'" :title="status?.running ? '运行中' : '未检测到'"></span>
    </header>

    <div v-if="status?.running" class="body">
      <p class="row">
        <span class="row-key">PID</span>
        <span class="row-val">{{ status.pid }}</span>
      </p>
      <p class="row">
        <span class="row-key">当前 provider</span>
        <span class="row-val">{{ status.current_provider ?? '—' }}</span>
      </p>
      <p class="row">
        <span class="row-key">Provider 数</span>
        <span class="row-val">{{ status.provider_count }}</span>
      </p>
      <p v-if="status.config_dir" class="config-path" :title="status.config_dir">
        {{ status.config_dir }}
      </p>
    </div>

    <div v-else class="empty">
      <p class="empty-title">未检测到 CC Switch</p>
      <p class="empty-sub">启动 cc-switch 后将自动出现</p>
      <p v-if="status?.config_dir" class="config-path" :title="status.config_dir">
        {{ status.config_dir }}
      </p>
    </div>
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
  flex-shrink: 0;
}
.status-dot.on {
  background: var(--c-green);
  box-shadow: 0 0 6px var(--c-green);
}
.status-dot.off {
  background: var(--text-4);
}
.body {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin: 0;
  font-size: 12px;
}
.row-key {
  color: var(--text-3);
}
.row-val {
  color: var(--text-1);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.config-path {
  margin: 4px 0 0;
  padding-top: 6px;
  border-top: 1px solid var(--border-soft);
  font-size: 10px;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 12px 8px;
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
</style>