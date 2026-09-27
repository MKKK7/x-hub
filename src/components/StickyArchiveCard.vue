<script setup lang="ts">
// 便签归档工作台概览部件
//
// 显示最近 5 条归档 + 「打开归档视图」跳转按钮
// 数据：store.stickyArchives（薄片 4 wiring：onMounted 调 loadStickyArchives）
//
// 注意：onOpenDetail 由 index.vue 的 dashCardProps 注入（参考 todo_overview）
//
// 渲染口径（与真卡一致）：
// - 按 archived_at DESC 取最近 5 条
// - 每行显示：source 徽标 + 内容预览（前 40 字）
// - 「查看全部」按钮 → openStickyArchive()

import { computed, onMounted } from 'vue'
import { ArchiveRestore, ArrowRight } from 'lucide-vue-next'
import { useStore } from '../stores/workbench'

const props = defineProps<{
  onOpenDetail?: () => void
  title?: string
  hideTitle?: boolean
}>()

const store = useStore()

const recent = computed(() => {
  return store.state.stickyArchives.slice(0, 5)
})

const total = computed(() => store.state.stickyArchives.length)

function previewOf(content: string) {
  const c = content.replace(/\s+/g, ' ').trim()
  return c.length > 40 ? c.slice(0, 40) + '…' : c
}

function sourceLabel(source: string) {
  switch (source) {
    case 'slot1':
      return '便签 1'
    case 'slot2':
      return '便签 2'
    case 'detached':
      return '浮窗'
    default:
      return source
  }
}

onMounted(async () => {
  await store.loadStickyArchives()
})
</script>

<template>
  <div class="card sticky-archive-card" :class="{ 'card-no-title': hideTitle }">
    <header v-if="!hideTitle" class="hd hd-split">
      <h3 class="hd-title">
        <ArchiveRestore class="ic" />
        <span>{{ title ?? '便签归档' }}</span>
      </h3>
      <span class="hd-btn" :title="'打开归档视图'" @click="props.onOpenDetail?.()" role="button">
        <ArrowRight class="ic" />
      </span>
    </header>

    <div v-if="total === 0" class="empty">
      <p class="empty-title">还没有归档便签</p>
      <p class="empty-sub">覆盖或销毁前自动留痕</p>
    </div>

    <div v-else class="body">
      <p class="body-meta">共 {{ total }} 条，最近 5 条：</p>
      <ul class="recent">
        <li v-for="a in recent" :key="a.id" class="recent-row">
          <span class="recent-src">{{ sourceLabel(a.source) }}</span>
          <span class="recent-content">{{ previewOf(a.content) || '(空)' }}</span>
        </li>
      </ul>
      <button
        v-if="total > 5"
        type="button"
        class="more"
        @click="props.onOpenDetail?.()"
      >
        查看全部 {{ total }} 条 →
      </button>
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
  gap: 8px;
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

.hd-btn {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  cursor: pointer;
  transition: all 150ms ease-out;
}

.hd-btn:hover {
  background: var(--bg-card-soft);
  color: var(--text-1);
}

.hd-btn .ic {
  width: 14px;
  height: 14px;
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

.body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
  min-height: 0;
}

.body-meta {
  font-size: 11px;
  color: var(--text-3);
  margin: 0;
}

.recent {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow: hidden;
}

.recent-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.4;
}

.recent-src {
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--bg-card-soft);
  color: var(--text-3);
  font-size: 10px;
  font-weight: 600;
}

.recent-content {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-1);
}

.more {
  margin-top: 4px;
  padding: 4px 8px;
  background: transparent;
  border: 1px dashed var(--border-soft);
  border-radius: 6px;
  color: var(--text-3);
  font-size: 11px;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.more:hover {
  background: var(--bg-card-soft);
  border-style: solid;
  color: var(--brand-600);
}
</style>