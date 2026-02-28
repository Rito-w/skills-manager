#!/bin/bash
# spawn-agent.sh - 启动编码代理
# 用法: ./spawn-agent.sh <task-id> <agent-type> <branch> <prompt>

set -e

TASK_ID="$1"
AGENT_TYPE="$2"
BRANCH="$3"
PROMPT="$4"
WORKTREE_BASE="/tmp/skills-manager-worktrees"
PROJECT_DIR="/Volumes/myDisk/workplace/skills-manager"

# 创建 worktree 目录
mkdir -p "$WORKTREE_BASE"
WORKTREE_DIR="$WORKTREE_BASE/$TASK_ID"

# 创建 worktree
cd "$PROJECT_DIR"
git worktree add "$WORKTREE_DIR" -b "$BRANCH" main 2>/dev/null || git worktree add "$WORKTREE_DIR" -b "$BRANCH" origin/main

# 安装依赖
cd "$WORKTREE_DIR"
pnpm install --silent

# 根据代理类型启动
case "$AGENT_TYPE" in
  codex)
    tmux new-session -d -s "agent-$TASK_ID" -c "$WORKTREE_DIR" \
      "codex exec --full-auto '$PROMPT

When completely finished, run: openclaw system event --text \"Done: $TASK_ID\" --mode now'"
    ;;
  claude)
    tmux new-session -d -s "agent-$TASK_ID" -c "$WORKTREE_DIR" \
      "claude --dangerously-skip-permissions '$PROMPT

When completely finished, run: openclaw system event --text \"Done: $TASK_ID\" --mode now'"
    ;;
  *)
    echo "Unknown agent type: $AGENT_TYPE"
    exit 1
    ;;
esac

echo "Agent started: $AGENT_TYPE in $WORKTREE_DIR"
echo "Tmux session: agent-$TASK_ID"
echo "Worktree: $WORKTREE_DIR"
