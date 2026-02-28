#!/bin/bash
# check-agents.sh - 检查所有活跃代理状态

TASKS_FILE="/Volumes/myDisk/workplace/skills-manager/.clawdbot/active-tasks.json"

# 检查 tmux 会话
echo "=== Active tmux sessions ==="
tmux list-sessions 2>/dev/null | grep "agent-" || echo "No active agent sessions"

# 检查 worktrees
echo ""
echo "=== Active worktrees ==="
git worktree list | grep "worktrees" || echo "No active worktrees"

# 检查任务注册表
echo ""
echo "=== Registered tasks ==="
cat "$TASKS_FILE" | jq '.tasks[]' 2>/dev/null || echo "No tasks registered"
