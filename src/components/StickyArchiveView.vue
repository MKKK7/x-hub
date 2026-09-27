<script setup lang="ts">
// 便签归档独立视图
//
// 入口：侧栏「归档」图标 / StickyArchiveCard 卡片「打开视图」按钮
// 数据：store.stickyArchives（薄片 4 wiring：onMounted 调 loadStickyArchives）
// 交互：
//   - 顶部 AppSelect 按 source 过滤（全部 / slot1 / slot2 / detached）
//   - 列表行内「恢复」按钮（slot 占用时弹 SLOT_OCCUPIED 确认 / detached 直接新建）
//   - 「彻底删除」按钮（仅 user reason 显示）
//   - 行用 StickyArchiveItem 子组件渲染（列表项模板独立可复用）

import { computed, onMounted, ref } from 'vue'
import { ArchiveRestore, Trash2, Inbox } from 'lucide-vue-next'
import { useStore } from '../stores/workbench'
import StickyArchiveItem from './StickyArchiveItem.vue'
import StickyArchiveRestoreDialog from './StickyArchiveRestoreDialog.vue'
import AppSelect from './AppSelect.vue'
import { useFocusTrap } from '../composables/useFocusTrap'

const store = useStore()

// 过滤：'all' | 'slot1' | 'slot2' | 'detached'
const sourceFilter = ref<'all' | 'slot1' | 'slot2' | 'detached'>('all')

const filteredArchives = computed(() => {
  const all = store.state.stickyArchives
  if (sourceFilter.value === 'all') return all
  return all.filter((a) => a.source === sourceFilter.value)
})

// 恢复对话框状态（slot 占用时弹出）
const pendingRestore = ref<{
  archiveId: number
  source: string
  content: string
} | null>(null)

async function onRestore(archiveId: number, _source: string) {
  try {
    await store.restoreStickyArchive(archiveId)
  } catch (e) {
    const msg = String((e as Error)?.message ?? e)
    if (msg.startsWith('SLOT_OCCUPIED')) {
      // slot 被占：弹就地确认（覆盖现有 slot 内容）
      const a = store.state.stickyArchives.find((x) => x.id === archiveId)
      if (a) {
        pendingRestore.value = {
          archiveId,
          source: a.source,
          content: a.content,
        }
      }
      return
    }
    if (msg.startsWith('NOT_FOUND')) {
      // 后端无此归档（极端竞态：刚被删）—— 刷一次列表
      await store.loadStickyArchives()
      return
    }
    throw e
  }
}

async function onConfirmOverwrite() {
  if (!pendingRestore.value) return
  const { archiveId } = pendingRestore.value
  pendingRestore.value = null
  // 后端按 source 还原，slot 占用时直接覆盖（无需前端二次确认 —— 用户已在弹窗确认）
  try {
    await store.restoreStickyArchive(archiveId)
  } catch (e) {
    const msg = String((e as Error)?.message ?? e)
    if (msg.startsWith('NOT_FOUND')) {
      await store.loadStickyArchives()
      return
    }
    throw e
  }
}

function onCancelOverwrite() {
  pendingRestore.value = null
}

const dialogRef = ref<HTMLElement | null>(null)
const dialogVisible = computed(() => pendingRestore.value !== null)
useFocusTrap(dialogVisible, dialogRef)

async function onDelete(archiveId: number) {
  // 仅删除 user reason 的归档（auto_* 由后端保留，UI 层不展示删除按钮）
  const a = store.state.stickyArchives.find((x) => x.id === archiveId)
  if (!a || a.reason !== 'user') return
  await store.deleteStickyArchive(archiveId)
}

onMounted(async () => {
  await store.loadStickyArchives()
})
</script>

<template>
  <div class="sticky-archive-view">
    <header class="hd">
      <div class="hd-left">
        <ArchiveRestore class="hd-icon" />
        <h2 class="hd-title">便签归档</h2>
        <span class="hd-count">{{ filteredArchives.length }} 条</span>
      </div>
      <div class="hd-right">
        <AppSelect
          v-model="sourceFilter"
          :options="[
            { value: 'all', label: '全部来源' },
            { value: 'slot1', label: '便签 1' },
            { value: 'slot2', label: '便签 2' },
            { value: 'detached', label: '浮窗便签' },
          ]"
        />
      </div>
    </header>

    <div v-if="filteredArchives.length === 0" class="empty">
      <Inbox class="empty-icon" />
      <p class="empty-title">还没有归档便签</p>
      <p class="empty-sub">
        覆盖前自动归档 · 浮窗关闭销毁前自动归档 · 手动 ⋯ → 归档
      </p>
    </div>

    <ul v-else class="archive-list">
      <StickyArchiveItem
        v-for="a in filteredArchives"
        :key="a.id"
        :archive="a"
        @restore="onRestore(a.id, a.source)"
        @delete="onDelete(a.id)"
      />
    </ul>

    <StickyArchiveRestoreDialog
      v-if="pendingRestore"
      ref="dialogRef"
      :archive="pendingRestore"
      @confirm="onConfirmOverwrite"
      @cancel="onCancelOverwrite"
    />

    <button
      v-if="false"
      class="trash-hidden"
      :title="'delete action handler'"
      aria-hidden="true"
    >
      <Trash2 />
    </button>
  </div>
</template>

<style scoped>
.sticky-archive-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px 20px 20px;
}

.hd {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.hd-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.hd-icon {
  width: 18px;
  height: 18px;
  color: var(--brand-500, var(--accent));
}

.hd-title {
  font-size: 20px;
  font-weight: 700;
  line-height: 1.25;
  color: var(--text-1);
  margin: 0;
}

.hd-count {
  font-size: 12px;
  color: var(--text-3);
  margin-left: 4px;
}

.hd-right {
  min-width: 160px;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  color: var(--text-3);
}

.empty-icon {
  width: 48px;
  height: 48px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-2);
  margin: 0 0 4px;
}

.empty-sub {
  font-size: 12px;
  color: var(--text-3);
  margin: 0;
}

.archive-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  flex: 1;
}

/* 隐藏但保留 Trash2 引用 —— Vue 模板会把 Trash2 当作 icon 节点用 */
.trash-hidden {
  display: none;
}
</style>