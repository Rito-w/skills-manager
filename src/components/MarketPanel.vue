<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { RemoteSkill, DownloadTask } from "../composables/types";
import { normalizeSkillName } from "../composables/utils";
import ManualAddSkillModal from "./ManualAddSkillModal.vue";

const { t, locale } = useI18n();

const props = defineProps<{
  query: string;
  loading: boolean;
  results: RemoteSkill[];
  hasMore: boolean;
  installingId: string | null;
  updatingId: string | null;
  localSkillNameSet: Set<string>;
  downloadQueue: DownloadTask[];
  recentTaskStatus: Record<string, "download" | "update">;
}>();

const downloadingIds = computed(() => new Set(props.downloadQueue.map((task) => task.id)));
const actionState = (skill: RemoteSkill) => props.recentTaskStatus[skill.id] ?? null;
const isInstalled = (skill: RemoteSkill) => props.localSkillNameSet.has(normalizeSkillName(skill.name));

defineEmits<{
  (e: "update:query", value: string): void;
  (e: "search"): void;
  (e: "refresh"): void;
  (e: "loadMore"): void;
  (e: "download", skill: RemoteSkill): void;
  (e: "update", skill: RemoteSkill): void;
  (e: "manualAdd", payload: { sourceUrl: string; name: string }): void;
}>();

const showManualAdd = ref(false);

async function openSource(skill: RemoteSkill) {
  if (!skill.sourceUrl?.trim()) return;
  await openUrl(skill.sourceUrl.trim());
}
</script>

<template>
  <section class="panel">
    <div class="panel-header-row">
      <div class="panel-title">{{ t("market.title") }}</div>
    </div>

    <div class="search-row">
      <input
        :value="query"
        class="input"
        :placeholder="t('market.searchPlaceholder')"
        :disabled="loading"
        @input="$emit('update:query', ($event.target as HTMLInputElement).value)"
        @keydown.enter.prevent="$emit('search')"
      />
      <button class="primary" :disabled="loading" @click="$emit('search')">
        {{ loading ? t("market.searching") : t("market.search") }}
      </button>
      <button class="ghost" :disabled="loading" @click="$emit('refresh')">
        {{ loading ? t("market.refreshing") : t("market.refresh") }}
      </button>
      <button class="ghost" :disabled="loading" @click="showManualAdd = true">
        {{ t("market.manualAdd") }}
      </button>
    </div>
  </section>

  <section class="panel">
    <div class="panel-title">{{ t("market.resultsTitle") }}</div>
    <div v-if="loading && results.length === 0" class="hint">{{ t("market.loadingHint") }}</div>
    <div v-if="results.length === 0 && !loading" class="hint">{{ t("market.emptyHint") }}</div>

    <div class="cards market-cards">
      <article v-for="skill in results" :key="skill.id" class="card">
        <div class="card-header">
          <div>
            <div class="card-title">{{ skill.name }}</div>
            <div class="card-meta">
              {{ t("market.meta", { author: skill.author }) }}
            </div>
          </div>
          <template v-if="isInstalled(skill)">
            <button
              class="ghost"
              :disabled="downloadingIds.has(skill.id) || actionState(skill) === 'update' || !skill.sourceUrl || !skill.sourceUrl.trim()"
              :title="(!skill.sourceUrl || !skill.sourceUrl.trim()) ? t('market.unavailable') : ''"
              @click="$emit('update', skill)"
            >
              {{
                (!skill.sourceUrl || !skill.sourceUrl.trim())
                  ? t("market.unavailable")
                  : downloadingIds.has(skill.id)
                    ? t("market.queued")
                    : actionState(skill) === "update"
                      ? t("market.updated")
                      : t("market.update")
              }}
            </button>
          </template>
          <template v-else>
            <button
              class="primary"
              :disabled="downloadingIds.has(skill.id) || actionState(skill) === 'download' || !skill.sourceUrl || !skill.sourceUrl.trim()"
              :title="(!skill.sourceUrl || !skill.sourceUrl.trim()) ? t('market.unavailable') : ''"
              @click="$emit('download', skill)"
            >
              {{
                (!skill.sourceUrl || !skill.sourceUrl.trim())
                  ? t("market.unavailable")
                  : downloadingIds.has(skill.id)
                    ? t("market.queued")
                    : actionState(skill) === "download"
                      ? t("market.downloaded")
                      : t("market.download")
              }}
            </button>
          </template>
        </div>
        <p class="card-desc">{{ locale === 'zh-CN' && skill.descriptionZh ? skill.descriptionZh : skill.description }}</p>
        <div class="card-source">{{ t("market.source", { source: skill.marketLabel }) }}</div>
        <div class="card-link">{{ skill.sourceUrl }}</div>
        <div class="card-actions market-card-actions">
          <button
            class="ghost"
            :disabled="!skill.sourceUrl || !skill.sourceUrl.trim()"
            @click="openSource(skill)"
          >
            {{ t("market.viewSource") }}
          </button>
        </div>
      </article>
    </div>

    <div v-if="hasMore" class="more">
      <button class="ghost" :disabled="loading" @click="$emit('loadMore')">
        {{ t("market.loadMore") }}
      </button>
    </div>
  </section>

  <ManualAddSkillModal
    :show="showManualAdd"
    @close="showManualAdd = false"
    @submit="$emit('manualAdd', $event)"
  />
</template>

<style scoped>
.panel-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.icon-btn {
  padding: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.market-card-actions {
  margin-top: 12px;
  gap: 8px;
  flex-wrap: wrap;
}
</style>
