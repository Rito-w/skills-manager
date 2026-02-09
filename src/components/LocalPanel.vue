<script setup lang="ts">
import type { LocalSkill, DownloadTask } from "../composables/useSkillsManager";
import DownloadQueue from "./DownloadQueue.vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  localSkills: LocalSkill[];
  localLoading: boolean;
  installingId: string | null;
  downloadQueue: DownloadTask[];
}>();

defineEmits<{
  (e: "install", skill: LocalSkill): void;
  (e: "refresh"): void;
  (e: "import"): void;
  (e: "retryDownload", taskId: string): void;
  (e: "removeFromQueue", taskId: string): void;
}>();
</script>

<template>
  <section class="panel">
    <div class="panel-title">{{ t("local.title") }}</div>
    <div class="hint">{{ t("local.hint") }}</div>
    <div class="actions">
      <div class="buttons">
        <button class="ghost" :disabled="localLoading" @click="$emit('refresh')">
          {{ localLoading ? t("local.scanning") : t("market.refresh") }}
        </button>
        <button class="primary" :disabled="localLoading" @click="$emit('import')">
          {{ t("local.import") }}
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
    <div v-if="localSkills.length > 0" class="cards">
      <article v-for="skill in localSkills" :key="skill.id" class="card">
        <div class="card-header">
          <div>
            <div class="card-title">{{ skill.name }}</div>
          </div>
          <button class="primary" :disabled="installingId === skill.id" @click="$emit('install', skill)">
            {{ installingId === skill.id ? t("local.processing") : t("local.install") }}
          </button>
        </div>
        <p class="card-desc">{{ skill.description }}</p>
        <div class="card-link">{{ skill.path }}</div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.buttons {
  display: flex;
  gap: 12px;
  margin-top: 12px;
}
</style>
