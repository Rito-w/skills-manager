<script setup lang="ts">
import { ref } from "vue";
import type { IdeOption, ProjectConfig } from "../composables/types";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  visible: boolean;
  ideOptions: IdeOption[];
  projects: ProjectConfig[];
  initialIdeTargets?: string[];
  initialProjectIds?: string[];
}>();

const emit = defineEmits<{
  (e: "confirm", installTarget: "ide" | "project", targetIds: string[], projects: ProjectConfig[]): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();

const selectedIdeTargets = ref<string[]>([]);
const selectedProjectIds = ref<string[]>([]);

import { watch } from "vue";
watch(
  () => [props.visible, props.initialIdeTargets, props.initialProjectIds] as [boolean, string[] | undefined, string[] | undefined],
  ([visible, ideTargets, projectIds]) => {
    if (visible) {
      selectedIdeTargets.value = ideTargets ? [...ideTargets] : [];
      selectedProjectIds.value = projectIds ? [...projectIds] : [];
    } else {
      selectedIdeTargets.value = [];
      selectedProjectIds.value = [];
    }
  },
  { deep: true, immediate: true }
);

function toggleIdeTarget(ideId: string) {
  const index = selectedIdeTargets.value.indexOf(ideId);
  if (index === -1) {
    selectedIdeTargets.value.push(ideId);
  } else {
    selectedIdeTargets.value.splice(index, 1);
  }
}

function toggleProject(projectId: string) {
  const index = selectedProjectIds.value.indexOf(projectId);
  if (index === -1) {
    selectedProjectIds.value.push(projectId);
  } else {
    selectedProjectIds.value.splice(index, 1);
  }
}

function confirmInstallToIde() {
  emit("confirm", "ide", [...selectedIdeTargets.value], props.projects);
}

function confirmInstallToProject() {
  emit("confirm", "project", [...selectedProjectIds.value], props.projects);
}

function close() {
  selectedIdeTargets.value = [];
  selectedProjectIds.value = [];
  emit("cancel");
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div v-if="visible" class="modal-backdrop" @click.self="close">
        <div class="modal">
          <div class="modal-header">
            <h2 class="modal-title">{{ t("installModal.selectTargetTitle") }}</h2>
            <button class="modal-close" @click="close">×</button>
          </div>

          <div class="modal-body">
            <div class="two-columns">
              <!-- IDE Column -->
              <div class="column">
                <div class="column-header">
                  <h3 class="column-title">
                    <span class="icon">IDE</span>
                    {{ t("installModal.globalIde") }}
                  </h3>
                  <span class="count">{{ selectedIdeTargets.length }} / {{ ideOptions.length }}</span>
                </div>
                <div class="options-list">
                  <label
                    v-for="ide in ideOptions"
                    :key="ide.id"
                    class="option-item"
                    :class="{ selected: selectedIdeTargets.includes(ide.label) }"
                  >
                    <input
                      type="checkbox"
                      :checked="selectedIdeTargets.includes(ide.label)"
                      @change="toggleIdeTarget(ide.label)"
                    />
                    <span class="option-label">{{ ide.label }}</span>
                  </label>
                </div>
              </div>

              <!-- Project Column -->
              <div class="column">
                <div class="column-header">
                  <h3 class="column-title">
                    <span class="icon">Project</span>
                    {{ t("installModal.project") }}
                  </h3>
                  <span class="count">{{ selectedProjectIds.length }} / {{ projects.length }}</span>
                </div>
                <div v-if="projects.length === 0" class="empty-hint">
                  {{ t("installModal.noProjects") }}
                </div>
                <div v-else class="options-list">
                  <label
                    v-for="project in projects"
                    :key="project.id"
                    class="option-item project-item"
                    :class="{ selected: selectedProjectIds.includes(project.id) }"
                  >
                    <input
                      type="checkbox"
                      :checked="selectedProjectIds.includes(project.id)"
                      @change="toggleProject(project.id)"
                    />
                    <div class="option-content">
                      <span class="option-label">{{ project.name }}</span>
                      <span class="option-desc">{{ project.path }}</span>
                    </div>
                  </label>
                </div>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <button class="primary" @click="confirmInstallToIde">
              {{ t("installModal.installToIde") }}
            </button>
            <button class="primary" :disabled="projects.length === 0" @click="confirmInstallToProject">
              {{ t("installModal.installToProject") }}
            </button>
            <button class="ghost" @click="close">{{ t("installModal.cancel") }}</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: var(--color-overlay-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.modal {
  background: var(--color-panel-bg);
  border-radius: 12px;
  width: 100%;
  max-width: 900px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-panel-border);
}

.modal-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.modal-close {
  background: transparent;
  border: none;
  font-size: 28px;
  color: var(--color-muted);
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: background 0.2s ease;
}

.modal-close:hover {
  background: var(--color-tabs-bg);
  color: var(--color-text);
}

.modal-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 24px;
}

.two-columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.column {
  display: flex;
  flex-direction: column;
  min-height: 300px;
  max-height: 50vh;
  border: 1px solid var(--color-card-border);
  border-radius: 8px;
  overflow: hidden;
}

.column-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: var(--color-tabs-bg);
  border-bottom: 1px solid var(--color-card-border);
}

.column-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.column-title .icon {
  font-size: 18px;
}

.count {
  font-size: 13px;
  color: var(--color-muted);
  font-weight: 500;
}

.options-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.option-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s ease;
}

.option-item:hover {
  background: var(--color-tabs-bg);
}

.option-item input[type="checkbox"] {
  margin-top: 2px;
  cursor: pointer;
}

.option-label {
  font-size: 14px;
  font-weight: 500;
}

.project-item {
  flex-direction: column;
  gap: 4px;
}

.option-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.option-desc {
  font-size: 12px;
  opacity: 0.7;
  word-break: break-all;
}

.empty-hint {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-muted);
  font-size: 14px;
  padding: 40px 20px;
  text-align: center;
}

.modal-footer {
  display: flex;
  justify-content: center;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--color-panel-border);
}

.modal-footer button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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
  .two-columns {
    grid-template-columns: 1fr;
  }
  
  .column {
    max-height: 40vh;
  }
}
</style>
