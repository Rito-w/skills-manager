<script setup lang="ts">
import type { DownloadTask } from "../composables/types";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  tasks: DownloadTask[];
}>();

defineEmits<{
  (e: "retry", taskId: string): void;
  (e: "remove", taskId: string): void;
}>();
</script>

<template>
  <div v-if="tasks.length > 0" class="download-queue">
    <div class="queue-title">{{ t("download.title") }}</div>
    <div class="queue-list">
      <div v-for="task in tasks" :key="task.id" class="queue-item">
        <div class="task-info">
          <span class="task-name">{{ task.name }}</span>
          <span class="task-status" :class="task.status">
            <template v-if="task.status === 'pending'">{{ t("download.pending") }}</template>
            <template v-else-if="task.status === 'downloading'">{{ t("download.downloading") }}</template>
            <template v-else-if="task.status === 'done'">{{ t("download.done") }}</template>
            <template v-else-if="task.status === 'error'">{{ task.error || t("download.error") }}</template>
          </span>
        </div>
        <div class="task-actions">
          <button v-if="task.status === 'error'" class="ghost small" @click="$emit('retry', task.id)">
            {{ t("download.retry") }}
          </button>
          <button v-if="task.status === 'error' || task.status === 'pending'" class="ghost small" @click="$emit('remove', task.id)">
            ×
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-queue {
  margin-bottom: 16px;
  padding: 12px;
  background: var(--surface-1);
  border-radius: 8px;
  border: 1px solid var(--border);
}

.queue-title {
  font-size: 0.9em;
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--text-1);
}

.queue-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.queue-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--surface-2);
  border-radius: 6px;
}

.task-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.task-name {
  font-weight: 500;
  font-size: 0.9em;
}

.task-status {
  font-size: 0.8em;
  color: var(--text-2);
}

.task-status.downloading {
  color: var(--primary);
}

.task-status.done {
  color: var(--success, #22c55e);
}

.task-status.error {
  color: var(--error, #ef4444);
}

.task-actions {
  display: flex;
  gap: 4px;
}

.task-actions button.small {
  padding: 4px 8px;
  font-size: 0.8em;
}
</style>
