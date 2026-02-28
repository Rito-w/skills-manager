#!/bin/bash
# cleanup.sh - 清理已完成的 worktree 和分支

WORKTREE_BASE="/tmp/skills-manager-worktrees"
PROJECT_DIR="/Volumes/myDisk/workplace/skills-manager"

cd "$PROJECT_DIR"

# 列出已完成的 worktrees
echo "Checking for orphaned worktrees..."

for dir in "$WORKTREE_BASE"/*; do
  if [ -d "$dir" ]; then
    task_name=$(basename "$dir")
    tmux_session="agent-$task_name"
    
    # 检查 tmux 会话是否还在运行
    if ! tmux has-session -t "$tmux_session" 2>/dev/null; then
      echo "Cleaning up: $task_name"
      
      # 获取分支名
      branch=$(git -C "$dir" branch --show-current 2>/dev/null)
      
      # 移除 worktree
      git worktree remove "$dir" --force 2>/dev/null || rm -rf "$dir"
      
      # 可选：删除分支 (保留已推送的)
      # git branch -D "$branch" 2>/dev/null
    fi
  fi
done

echo "Cleanup complete."
