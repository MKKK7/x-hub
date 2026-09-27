<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { Cpu, MemoryStick, ArrowDown, ArrowUp } from 'lucide-vue-next'
import { useStore } from '../stores/workbench'
import { useAdaptivePolling } from '../composables/useAdaptivePolling'
import type { NetStats } from '../api/tauri'

// 标题可由工作台自定义布局覆盖：title = 自定义文案，hideTitle = 关闭标题行
defineProps<{ title?: string; hideTitle?: boolean }>()

const store = useStore()

// 卡片根元素：滚出工作台视口时暂停采样
const cardRef = ref<HTMLElement | null>(null)

// 自适应采样：可见且聚焦 1s、失焦 3s、隐藏/滚出视口停（Q1 网速要 1s 刷新）
useAdaptivePolling(() => store.refreshSystemInfo(), {
  activeMs: 1000,
  idleMs: 3000,
  viewport: cardRef,
})

// 网络速率：1s 差值计算（KB/s），prev 缓存上次字节数
const prevNet = ref<{ ts: number; rx: number; tx: number } | null>(null)
const rateKBps = ref({ rx: 0, tx: 0 })

function updateNetRates(stats: NetStats | null): void {
  if (!stats) return
  // 汇总所有接口的累计字节（去掉回环接口 0.0.0.0/Loopback）
  let totalRx = 0
  let totalTx = 0
  for (const iface of stats.interfaces) {
    if (iface.name.toLowerCase().includes('loopback')) continue
    totalRx += iface.rxBytes
    totalTx += iface.txBytes
  }
  const now = Date.now()
  if (prevNet.value && now > prevNet.value.ts) {
    const dtSec = (now - prevNet.value.ts) / 1000
    const dRx = totalRx - prevNet.value.rx
    const dTx = totalTx - prevNet.value.tx
    // 防止计数器回绕（系统休眠 / 网络重置导致负值）
    rateKBps.value = {
      rx: Math.max(0, dRx) / dtSec / 1024,
      tx: Math.max(0, dTx) / dtSec / 1024,
    }
  }
  prevNet.value = { ts: now, rx: totalRx, tx: totalTx }
}

// 单独订阅 net 采样（1s 周期与 useAdaptivePolling 同步）
useAdaptivePolling(async () => {
  const stats = await store.refreshNetStats()
  updateNetRates(stats)
}, {
  activeMs: 1000,
  idleMs: 3000,
  viewport: cardRef,
})

onBeforeUnmount(() => {
  prevNet.value = null
})

const info = () => store.state.systemInfo

const cpuPct = () => Math.round(info()?.cpuUsage ?? 0)
const memPct = () => Math.round(info()?.memPercent ?? 0)
const memLabel = () => {
  const i = info()
  if (!i) return '—'
  return `${(i.memUsedMb / 1024).toFixed(1)} / ${(i.memTotalMb / 1024).toFixed(1)} GB`
}

function formatKBps(kbps: number): string {
  if (kbps >= 1024) {
    return `${(kbps / 1024).toFixed(1)} MB/s`
  }
  return `${kbps.toFixed(1)} KB/s`
}
</script>

<template>
  <section ref="cardRef" class="card sys-monitor" :aria-label="title ?? '系统资源'">
    <header v-if="!hideTitle" class="sm-header">
      <h3 class="sm-title">
        <Cpu :size="14" :stroke-width="2" aria-hidden="true" />
        <span>{{ title ?? '系统资源' }}</span>
        <span class="sm-live-dot" aria-hidden="true"></span>
      </h3>
    </header>

    <div class="sm-body">
      <div class="sm-item">
        <div class="sm-item-top">
          <span class="sm-item-name">
            <Cpu :size="12" :stroke-width="2" aria-hidden="true" />
            CPU
          </span>
          <span class="sm-item-value">{{ cpuPct() }}<em>%</em></span>
        </div>
        <div class="sm-bar">
          <div
            class="sm-bar-fill"
            :class="{ warn: cpuPct() >= 85 }"
            :style="{ transform: 'scaleX(' + cpuPct() / 100 + ')' }"
          ></div>
        </div>
      </div>

      <div class="sm-item">
        <div class="sm-item-top">
          <span class="sm-item-name">
            <MemoryStick :size="12" :stroke-width="2" aria-hidden="true" />
            内存
          </span>
          <span class="sm-item-value">{{ memPct() }}<em>%</em></span>
        </div>
        <div class="sm-bar">
          <div
            class="sm-bar-fill"
            :class="{ warn: memPct() >= 85 }"
            :style="{ transform: 'scaleX(' + memPct() / 100 + ')' }"
          ></div>
        </div>
        <p class="sm-mem-label">{{ memLabel() }}</p>
      </div>

      <div class="sm-item">
        <div class="sm-item-top">
          <span class="sm-item-name">
            <ArrowDown :size="12" :stroke-width="2" aria-hidden="true" />
            下行
          </span>
          <span class="sm-item-value">{{ formatKBps(rateKBps.rx) }}</span>
        </div>
        <div class="sm-bar">
          <div
            class="sm-bar-fill sm-bar-net"
            :class="{ warn: rateKBps.rx >= 5 * 1024 }"
            :style="{ transform: 'scaleX(' + Math.min(rateKBps.rx / 5120, 1) + ')' }"
          ></div>
        </div>
      </div>

      <div class="sm-item">
        <div class="sm-item-top">
          <span class="sm-item-name">
            <ArrowUp :size="12" :stroke-width="2" aria-hidden="true" />
            上行
          </span>
          <span class="sm-item-value">{{ formatKBps(rateKBps.tx) }}</span>
        </div>
        <div class="sm-bar">
          <div
            class="sm-bar-fill sm-bar-net"
            :class="{ warn: rateKBps.tx >= 1 * 1024 }"
            :style="{ transform: 'scaleX(' + Math.min(rateKBps.tx / 2048, 1) + ')' }"
          ></div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 紧凑版：约 220px 总高，4 行 (CPU/内存/↓/↑)，无趋势图 */
.sys-monitor {
  display: flex;
  flex-direction: column;
  padding: 12px;
}
.sm-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.sm-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-1);
  letter-spacing: -0.01em;
  margin: 0;
}
.sm-title :deep(svg) {
  color: var(--brand-500);
}
.sm-live-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--c-green);
  animation: sm-blink 1.6s ease-in-out infinite;
  flex-shrink: 0;
}
@keyframes sm-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
.sm-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.sm-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sm-item-top {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}
.sm-item-name {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-2);
}
.sm-item-value {
  font-size: 0.875rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
  color: var(--text-1);
  line-height: 1;
}
.sm-item-value em {
  font-style: normal;
  font-size: 0.625rem;
  font-weight: 500;
  color: var(--text-3);
  margin-left: 2px;
}
.sm-bar {
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--bg-card-soft);
  overflow: hidden;
}
.sm-bar-fill {
  height: 100%;
  width: 100%;
  border-radius: var(--radius-pill);
  background: linear-gradient(90deg, var(--brand-600), var(--brand-500));
  transform-origin: left center;
  transition: transform 0.5s ease-out;
}
/* 网速：颜色与 CPU/内存不同, 用蓝色系区分 */
.sm-bar-net {
  background: linear-gradient(90deg, var(--c-blue), var(--c-blue-ink, #1d4ed8));
}
.sm-bar-fill.warn {
  background: linear-gradient(90deg, var(--c-orange), var(--c-red));
}
.sm-mem-label {
  margin: 0;
  font-size: 0.625rem;
  line-height: 1.2;
  color: var(--text-3);
}
</style>