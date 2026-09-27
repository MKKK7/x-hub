# SPEC — Q3 便签归档（sticky archives）

**版本**：v0.7.0-fork Q3
**日期**：2026-09-27
**范围**：仅 Q3（Q1/Q2/Q5/Q6 各自独立 spec，本文件不含）

---

## 1. Objective（目标与用户）

**目标**：让工作台便签（slot 1/2）与脱离浮窗便签（detached）的「覆盖即丢」现象被软归档替代——被覆盖/被关闭销毁的内容自动入归档表，用户可随时查看、恢复、彻底删除。

**用户**：x-hub 个人用户。把工作台便签当作临时笔记/草稿/速记用的用户，偶尔会因自动覆盖或窗口误关丢失内容，希望保留痕迹并能找回。

**不做**：
- 不把归档变成「核心笔记」（Q4 已砍）
- 不加标签/搜索/导出 markdown（后续迭代）
- 不云同步/不上传（守住本地优先原则）
- 不动原 `stickies` / `detached_stickies` 表结构（CLAUDE.md 约束：守住 0.7.0 兼容）

---

## 2. Commands（新增命令清单）

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `archive_sticky(source, source_id?, content, reason)` | source: `slot1`/`slot2`/`detached`；source_id: number \| null（slot 时为 null，detached 时为 detached_stickies.id）；content: string；reason: `user`/`auto_replace`/`auto_destroy` | `{ id: number }` | 创建一条归档。前端写入时调用。`auto_replace`/`auto_destroy` 由 Rust 端在覆盖/销毁时自动调；`user` 由前端 ⋯ 菜单触发。 |
| `list_sticky_archives(source?, limit?, offset?)` | source 可选过滤；limit 默认 50；offset 默认 0 | `StickyArchive[]` | 列归档。按 `archived_at DESC` 排序。 |
| `get_sticky_archive(id)` | `id: number` | `StickyArchive` | 取单条。恢复/查看用。 |
| `restore_sticky_archive(id)` | `id: number` | `{ target: 'slot1' \| 'slot2' \| 'detached_created', content: string }` | 按 source 还原。slot → 写回原 slot（如该 slot 已被占则返回错误 `SLOT_OCCUPIED`，由前端提示用户先归档当前内容）；detached → 创建新 detached sticky 并返回新 id。 |
| `delete_sticky_archive(id)` | `id: number` | `void` | 彻底删除（用户主动）。自动归档不出现在删除候选。 |

**模型**（在 `src-tauri/src/models.rs` 加，与 `Sticky` / `DetachedSticky` 并列）：
```rust
pub struct StickyArchive {
    pub id: i64,
    pub source: String,          // "slot1" | "slot2" | "detached"
    pub source_id: Option<i64>,  // detached 时为 detached_stickies.id；slot 时 None
    pub content: String,
    pub archived_at: i64,        // ms epoch
    pub reason: String,          // "user" | "auto_replace" | "auto_destroy"
}
```

**前端封装**（`src/api/tauri.ts`）：`tauriApi.archiveSticky` / `listStickyArchives` / `getStickyArchive` / `restoreStickyArchive` / `deleteStickyArchive`。

---

## 3. Project structure（新增/修改文件）

### 新增
- `src-tauri/src/repo/sticky_archive.rs` — 数据访问层（CRUD + 排序 + source 过滤）。含 `mod tests` 单元测试（沿用 `repo/snippet.rs::tests` / `repo/chat.rs::tests` 模式）。
- `src-tauri/src/repo/mod.rs` — 加 `pub mod sticky_archive;`
- `src/components/StickyArchiveView.vue` — 独立视图（侧栏入口对应）。
- `src/components/StickyArchiveCard.vue` — 工作台可选部件（最近 5 条 + 点开进视图）。
- `src/components/StickyArchiveItem.vue` — 列表项（行内恢复/删除按钮 + 来源徽标 + 相对时间 + 内容预览）。
- `src/components/StickyArchiveRestoreDialog.vue` — 恢复时的 slot 占用确认（slot1/slot2 占用时弹）+ detached 创建新窗口提示。

### 修改（守住兼容原则：不改签名）
- `src-tauri/src/db.rs` — 加 `sticky_archives` 表 CREATE TABLE（写在初始化 SQL 末尾，幂等 `CREATE TABLE IF NOT EXISTS`）+ `migrate()` 现有路径加一条 `ALTER TABLE` 兼容兜底（如历史库已有同名表则忽略 schema mismatch）。
- `src-tauri/src/commands.rs` — 加 5 个 `#[tauri::command]` 入口，签名严格按 Commands 节。
- `src-tauri/src/lib.rs` — `invoke_handler!` 注册 5 个新命令。
- `src/api/tauri.ts` — 加 `StickyArchive` 类型 + 5 个 `tauriApi` 封装。
- `src/components/StickyCard.vue` — 右上 ⋯ 菜单加「归档」项 + 「归档历史」项（脱离悬浮窗按钮左边）。
- `src/components/DetachedStickyWindow.vue` — 关闭销毁时调 `archive_sticky`（reason=auto_destroy）；保留原 detach 行为不变。
- `src/stores/workbench.ts` — 加 stickyArchives 状态 + load 入口 + restore/delete action。
- `src/composables/useDashboardLayout.ts` — `DASH_MODULES` 注册 `sticky_archive` 部件（参考 `todo_overview` 写法）。
- `src/index/index.vue` — 侧栏 navigation 数组加「便签归档」入口（与速记/速达同级）；activeView 处理新视图。
- `src/App.vue` — 已有 default else 分支会渲染 `Index`，无需新增 label（独立视图走主窗 Index 内的视图切换，不开新窗口）。

---

## 4. Code style

完全沿用项目现有约定（详见 `AGENTS.md` 与 `CLAUDE.md`），不新增任何规范。重点：

- **不可变模式**：前端 store 更新走 readonly + 重新赋值，不原地修改（CLAUDE.md §3）
- **文件 ≤400 行 / 函数 ≤50 行 / 嵌套 ≤4 层**
- **Rust 命名**：snake_case 函数 + PascalCase 结构体 + UPPER_SNAKE_CASE 常量
- **TypeScript 命名**：camelCase 变量/函数 + PascalCase 类型 + UPPER_SNAKE_CASE 常量
- **Vue**：`<script setup>` 强制；scoped CSS；`flex`/`grid` 布局
- **错误处理**：异步必须 try-catch，禁空 catch
- **中文注释**：公共 API 和复杂逻辑必须有
- **三轴主题**：组件内仅用 CSS 变量（`var(--xxx)`），不写死色值
- **AGENTS.md 约定 41（运行期禁 build/destroy）**：本功能不新增浮窗，全在主窗内渲染，零窗口生命周期风险
- **AGENTS.md 约定 64（label 路由用 webview label）**：本功能无新 webview

---

## 5. Testing strategy（TDD 流程）

按 `agent-skills:test-driven-development` skill 执行：RED → GREEN → IMPROVE 三步循环。**目标覆盖率 ≥ 80%**（CLAUDE.md 全局约束）。

### 测试层级

| 层级 | 范围 | 文件位置 | 工具 |
|---|---|---|---|
| 单元（Rust） | `repo/sticky_archive.rs` CRUD + 排序 + 过滤 | `src-tauri/src/repo/sticky_archive.rs::tests` 模块 | `cargo test`（必须走 `npm run tauri:test` 包装脚本——AGENTS.md 注意事项） |
| 单元（TS） | `tauriApi` 封装类型正确性 | 暂不写（与已有 `tauri.ts` 一致风格，由 `vue-tsc` 类型检查兜底） | — |
| 集成 | StickyCard 写新内容触发 `archive_sticky`（reason=auto_replace） | `src/components/StickyCard.vue` 人工 + 现有 `tests/security.test.mjs` | 浏览器预览 / 实机 |
| 端到端 | 覆盖 → 归档列表可见 → 恢复 → 内容回 slot | 实机触发（CLAUDE.md 全局约束） | 实机 |

### 必写单元测试用例（repo/sticky_archive.rs::tests）
1. `create_with_user_reason` — 插一条 user 归档，验证字段。
2. `create_with_auto_replace_reason` — 插一条 auto_replace。
3. `list_orders_by_archived_at_desc` — 插 3 条不同时刻，list 倒序。
4. `list_filter_by_source` — slot1 + slot2 + detached 混合，filter source=slot1 只返 slot1。
5. `restore_to_slot_writes_content` — 恢复一条 slot1 归档，stickies.slot1.content 等于原 content。
6. `restore_to_slot_occupied_returns_error` — slot1 已有内容，恢复时返 SLOT_OCCUPIED。
7. `restore_to_detached_creates_new_row` — 恢复 detached 归档，detached_stickies 表新增一行（不删原归档，源数据保留）。
8. `delete_purges_row` — delete 后 get 返回 None，list 不含。
9. `archived_at_uses_ms_epoch` — 验证时间字段为 ms epoch（不是 s）。

### TDD 顺序（薄片切分）
按 incremental-implementation 切 4 个薄片：
- **薄片 1**：Rust `sticky_archives` 表 + repo 增删查（仅 9 个单元测试通过即可提交）
- **薄片 2**：5 个 Tauri 命令 + 前端 tauriApi 封装 + 类型
- **薄片 3**：StickyCard 写新内容触发自动归档（auto_replace）+ DetachedStickyWindow 关闭触发自动归档（auto_destroy）
- **薄片 4**：UI 三件套（StickyArchiveCard / StickyArchiveView / StickyArchiveRestoreDialog）+ 侧栏入口 + 概览部件注册 + 手动归档入口

每薄片完成后：
- 跑 `cargo test`（`npm run tauri:test`）+ `npm run build`（vue-tsc + vite）
- 增量 commit（conventional commits，中文描述）
- 进入下一薄片前由 code-reviewer 走 review（`agent-skills:review` skill）

---

## 6. Boundaries（边界）

### Always（始终做）
- **保持 0.7.0 兼容**：所有改动走新文件 / 新增表 / 新增命令；不改 `stickies` / `detached_stickies` 表结构；不改上游现有函数签名。
- **新命令必入 `capabilities/default.json`**：本功能 5 个新命令均为应用自身命令（`#[tauri::command]`），不经 ACL 门控，但仍按现有模式登记。
- **新工作台部件必入 `useDashboardLayout.ts::DASH_MODULES`**。
- **Rust 命令写日志**：所有命令入口记录成功/失败（沿用 `commands.rs` 现有 logging 模式）。
- **回归测试**：薄片 1 完成后跑一次 `cargo test` 全套（确认未踩坏已有测试）。
- **CLAUDE.md 全局约束**：22 条铁律全文适用，特别注意：
  - 约束 11：后端单独写盘字段四件套（本功能无新增配置字段，但 `reason` 枚举值若将来需要加配置项则必须四件套同步）
  - 约束 12：路径给外部进程过 `paths::simplify_path`（本功能无跨进程路径）
  - 约束 14：新增浮窗 label 四处同步（本功能**不开新浮窗**，零风险）
  - 约束 4：主窗句柄走 `crate::main_window(app)`（commands 实现如需主窗句柄必须用此）
- **AGENTS.md 关键约定**：62 条编号约定，按需查阅。
- **CLAUDE.md 约束 3（运行期禁 build/destroy）**：本功能无运行期建窗。
- **CLAUDE.md 约束 6（:global() 整条选择器同括号）**：新增 scoped CSS 时严格遵守。
- **CLAUDE.md 约束 16（App.vue 路由用 webview label）**：本功能不新增 webview。

### Ask first（动手前必问）
- 任何**新增配置字段**（如未来要加「保留期限」「归档上限」等）。
- 任何**改原表 schema** 的方案（即便兼容性 ALTER 也要先确认范围）。
- 任何**跨进程数据导出**（如要支持导出 markdown 文件）。
- 任何**修改上游现有命令行为**（即便不改签名也可能踩到调用方预期）。

### Never（绝不）
- 不删 `stickies` / `detached_stickies` 表数据（仅用软归档）。
- 不改原命令函数签名（包括 `set_sticky_content` / `create_detached_sticky` 等）。
- 不把归档数据上传云端（守住本地优先原则）。
- 不在归档表上做「过期清理」（保留期=永久，唯一移除路径=用户手动 delete）。
- 不把 `auto_replace` / `auto_destroy` 暴露给前端作为 reason 入参（这两种 reason 仅由 Rust 端自动调用时设，前端只能传 `user`）。
- 不在归档视图展示「自动归档」之外的其他原始便签内容（防止越权读未归档便签）。
- 不用原生 `<select>`（用 `AppSelect`，CLAUDE.md 约束 10）。
- 不用 `<style scoped>` 外的 `:global(前缀) 后代`（CLAUDE.md 约束 6）。

---

## 验收标准（Definition of Done）

- [ ] `npm run build` 通过（vue-tsc + vite）
- [ ] `npm run tauri:test` 通过（Rust 单元测试，含 9 个新增 sticky_archive 测试）
- [ ] 覆盖率：repo/sticky_archive.rs 行覆盖 ≥ 80%
- [ ] 实机验证（CLAUDE.md 约束：v0.5.3/v0.5.4 事故后规定）：
  - StickyCard 写新内容 → 关闭态查 archive 列表可见旧内容 + reason=auto_replace
  - DetachedStickyWindow 关闭销毁 → archive 列表可见 + reason=auto_destroy
  - 手动点 ⋯ → 归档 → 列表可见 + reason=user
  - 点恢复 → slot 内容回写（slot 占用时弹 SLOT_OCCUPIED 确认弹窗）
  - 点彻底删除 → 行消失
  - 侧栏「便签归档」入口可点击进独立视图
  - 工作台 StickyArchiveCard 部件可见最近 5 条
- [ ] 代码 review（`agent-skills:review` skill 五维评估：无 CRITICAL/HIGH）
- [ ] 代码精简（`agent-skills:code-simplify` skill：保持行为不变的前提下缩减复杂度）
- [ ] 增量 commit 历史干净（薄片切分，每个 commit 一件事）
- [ ] docs/ 不入库（CLAUDE.md：docs/ 在 .gitignore 内，仅本地）
- [ ] 没有动 AGENTS.md / DESIGN.md / README.md / CONTRIBUTING.md / PRODUCT.md / CONTEXT.md / RELEASE_NOTES.md（守住上游兼容）
- [ ] 没有动 .gitignore（CLAUDE.md 已有规则覆盖 .claude/ CLAUDE.md docs/）

---

## 待 user 最终确认

**全部敲定，请批准进入 plan（拆任务）+ build 阶段。**

如需调整哪条，告诉我具体改什么。