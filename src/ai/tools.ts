// AI 工具调用系统（Q6）
//
// 设计：DIY tool calling —— 不依赖 OpenAI/Anthropic 原生 function-calling 协议，
// 改在 system prompt 注入工具说明，AI 在回复里用 `<tool_call>{json}</tool_call>`
// 块表达调用意图，前端 ChatPanel 解析执行后把结果以新 user 消息再发给 AI。
// 优势：与现有 chat.rs SSE 透传完全兼容、零后端改造、跨供应商（OpenAI / DeepSeek / Ollama 等）通用。

import { isTauri, tauriApi } from '../api/tauri'
import type { Countdown, Todo } from '../api/tauri'

/** 工具 schema 描述：会拼到 system prompt */
export const TOOL_DESCRIPTIONS = `【可用工具】（仅当用户明确要做对应操作时调用，不要无端使用）

工具调用格式（必须严格遵守）：
<tool_call>
{"name": "<工具名>", "args": { ... }}
</tool_call>

单条回复里可以连续多次 <tool_call>...</tool_call> 表示多个操作。

工具列表：

1. add_countdown(args: { name: string, endAt: number, repeatMode?: 'once'|'daily'|'interval', totalMs?: number, intervalMinutes?: number|null })
   - endAt 为毫秒时间戳；once/daily 模式 totalMs 必填（用于水位），interval 模式 intervalMinutes 必填
   - 例：添加明天 9 点提醒：endAt = (明天 9 点对应的 epoch ms)

2. add_todo(args: { title: string, priority?: 0|1|2, dueAt?: number|null })
   - priority 0=普通 1=重要 2=紧急；dueAt 毫秒时间戳
   - 例：添加待办"开会"：{"name":"add_todo","args":{"title":"开会","priority":1}}

3. add_sticky(args: { slot: 1|2, content: string })
   - 工作台便签卡片（slot 1/2）；content 为便签全文
   - 例：写便签 1 内容"买菜清单"：{"name":"add_sticky","args":{"slot":1,"content":"买菜清单"}}

4. add_note(args: { title: string, content: string })
   - 速记笔记（Markdown 内容）；title 留空则自动从首行取
   - 例：{"name":"add_note","args":{"title":"周报","content":"## 本周完成\\n- xxx\\n- yyy"}}

5. read_file(args: { path: string })
   - 读取文件全文（用户提供的路径，例如用户在消息里写"@D:/path/to/file.md"）
   - 返回 {content, error?: string}
   - 仅文本文件（最大 500KB），不要读取二进制

注意：
- 用户没要求做某事时不要主动调工具（不要"帮"用户加无意义的内容）
- 路径必须在用户消息里显式给出，不要编造
- 工具执行失败（store action 报错）要把 error 字段如实告诉用户
- 工具调用块之后可以继续写自然语言回复，告诉用户你做了什么
`

/** 工具名 → args 形状 → 返回结果 */
export interface ToolCall {
  name: string
  args: Record<string, unknown>
}

export interface ToolResult {
  ok: boolean
  data?: unknown
  error?: string
}

/** 单个工具执行（store.action 包装） */
export async function executeTool(call: ToolCall): Promise<ToolResult> {
  if (!isTauri()) {
    return { ok: false, error: '浏览器预览模式不支持工具执行' }
  }
  try {
    switch (call.name) {
      case 'add_countdown': {
        const { name, endAt, repeatMode = 'once', totalMs, intervalMinutes } = call.args as {
          name: string
          endAt: number
          repeatMode?: 'once' | 'daily' | 'interval'
          totalMs?: number
          intervalMinutes?: number | null
        }
        if (!name || typeof endAt !== 'number') {
          return { ok: false, error: 'add_countdown 需要 name + endAt' }
        }
        const total = totalMs ?? (repeatMode === 'daily' ? 24 * 60 * 60 * 1000 : 0)
        const cd: Countdown = await tauriApi.createCountdown({
          name,
          repeatMode,
          endAt,
          totalMs: total,
          intervalMinutes: intervalMinutes ?? null,
        })
        return { ok: true, data: { id: cd.id, name: cd.name, end_at: cd.end_at } }
      }
      case 'add_todo': {
        const { title, priority = 0, dueAt = null } = call.args as {
          title: string
          priority?: 0 | 1 | 2
          dueAt?: number | null
        }
        if (!title) {
          return { ok: false, error: 'add_todo 需要 title' }
        }
        // 后端 create_todo 命令签名只接 title（不破坏 0.7.0 兼容），
        // priority 与 dueAt 通过 update_todo / schedule_todo 后置
        const todo: Todo = await tauriApi.createTodo(title)
        if (priority) {
          await tauriApi.updateTodo(todo.id, title, priority)
        }
        if (dueAt !== null && dueAt !== undefined) {
          await tauriApi.scheduleTodo(todo.id, dueAt, null)
        }
        return { ok: true, data: { id: todo.id, title: todo.title, priority, dueAt } }
      }
      case 'add_sticky': {
        const { slot, content } = call.args as { slot: 1 | 2; content: string }
        if (slot !== 1 && slot !== 2) {
          return { ok: false, error: 'add_sticky slot 必须是 1 或 2' }
        }
        if (typeof content !== 'string') {
          return { ok: false, error: 'add_sticky 需要 content 字符串' }
        }
        const s = await tauriApi.saveSticky(slot, content)
        return { ok: true, data: { slot: s.slot, content_length: s.content.length } }
      }
      case 'add_note': {
        const { title, content } = call.args as { title: string; content: string }
        if (typeof content !== 'string') {
          return { ok: false, error: 'add_note 需要 content 字符串' }
        }
        // 后端 create_note 只接 title（守住 0.7.0 兼容），通过 update_note 写入内容
        const t = title?.trim() ?? ''
        const note = await tauriApi.createNote(t || '无标题笔记')
        await tauriApi.updateNote(note.id, t || '无标题笔记', content)
        return { ok: true, data: { id: note.id, title: t || '无标题笔记' } }
      }
      case 'read_file': {
        const { path } = call.args as { path: string }
        if (!path) return { ok: false, error: 'read_file 需要 path' }
        // 走新建命令读文件（薄片 4 加）：底层走 store + 后端命令
        try {
          const content = await tauriApi.readTextFile(path)
          return { ok: true, data: { path, content, length: content.length } }
        } catch (e) {
          return { ok: false, error: `读取失败: ${String((e as Error)?.message ?? e)}` }
        }
      }
      default:
        return { ok: false, error: `未知工具: ${call.name}` }
    }
  } catch (e) {
    return { ok: false, error: String((e as Error)?.message ?? e) }
  }
}

/** 工具调用结果 → 序列化给 AI 看的简短文本（注入到下次消息） */
export function formatToolResult(result: ToolResult, name: string): string {
  if (result.ok) {
    return `[tool_result: ${name}] OK: ${JSON.stringify(result.data ?? {}, null, 0)}`
  }
  return `[tool_result: ${name}] FAIL: ${result.error ?? '未知错误'}`
}