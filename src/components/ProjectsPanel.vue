<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { ProjectConfig, LocalSkill, IdeOption } from "../composables/types";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = defineProps<{
  projects: ProjectConfig[];
  selectedProjectId: string | null;
  localSkills: LocalSkill[];
  ideOptions: IdeOption[];
  localLoading: boolean;
}>();

const emit = defineEmits<{
  (e: "addProject"): void;
  (e: "removeProject", projectId: string): void;
  (e: "selectProject", projectId: string | null): void;
  (e: "configureProject", projectId: string): void;
  (e: "linkSkills", projectId: string): void;
  (e: "jumpToLocalSkill", skillName: string): void;
  (e: "unmanagedClick", skill: LocalSkill): void;
}>();

function handleAddProject() {
  emit("addProject");
}

function handleRemoveProject(projectId: string) {
  emit("removeProject", projectId);
}

function handleSelectProject(projectId: string) {
  emit("selectProject", projectId === props.selectedProjectId ? null : projectId);
}

function handleConfigureProject(projectId: string) {
  emit("configureProject", projectId);
}

function handleLinkSkills(projectId: string) {
  emit("linkSkills", projectId);
}

async function handleOpenDirectory(project: ProjectConfig) {
  try {
    await revealItemInDir(project.path);
  } catch (err) {
    console.error("Failed to open directory:", err);
  }
}

function buildIdeBadgeList(project: ProjectConfig) {
  return project.ideTargets.map((ideLabel) => ({
    label: ideLabel,
    active: true
  }));
}

function isManagedSkill(skill: LocalSkill) {
  return props.localSkills.some((ls: LocalSkill) => ls.name === skill.name);
}

const projectSkillsMap = ref<Record<string, LocalSkill[]>>({});
const collapsedGroups = ref<Record<string, boolean>>({});

interface SkillGroup {
  dir: string;
  skills: LocalSkill[];
}

function getGroupedSkills(skills: LocalSkill[]): SkillGroup[] {
  const map = new Map<string, LocalSkill[]>();
  for (const skill of skills) {
    const dir = skill.ide || 'Unknown Directory';
    if (!map.has(dir)) {
      map.set(dir, []);
    }
    map.get(dir)!.push(skill);
  }
  return Array.from(map.entries()).map(([dir, curSkills]) => ({ dir, skills: curSkills }));
}

function toggleGroup(projectId: string, dir: string) {
  const key = `${projectId}:${dir}`;
  collapsedGroups.value[key] = !collapsedGroups.value[key];
}

async function openSkillDir(projectPath: string, dir: string) {
  try {
    const separator = projectPath.includes('\\') ? '\\' : '/';
    const cleanDir = dir.replace(/\//g, separator);
    const suffix = projectPath.endsWith(separator) ? cleanDir : `${separator}${cleanDir}`;
    const fullPath = `${projectPath}${suffix}`;
    await revealItemInDir(fullPath);
  } catch (err) {
    console.error("Failed to open skill directory:", err);
  }
}

async function loadProjectSkills() {
  for (const project of props.projects) {
    try {
      const skills = await invoke<LocalSkill[]>("scan_project_skills", {
        request: { 
          projectDir: project.path,
          ideDirs: props.ideOptions.map((ide: IdeOption) => ({
            label: ide.label,
            relativeDir: ide.projectDir || ide.globalDir
          }))
        }
      });
      projectSkillsMap.value[project.id] = skills;
    } catch (e) {
      console.error("Failed to load skills for project", project.path, e);
      projectSkillsMap.value[project.id] = [];
    }
  }
}

watch(
  () => [props.projects, props.localSkills],
  () => {
    loadProjectSkills();
  },
  { deep: true, immediate: true }
);

</script>

<template>
  <section class="panel">
    <div class="panel-header">
      <div class="panel-title">{{ t("projects.title") }}</div>
      <button class="primary" @click="handleAddProject">
        {{ t("projects.add") }}
      </button>
    </div>
    <div class="hint">{{ t("projects.hint") }}</div>

    <div v-if="projects.length === 0" class="hint">{{ t("projects.emptyHint") }}</div>

    <div v-else class="project-list">
      <div
        v-for="project in projects"
        :key="project.id"
        class="project-item"
        :class="{ selected: selectedProjectId === project.id }"
      >
        <div class="project-header">
          <div class="project-info">
            <div class="project-name">{{ project.name }}</div>
            <div class="project-path">{{ project.path }}</div>
          </div>
          <div class="project-actions">
            <button
              class="ghost small"
              @click="handleSelectProject(project.id)"
            >
              {{ selectedProjectId === project.id ? t("projects.deselect") : t("projects.select") }}
            </button>
            <button
              class="ghost small"
              @click="handleConfigureProject(project.id)"
            >
              {{ t("projects.configure") }}
            </button>
            <button
              class="ghost small"
              @click="handleOpenDirectory(project)"
            >
              {{ t("projects.openDirectory") }}
            </button>
            <button
              class="primary small"
              :disabled="localLoading"
              @click="handleLinkSkills(project.id)"
            >
              {{ t("projects.linkSkills") }}
            </button>
            <button
              class="ghost danger small"
              @click="handleRemoveProject(project.id)"
            >
              {{ t("projects.remove") }}
            </button>
          </div>
        </div>
        <div class="project-meta">
          <span class="meta-item">
            {{ t("projects.ideTargets", { count: project.ideTargets.length }) }}
          </span>
          <span v-if="project.detectedIdeDirs.length > 0" class="meta-item">
            {{ t("projects.detected", { count: project.detectedIdeDirs.length }) }}
          </span>
        </div>
        <div class="ide-badges">
          <span
            v-for="badge in buildIdeBadgeList(project)"
            :key="badge.label"
            class="ide-badge"
            :class="{ active: badge.active }"
          >
            {{ badge.label }}
          </span>
        </div>
        <div v-if="projectSkillsMap[project.id]?.length > 0" class="project-skills">
          <div class="skills-title">{{ t("projects.installedSkills") || 'Installed Skills' }}:</div>
          <div class="skill-groups">
            <div 
              v-for="group in getGroupedSkills(projectSkillsMap[project.id])" 
              :key="group.dir" 
              class="skill-group"
            >
              <div class="skill-group-header" @click="toggleGroup(project.id, group.dir)">
                <div class="skill-group-title">
                  <span class="fold-icon">
                    <svg v-if="collapsedGroups[`${project.id}:${group.dir}`]" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
                    <svg v-else xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
                  </span>
                  <span class="dir-name">{{ group.dir }}</span>
                  <span class="skill-count">({{ group.skills.length }})</span>
                </div>
                <button class="icon-btn jump-btn" @click.stop="openSkillDir(project.path, group.dir)" title="Open Directory">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path><polyline points="15 3 21 3 21 9"></polyline><line x1="10" y1="14" x2="21" y2="3"></line></svg>
                </button>
              </div>
              <div class="skill-chips" v-show="!collapsedGroups[`${project.id}:${group.dir}`]">
                <span
                  v-for="skill in group.skills"
                  :key="skill.id"
                  class="skill-chip"
                  :class="{ managed: isManagedSkill(skill) }"
                  @click="isManagedSkill(skill) ? $emit('jumpToLocalSkill', skill.name) : $emit('unmanagedClick', skill)"
                >
                  {{ skill.name }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.panel-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
}

.project-list {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.project-item {
  padding: 16px;
  background: var(--color-card-bg);
  border: 1px solid var(--color-card-border);
  border-radius: 8px;
  transition: all 0.2s ease;
}

.project-item.selected {
  border-color: var(--color-primary-bg);
  box-shadow: 0 0 0 2px rgba(0, 113, 227, 0.2);
}

.project-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  flex-wrap: wrap;
}

.project-info {
  flex: 1;
  min-width: 200px;
}

.project-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 4px;
}

.project-path {
  font-size: 12px;
  color: var(--color-muted);
  word-break: break-all;
}

.project-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.project-actions button {
  padding: 6px 12px;
  font-size: 13px;
}

.project-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-top: 12px;
  font-size: 12px;
  color: var(--color-muted);
}

.meta-item {
  padding: 2px 8px;
  background: var(--color-chip-bg);
  border-radius: 999px;
  font-size: 11px;
}

.ide-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.ide-badge {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--color-chip-border);
  background: transparent;
  color: var(--color-muted);
  font-size: 11px;
  line-height: 1.2;
}

.ide-badge.active {
  border-color: var(--color-success-border);
  background: var(--color-success-bg);
  color: var(--color-success-text);
  font-weight: 600;
}

.project-skills {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--color-card-border);
}

.skills-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 8px;
}

.skill-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.skill-groups {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
}

.skill-group {
  background: var(--color-card-bg);
  border: 1px solid var(--color-card-border);
  border-radius: 6px;
  overflow: hidden;
}

.skill-group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background: rgba(0, 0, 0, 0.03);
  cursor: pointer;
  user-select: none;
}
[data-theme="dark"] .skill-group-header {
  background: rgba(255, 255, 255, 0.03);
}

.skill-group-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--color-text);
  flex: 1;
}

.fold-icon {
  color: var(--color-muted);
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.dir-name {
  font-family: inherit;
  font-size: 13px;
}

.skill-count {
  font-size: 12px;
  color: var(--color-muted);
}

.jump-btn {
  padding: 4px;
  color: var(--color-muted);
  background: transparent;
  border: none;
  cursor: pointer;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.jump-btn:hover {
  color: var(--color-primary);
  background: rgba(0, 0, 0, 0.05);
}
[data-theme="dark"] .jump-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.skill-group .skill-chips {
  margin-top: 0;
  padding: 10px;
  border-top: 1px solid var(--color-card-border);
}

.skill-chip {
  padding: 4px 10px;
  background: var(--color-chip-bg);
  border: 1px solid var(--color-chip-border);
  border-radius: 6px;
  font-size: 12px;
  color: var(--color-text);
  transition: all 0.2s;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
}

.skill-chip:hover {
  border-color: var(--color-primary-bg);
  background: rgba(0, 113, 227, 0.05);
}

.skill-chip.managed {
  border-color: var(--color-success-border);
  color: var(--color-success-text);
  background: var(--color-success-bg);
  cursor: pointer;
}

.skill-chip.managed:hover {
  filter: brightness(0.95);
}
</style>
