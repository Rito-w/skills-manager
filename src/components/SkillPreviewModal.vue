<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { LocalSkill, LocalSkillPreview } from "../composables/types";

const { t } = useI18n();

const props = defineProps<{
  visible: boolean;
  skill: LocalSkill | null;
  preview: LocalSkillPreview | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const usedByText = computed(() => {
  if (!props.skill || props.skill.usedBy.length === 0) return "-";
  return props.skill.usedBy.join("?");
});

const descriptionText = computed(() => {
  const value = props.skill?.description?.trim();
  return value || t("local.previewEmptyDescription");
});

function close() {
  emit("close");
}
</script>

<template>
  <Transition name="modal-fade">
    <div v-if="visible" class="preview-backdrop" @click.self="close">
      <div class="preview-modal">
        <div class="preview-header">
          <div class="preview-heading">
            <div class="preview-badge">Skill</div>
            <h2 class="preview-title">{{ skill?.name ?? t("local.previewTitle") }}</h2>
            <p class="preview-description">{{ descriptionText }}</p>
          </div>
          <button class="preview-close" @click="close">?</button>
        </div>

        <div class="preview-body">
          <div class="preview-meta-grid">
            <div class="preview-meta-item">
              <div class="preview-meta-label">{{ t("local.previewUsedBy") }}</div>
              <div class="preview-meta-value">{{ usedByText }}</div>
            </div>
            <div class="preview-meta-item preview-meta-item-wide">
              <div class="preview-meta-label">{{ t("local.previewPath") }}</div>
              <div class="preview-meta-value preview-path">{{ skill?.path ?? "-" }}</div>
            </div>
          </div>

          <div class="preview-markdown">
            <div class="preview-markdown-header">
              <span class="preview-markdown-title">{{ t("local.previewSkillMdPath") }}</span>
              <span class="preview-markdown-path">{{ preview?.skillMdPath ?? "-" }}</span>
            </div>
            <div v-if="loading" class="preview-loading">{{ t("local.processing") }}</div>
            <pre v-else class="preview-markdown-content">{{ preview?.skillMdContent ?? "" }}</pre>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.preview-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.32);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 28px;
  z-index: 1000;
}

.preview-modal {
  width: min(1120px, 100%);
  max-height: min(88vh, 920px);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: var(--color-panel-bg);
  border: 1px solid var(--color-panel-border);
  border-radius: 28px;
  box-shadow: 0 24px 60px rgba(15, 23, 42, 0.18);
}

.preview-header {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding: 28px 32px 24px;
  border-bottom: 1px solid var(--color-card-border);
}

.preview-heading {
  min-width: 0;
}

.preview-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 56px;
  padding: 6px 12px;
  border-radius: 999px;
  background: var(--color-chip-bg);
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 14px;
}

.preview-title {
  margin: 0;
  font-size: 40px;
  line-height: 1.1;
}

.preview-description {
  margin: 12px 0 0;
  color: var(--color-muted);
  font-size: 16px;
}

.preview-close {
  border: none;
  background: transparent;
  color: var(--color-muted);
  font-size: 36px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
}

.preview-body {
  padding: 24px 32px 32px;
  overflow: auto;
}

.preview-meta-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}

.preview-meta-item {
  padding: 16px 18px;
  border: 1px solid var(--color-card-border);
  border-radius: 18px;
  background: var(--color-card-bg);
}

.preview-meta-item-wide {
  grid-column: 1 / -1;
}

.preview-meta-label,
.preview-markdown-title {
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.preview-meta-value {
  margin-top: 8px;
  font-size: 15px;
  line-height: 1.6;
  word-break: break-word;
}

.preview-path,
.preview-markdown-path {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
}

.preview-markdown {
  border: 1px solid var(--color-card-border);
  border-radius: 22px;
  overflow: hidden;
  background: var(--color-panel-bg);
}

.preview-markdown-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 18px 24px;
  border-bottom: 1px solid var(--color-card-border);
  background: var(--color-card-bg);
}

.preview-markdown-path {
  color: var(--color-text);
  font-size: 13px;
  background: var(--color-chip-bg);
  border-radius: 10px;
  padding: 8px 12px;
  max-width: 70%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-loading,
.preview-markdown-content {
  margin: 0;
  padding: 24px;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: 15px;
  line-height: 1.8;
}

.preview-markdown-content {
  white-space: pre-wrap;
  word-break: break-word;
  overflow: auto;
}

.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}

@media (max-width: 768px) {
  .preview-backdrop {
    padding: 16px;
  }

  .preview-modal {
    max-height: 92vh;
    border-radius: 20px;
  }

  .preview-header,
  .preview-body {
    padding-left: 20px;
    padding-right: 20px;
  }

  .preview-title {
    font-size: 28px;
  }

  .preview-meta-grid {
    grid-template-columns: 1fr;
  }

  .preview-markdown-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .preview-markdown-path {
    max-width: 100%;
  }
}
</style>
