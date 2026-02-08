<script setup lang="ts">
import { useToast } from "../composables/useToast";

const { toasts, remove } = useToast();
</script>

<template>
  <div class="toast-container">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast"
        :class="toast.type"
        @click="remove(toast.id)"
      >
        <span>{{ toast.content }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  gap: 10px;
  z-index: 1000;
  pointer-events: none;
}

.toast {
  pointer-events: auto;
  padding: 10px 16px;
  border-radius: 999px;
  font-size: 14px;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 200px;
  max-width: 90vw;
  text-align: center;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.toast.success {
  background: var(--color-success-bg);
  color: var(--color-success-text);
  border: 1px solid var(--color-success-border);
}

.toast.error {
  background: var(--color-error-bg);
  color: var(--color-error-text);
  border: 1px solid var(--color-error-border);
}

.toast.info {
  background: var(--color-panel-bg);
  color: var(--color-text);
  border: 1px solid var(--color-panel-border);
}

/* Transitions */
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

.toast-leave-active {
  position: absolute;
}
</style>
