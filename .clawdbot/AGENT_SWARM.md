# Skills Manager Agent Swarm 配置

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                     OpenClaw (编排层)                        │
│  - 持有项目上下文、决策历史、业务知识                          │
│  - 分配任务给合适的代理                                       │
│  - 监控进度、处理失败、合并结果                                │
└──────────────────────┬──────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│   Codex     │ │ Claude Code │ │   Gemini    │
│  (后端逻辑)  │ │  (前端/UI)  │ │  (设计审查)  │
└─────────────┘ └─────────────┘ └─────────────┘
```

## 代理分工

### Codex - 后端专家 (90% 任务)
- **擅长**: Rust 逻辑、复杂 bug、多文件重构、API 设计
- **特点**: 深度推理，处理复杂逻辑能力强
- **使用场景**:
  - lib.rs 修改
  - Tauri commands 实现
  - 文件系统操作
  - 错误处理完善

### Claude Code - 前端专家
- **擅长**: Vue 组件、UI 实现、TypeScript、快速迭代
- **特点**: 速度快，前端代码质量高
- **使用场景**:
  - Vue 组件开发
  - composables 编写
  - 样式调整
  - 前端 bug 修复

### Gemini - 设计审查 (可选)
- **擅长**: UI 设计、代码审查、安全审计
- **使用场景**:
  - UI 设计稿生成
  - PR 审查
  - 安全问题检测

## 任务类型映射

| 任务类型 | 首选代理 | 工作目录 |
|---------|---------|---------|
| Rust 后端功能 | Codex | worktree |
| Vue 前端功能 | Claude Code | worktree |
| 全栈功能 | Codex (后端) + Claude Code (前端) | 分开 worktree |
| Bug 修复 | Codex (后端) / Claude Code (前端) | worktree |
| 重构 | Codex | worktree |
| 文档更新 | Claude Code | main |
| 小改动 (<5行) | 直接 edit | - |

## Worktree 策略

每个独立任务创建独立 worktree，避免冲突：

```bash
# 创建 worktree
git worktree add /tmp/skills-manager-worktrees/feat-xxx -b feat/xxx main

# 任务完成后
git worktree remove /tmp/skills-manager-worktrees/feat-xxx
```

## 监控频率

- Heartbeat: 每 30 分钟检查活跃任务
- 任务完成时: 主动通知（通过 `openclaw system event`）

## 完成定义 (Definition of Done)

PR 视为完成需满足：
1. ✅ PR 已创建
2. ✅ CI 通过 (如果有)
3. ✅ TypeScript 类型检查通过 (`vue-tsc --noEmit`)
4. ✅ Build 成功 (`pnpm build`)
5. ✅ 至少一个 AI 审查通过

## 目录结构

```
.clawdbot/
├── active-tasks.json      # 任务注册表
├── AGENT_SWARM.md         # 本文档
├── scripts/
│   ├── spawn-agent.sh     # 启动代理
│   ├── check-agents.sh    # 检查状态
│   └── cleanup.sh         # 清理 worktree
└── logs/                  # 代理日志 (可选)
```
