<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LocalSkill, LocalSkillPreview, DownloadTask, IdeOption } from "../composables/types";
import DownloadQueue from "./DownloadQueue.vue";
import SkillPreviewModal from "./SkillPreviewModal.vue";
import { useI18n } from "vue-i18n";
import { normalizeSkillName } from "../composables/utils";
import { useToast } from "../composables/useToast";

const { t } = useI18n();
const toast = useToast();

const props = defineProps<{
  localSkills: LocalSkill[];
  localLoading: boolean;
  installingId: string | null;
  downloadQueue: DownloadTask[];
  ideOptions: IdeOption[];
}>();

const emit = defineEmits<{
  (e: "install", skill: LocalSkill): void;
  (e: "installMany", skills: LocalSkill[]): void;
  (e: "exportLocal", skills: LocalSkill[]): void;
  (e: "deleteLocal", skills: LocalSkill[]): void;
  (e: "openDir", path: string): void;
  (e: "refresh"): void;
  (e: "import"): void;
  (e: "retryDownload", taskId: string): void;
  (e: "removeFromQueue", taskId: string): void;
}>();

const selectedIds = ref<string[]>([]);
const searchQuery = ref("");
const previewVisible = ref(false);
const previewLoading = ref(false);
const previewSkill = ref<LocalSkill | null>(null);
const previewData = ref<LocalSkillPreview | null>(null);

const filteredLocalSkills = computed(() => {
  const keyword = searchQuery.value.trim().toLowerCase();
  if (!keyword) return props.localSkills;
  const normalizedKeyword = normalizeSkillName(keyword);
  return props.localSkills.filter((skill) => {
    const haystacks = [skill.name, skill.description, skill.path];
    return haystacks.some((value) => {
      const lowered = value.toLowerCase();
      return lowered.includes(keyword) || normalizeSkillName(value).includes(normalizedKeyword);
    });
  });
});

watch(
  () => props.localSkills,
  (skills) => {
    const available = new Set(skills.map((skill) => skill.id));
    selectedIds.value = selectedIds.value.filter((id) => available.has(id));
  },
  { deep: true }
);

const selectedSkills = computed(() =>
  filteredLocalSkills.value.filter((skill) => selectedIds.value.includes(skill.id))
);

const allSelected = computed(
  () =>
    filteredLocalSkills.value.length > 0 &&
    filteredLocalSkills.value.every((skill) => selectedIds.value.includes(skill.id))
);

function toggleSelectAll(checked: boolean) {
  const filteredIds = filteredLocalSkills.value.map((skill) => skill.id);
  if (checked) {
    selectedIds.value = Array.from(new Set([...selectedIds.value, ...filteredIds]));
    return;
  }
  selectedIds.value = selectedIds.value.filter((id) => !filteredIds.includes(id));
}

function toggleSelected(skillId: string, checked: boolean) {
  selectedIds.value = checked
    ? [...selectedIds.value, skillId]
    : selectedIds.value.filter((id) => id !== skillId);
}

function buildIdeBadgeList(skill: LocalSkill) {
  return props.ideOptions.map((option) => ({
    label: option.label,
    active: skill.usedBy.includes(option.label)
  }));
}

function installSelected() {
  if (selectedSkills.value.length === 0) return;
  emit("installMany", selectedSkills.value);
}

function exportSelected() {
  if (selectedSkills.value.length === 0) return;
  emit("exportLocal", selectedSkills.value);
}

function deleteSelected() {
  if (selectedSkills.value.length === 0) return;
  emit("deleteLocal", selectedSkills.value);
}

async function openPreview(skill: LocalSkill) {
  const currentSkillPath = skill.path;
  previewVisible.value = true;
  previewLoading.value = true;
  previewSkill.value = skill;
  previewData.value = null;

  try {
    const result = await invoke<LocalSkillPreview>("read_local_skill_preview", {
      skillPath: currentSkillPath
    });
    if (previewSkill.value?.path !== currentSkillPath) return;
    previewData.value = result;
  } catch {
    if (previewSkill.value?.path !== currentSkillPath) return;
    closePreview();
    toast.error(t("errors.previewFailed"));
  } finally {
    if (previewSkill.value?.path === currentSkillPath || previewSkill.value === null) {
      previewLoading.value = false;
    }
  }
}

function closePreview() {
  previewVisible.value = false;
  previewLoading.value = false;
  previewSkill.value = null;
  previewData.value = null;
}
</script>

<template>
  <section class="panel">
    <div class="panel-title">{{ t("local.title") }}</div>
    <div class="hint">{{ t("local.hint") }}</div>
    <div class="panel-summary">
      <span>{{ t("local.total", { count: localSkills.length }) }}</span>
      <label class="checkbox select-all">
        <input
          type="checkbox"
          :checked="allSelected"
          :disabled="filteredLocalSkills.length === 0"
          @change="toggleSelectAll(($event.target as HTMLInputElement).checked)"
        />
        {{ t("local.selectAll") }}
      </label>
    </div>
    <div class="search-row">
      <input
        v-model="searchQuery"
        class="input"
        :placeholder="t('local.searchPlaceholder')"
      />
      <div class="hint search-summary">
        {{ t("local.filteredTotal", { shown: filteredLocalSkills.length, total: localSkills.length }) }}
      </div>
    </div>
    <div class="actions">
      <div class="buttons">
        <button class="ghost" :disabled="localLoading" @click="$emit('refresh')">
          {{ localLoading ? t("local.scanning") : t("market.refresh") }}
        </button>
        <button class="primary" :disabled="localLoading" @click="$emit('import')">
          {{ t("local.import") }}
        </button>
        <button class="ghost" :disabled="selectedSkills.length === 0 || localLoading" @click="installSelected">
          {{ t("local.installSelected", { count: selectedSkills.length }) }}
        </button>
        <button class="ghost" :disabled="selectedSkills.length === 0 || localLoading" @click="exportSelected">
          {{ t("local.exportSelected", { count: selectedSkills.length }) }}
        </button>
        <button class="ghost danger" :disabled="selectedSkills.length === 0 || localLoading" @click="deleteSelected">
          {{ t("local.deleteSelected", { count: selectedSkills.length }) }}
        </button>
        <button
          class="ghost danger"
          :disabled="localSkills.length === 0 || localLoading"
          @click="$emit('deleteLocal', localSkills)"
        >
          {{ t("local.deleteAll") }}
        </button>
      </div>
    </div>

    <DownloadQueue
      :tasks="downloadQueue"
      @retry="$emit('retryDownload', $event)"
      @remove="$emit('removeFromQueue', $event)"
    />

    <div v-if="localLoading" class="hint">{{ t("local.scanning") }}</div>
    <div v-if="!localLoading && localSkills.length === 0" class="hint">{{ t("local.emptyHint") }}</div>
    <div v-else-if="!localLoading && filteredLocalSkills.length === 0" class="hint">
      {{ t("local.searchEmptyHint") }}
    </div>
    <div v-if="filteredLocalSkills.length > 0" class="cards">
      <article
        v-for="(skill, index) in filteredLocalSkills"
        :key="skill.id"
        class="card local-card"
        :class="{ linked: skill.usedBy.length > 0 }"
      >
        <div class="card-header">
          <div class="card-title-row">
            <label class="checkbox card-select">
              <input
                type="checkbox"
                :checked="selectedIds.includes(skill.id)"
                @change="toggleSelected(skill.id, ($event.target as HTMLInputElement).checked)"
              />
            </label>
            <div>
              <div class="card-title">{{ index + 1 }}. {{ skill.name }}</div>
              <div class="card-meta">
                {{ skill.usedBy.length > 0 ? t("local.linked") : t("local.unused") }}
              </div>
            </div>
          </div>
          <div class="card-actions">
            <button class="primary" :disabled="installingId === skill.id" @click="$emit('install', skill)">
            {{ installingId === skill.id ? t("local.processing") : t("local.install") }}
            </button>
            <button class="ghost" @click="openPreview(skill)">
              {{ t("local.preview") }}
            </button>
            <button class="ghost" @click="$emit('openDir', skill.path)">
              {{ t("local.openDir") }}
            </button>
            <button class="ghost" @click="$emit('exportLocal', [skill])">
              {{ t("local.exportOne") }}
            </button>
            <button class="ghost danger" @click="$emit('deleteLocal', [skill])">
              {{ t("local.deleteOne") }}
            </button>
          </div>
        </div>
        <p class="card-desc">{{ skill.description }}</p>
        <div class="card-link">{{ skill.path }}</div>
        <div class="ide-badges">
          <span
            v-for="badge in buildIdeBadgeList(skill)"
            :key="badge.label"
            class="ide-badge"
            :class="{ active: badge.active }"
          >
            {{ badge.label }}
          </span>
        </div>
      </article>
    </div>

    <SkillPreviewModal
      :visible="previewVisible"
      :skill="previewSkill"
      :preview="previewData"
      :loading="previewLoading"
      @close="closePreview"
    />
  </section>
</template>

<style scoped>
.panel-summary {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
  font-size: 13px;
  color: var(--color-muted);
}

.buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 12px;
}

.search-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.search-row .input {
  flex: 1 1 280px;
}

.search-summary {
  white-space: nowrap;
}

.select-all {
  justify-content: flex-end;
}

.local-card {
  position: relative;
}

.local-card.linked {
  border-color: var(--color-success-border);
  box-shadow: inset 0 0 0 1px var(--color-success-border);
}

.card-title-row,
.card-actions {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.card-actions {
  flex-wrap: wrap;
  justify-content: flex-end;
}

.card-select {
  padding-top: 2px;
}

.ide-badges {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
  margin-top: 12px;
}

.ide-badge {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--color-chip-border);
  background: transparent;
  color: var(--color-meta);
  font-size: 11px;
  line-height: 1.2;
}

.ide-badge.active {
  border-color: var(--color-success-border);
  background: var(--color-success-bg);
  color: var(--color-success-text);
  font-weight: 600;
}
</style>
