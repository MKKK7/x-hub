// 工具调用块解析（Q6）
//
// AI 在回复里输出 `<tool_call>{json}</tool_call>` 块表达工具调用意图。
// 本模块用正则提取所有块、尝试 JSON.parse、收集 parse 失败的错误。

import type { ToolCall } from './tools'

/** 单个解析结果：可能是成功也可能失败（失败时 error 字段说明） */
export type ParsedToolCall =
  | { ok: true; call: ToolCall; raw: string; index: number }
  | { ok: false; error: string; raw: string; index: number }

/** 从 AI 回复文本中提取所有 <tool_call>...</tool_call> 块（按出现顺序）
 */
export function parseToolCalls(text: string): ParsedToolCall[] {
  const results: ParsedToolCall[] = []
  // 非贪婪匹配 + dot-all (s 标志跨行)；工具块里可能有换行
  const regex = /<tool_call>([\s\S]*?)<\/tool_call>/g
  let m: RegExpExecArray | null
  let index = 0
  while ((m = regex.exec(text)) !== null) {
    const raw = m[1].trim()
    try {
      const obj = JSON.parse(raw)
      if (typeof obj !== 'object' || obj === null) {
        results.push({
          ok: false,
          error: '工具块不是 JSON 对象',
          raw,
          index: index++,
        })
        continue
      }
      const name = typeof obj.name === 'string' ? obj.name : ''
      const args = typeof obj.args === 'object' && obj.args !== null ? obj.args : {}
      if (!name) {
        results.push({
          ok: false,
          error: '工具块缺少 name 字段',
          raw,
          index: index++,
        })
        continue
      }
      results.push({
        ok: true,
        call: { name, args: args as Record<string, unknown> },
        raw,
        index: index++,
      })
    } catch (e) {
      results.push({
        ok: false,
        error: `JSON.parse 失败: ${String((e as Error)?.message ?? e)}`,
        raw,
        index: index++,
      })
    }
  }
  return results
}

/** 移除所有 tool_call 块，返回 AI 想给用户看的纯文本回复 */
export function stripToolCalls(text: string): string {
  return text.replace(/<tool_call>[\s\S]*?<\/tool_call>/g, '').trim()
}