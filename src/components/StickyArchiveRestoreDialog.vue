<script setup lang="ts">
// 便签归档恢复确认弹窗
//
// 仅在 slot 被占时弹出（store 捕获 SLOT_OCCUPIED 错误后置 pendingRestore）
// 让用户选择「覆盖当前 slot」还是「取消」
// detached 来源不弹此弹窗（恢复时直接创建新 detached sticky）

import { AlertTriangle } from 'lucide-vue-next'

const props = defineProps<{
  archive: {
    archiveId: number
    source: string
    content: string
  }
}>()

const emit = defineEmits<{
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

function onConfirm() {
  emit('confirm')
}
function onCancel() {
  emit('cancel')
}

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit('cancel')
}
</script>

<template>
  <div class="modal-mask" @click="onBackdrop">
    <div class="modal-card" role="dialog" aria-labelledby="restore-title">
      <header class="card-hd">
        <AlertTriangle class="warn-icon" :size="18" />
        <h3 id="restore-title">覆盖当前便签？</h3>
      </header>
      <div class="card-body">
        <p>
          {{ archive.source === 'slot1' ? '便签 1' : '便签 2' }}
          已有内容，恢复归档会覆盖它。如不确定可点取消先去归档当前内容。
        </p>
        <p class="archive-preview">{{ archive.content.slice(0, 120) }}{{ archive.content.length > 120 ? '…' : '' }}</p>
      </div>
      <footer class="card-ft">
        <button type="button" class="btn btn-secondary" @click="onCancel">取消</button>
        <button type="button" class="btn btn-danger" @click="onConfirm">覆盖并恢复</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  inset: 0;
  background: var(--scrim);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(6px);
}

.modal-card {
  background: var(--bg-card-solid);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: var(--shadow-dock);
  min-width: 320px;
  max-width: 480px;
  display: flex;
  flex-direction: column;
}

.card-hd {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 18px 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
}

.warn-icon {
  color: var(--c-red-ink, #b91c1c);
}

.card-hd h3 {
  font-size: 15px;
  font-weight: 700;
  margin: 0;
}

.card-body {
  padding: 14px 18px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-1);
}

.card-body p {
  margin: 0 0 8px;
}

.archive-preview {
  padding: 8px 10px;
  background: var(--bg-card-soft);
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-2);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 4em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
}

.card-ft {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 4px 18px 16px;
}

.btn {
  padding: 6px 14px;
  border-radius: 6px;
  border: 1px solid var(--border-soft);
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 150ms ease-out;
}

.btn:hover {
  background: var(--bg-card-soft);
  color: var(--text-1);
}

.btn-danger {
  background: var(--c-red-ink, #b91c1c);
  border-color: var(--c-red-ink, #b91c1c);
  color: #fff;
}

.btn-danger:hover {
  background: #991b1b;
  border-color: #991b1b;
  color: #fff;
}
</style>