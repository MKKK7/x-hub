# Q3 便签归档 — 任务跟踪

**分支**：`local/customize`
**计划文件**：`C:\Users\MK\.claude\plans\drifting-hatching-treasure.md`
**SPEC**：`SPEC.md`

## 薄片进度

- [x] **薄片 0** — 预备：gitignore + SPEC 落库
- [x] **薄片 1** — Rust 后端数据层（repo + 模型 + 表 + 9 单元测试）
- [x] **薄片 2** — Rust 命令 + 前端 tauriApi 封装
- [x] **薄片 3** — 自动归档触发（save_sticky / delete_detached_sticky 钩子）
- [x] **薄片 4** — UI 三件套 + 侧栏入口 + 概览部件 + 手动归档

## 每薄片状态

### 薄片 0 — 预备
- 状态：✅ 完成
- 提交：`c74d583`

### 薄片 1
- 状态：✅ 完成
- 提交：`e592503`

### 薄片 2
- 状态：✅ 完成
- 提交：`9852b79`

### 薄片 3
- 状态：✅ 完成
- 提交：`ef4875f`

### 薄片 4
- 状态：✅ 完成
- 提交：`eefdbd0`

## 已知限制

- **cargo test 无法在本机运行**：本机 Rust 1.97.1 + mingw-w64 16.2.0 有已知链接器 bug
  （`export ordinal too large`），与 Q3 改动无关。`cargo check` 通过 = 代码类型/语法正确。
  单元测试待用户在 CI 环境（windows-latest 默认 msvc 工具链）验证。
- **实机触发验证未做**：Q3 涉及前端运行时行为（auto_replace / auto_destroy 触发、
  detached 关闭路径），按 CLAUDE.md 全局约束需实机测试。建议用户回来后:
  1. 干净启动 → 写新便签 → 关闭 detached → 查 DB 验证 sticky_archives 表
  2. 点 ⋯ → 归档 → 侧栏「归档」→ 列表可见
  3. 恢复 slot 归档 → 内容回写