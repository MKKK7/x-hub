<script setup lang="ts">
// 便签归档列表行
// 渲染一条归档：source 徽标 + 内容预览（前 80 字 + tooltip）+ 相对时间 + 恢复/删除
//
// 删除按钮仅 user reason 显示；auto_* 归档由后端永久保留、不允许前端删除
// 恢复按钮：所有 reason 都允许（slot / detached 都能恢复；写回 slot 时由 store 处理
// SLOT_OCCUPIED 错误并弹就地确认）

import { computed } from 'vue'
import { ArchiveRestore, Trash2, FileText, PanelTopClose } from 'lucide-vue-next'
import type { StickyArchive } from '../api/tauri'

const props = defineProps<{
  archive: StickyArchive
}>()

const emit = defineEmits<{
  (e: 'restore'): void
  (e: 'delete'): void
}>()

const preview = computed(() => {
  const c = props.archive.content
  // 第一行 80 字（去除空白）
  const firstLine = c.replace(/\s+/g, ' ').trim()
  return firstLine.length > 80 ? firstLine.slice(0, 80) + '…' : firstLine
})

const fullPreview = computed(() => {
  return props.archive.content.trim()
})

const relativeTime = computed(() => {
  // archived_at 是 "%Y-%m-%d %H:%M:%S%.6f" UTC，解析成 ms
  // 形如 "2026-09-27 12:34:56.789012"
  const m = props.archive.archived_at.match(
    /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})\.(\d{3})(\d{3})?$/,
  )
  if (!m) return props.archive.archived_at
  const [, y, mo, d, h, mi, s, ms3] = m
  const ms = Date.UTC(+y, +mo - 1, +d, +h, +mi, +s) + +ms3
  const diffSec = Math.max(0, Math.floor((Date.now() - ms) / 1000))
  if (diffSec < 60) return '刚刚'
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)} 分钟前`
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} 小时前`
  if (diffSec < 86400 * 30) return `${Math.floor(diffSec / 86400)} 天前`
  // 超过 30 天显示日期
  const date = new Date(ms)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
})

const sourceLabel = computed(() => {
  switch (props.archive.source) {
    case 'slot1':
      return { text: '便签 1', cls: 'src-slot' }
    case 'slot2':
      return { text: '便签 2', cls: 'src-slot' }
    case 'detached':
      return { text: '浮窗便签', cls: 'src-detached' }
    default:
      return { text: props.archive.source, cls: 'src-unknown' }
  }
})

const reasonLabel = computed(() => {
  switch (props.archive.reason) {
    case 'user':
      return { text: '手动', cls: 'reason-user' }
    case 'auto_replace':
      return { text: '覆盖前', cls: 'reason-auto' }
    case 'auto_destroy':
      return { text: '销毁前', cls: 'reason-auto' }
    default:
      return { text: props.archive.reason, cls: 'reason-unknown' }
  }
})

const SourceIcon = computed(() => {
  return props.archive.source.startsWith('slot') ? FileText : PanelTopClose
})
</script>

<template>
  <li class="archive-item">
    <div class="row-main">
      <div class="row-meta">
        <span class="src-badge" :class="sourceLabel.cls">
          <component :is="SourceIcon" class="src-icon" />
          {{ sourceLabel.text }}
        </span>
        <span class="reason-badge" :class="reasonLabel.cls">
          {{ reasonLabel.text }}
        </span>
        <span class="time">{{ relativeTime }}</span>
      </div>
      <div class="row-actions">
        <button
          type="button"
          class="btn btn-restore"
          :title="'恢复该归档'"
          @click="emit('restore')"
        >
          <ArchiveRestore :size="14" />
          <span>恢复</span>
        </button>
        <button
          v-if="archive.reason === 'user'"
          type="button"
          class="btn btn-delete"
          :title="'彻底删除该归档'"
          @click="emit('delete')"
        >
          <Trash2 :size="14" />
        </button>
      </div>
    </div>
    <div class="row-content" :title="fullPreview">
      {{ preview || '(空内容)' }}
    </div>
  </li>
</template>

<style scoped>
.archive-item {
  background: var(--bg-card);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: border-color 150ms ease-out, background 150ms ease-out;
}

.archive-item:hover {
  border-color: var(--border-strong);
}

.row-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.row-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-3);
}

.src-badge,
.reason-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.01em;
}

.src-badge .src-icon {
  width: 11px;
  height: 11px;
}

.src-slot {
  background: var(--brand-50);
  color: var(--brand-600);
}

.src-detached {
  background: var(--c-purple-soft, rgba(167, 139, 250, 0.18));
  color: var(--c-purple-ink, #6d28d9);
}

.reason-user {
  background: var(--c-yellow-soft, rgba(250, 204, 21, 0.18));
  color: var(--c-yellow-ink, #806600);
}

.reason-auto {
  background: var(--c-blue-soft, rgba(59, 130, 246, 0.16));
  color: var(--c-blue-ink, #1d4ed8);
}

.time {
  font-size: 12px;
  color: var(--text-3);
}

.row-actions {
  display: flex;
  gap: 6px;
  opacity: 0;
  transition: opacity 150ms ease-out;
}

.archive-item:hover .row-actions,
.archive-item:focus-within .row-actions {
  opacity: 1;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: transparent;
  color: var(--text-2);
  font-size: 12px;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.btn:hover {
  background: var(--bg-card-soft);
  border-color: var(--border-strong);
  color: var(--text-1);
}

.btn-restore:hover {
  background: var(--brand-50);
  color: var(--brand-600);
}

.btn-delete:hover {
  background: var(--c-red-soft, rgba(239, 68, 68, 0.16));
  color: var(--c-red-ink, #b91c1c);
  border-color: var(--c-red-ink, #b91c1c);
}

.row-content {
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 4.5em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
}
</style>