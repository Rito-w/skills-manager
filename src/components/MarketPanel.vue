<script setup lang="ts">
import type { RemoteSkill, MarketStatus, DownloadTask } from "../composables/useSkillsManager";
import { useI18n } from "vue-i18n";
import { ref, computed } from "vue";
import MarketSettingsModal from "./MarketSettingsModal.vue";

const { t } = useI18n();

const props = defineProps<{
  query: string;
  loading: boolean;
  results: RemoteSkill[];
  hasMore: boolean;
  installingId: string | null;
  updatingId: string | null;
  localSkillNameSet: Set<string>;
  marketConfigs: Record<string, string>;
  marketStatuses: MarketStatus[];
  enabledMarkets: Record<string, boolean>;
  downloadQueue: DownloadTask[];
}>();

const downloadingIds = computed(() => new Set(props.downloadQueue.map(t => t.id)));

defineEmits<{
  (e: "update:query", value: string): void;
  (e: "search"): void;
  (e: "refresh"): void;
  (e: "loadMore"): void;
  (e: "download", skill: RemoteSkill): void;
  (e: "update", skill: RemoteSkill): void;
  (e: "saveConfigs", configs: Record<string, string>, enabled: Record<string, boolean>): void;
}>();

const showSettings = ref(false);
</script>

<template>
  <section class="panel">
    <div class="panel-header-row">
      <div class="panel-title">{{ t("market.title") }}</div>
      <button class="ghost icon-btn" @click="showSettings = true" title="Settings">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.72v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
      </button>
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
              {{ t("market.meta", { author: skill.author, stars: skill.stars, installs: skill.installs }) }}
            </div>
          </div>
          <template v-if="localSkillNameSet.has(skill.name.trim().toLowerCase())">
            <button class="ghost" :disabled="updatingId === skill.id" @click="$emit('update', skill)">
              {{ updatingId === skill.id ? t("market.updating") : t("market.update") }}
            </button>
          </template>
          <template v-else>
            <button 
              class="primary" 
              :disabled="downloadingIds.has(skill.id)" 
              @click="$emit('download', skill)"
            >
              {{ downloadingIds.has(skill.id) ? t("market.queued") : t("market.download") }}
            </button>
          </template>
        </div>
        <p class="card-desc">{{ skill.description }}</p>
        <div class="card-source">{{ t("market.source", { source: skill.marketLabel }) }}</div>
        <div class="card-link">{{ skill.sourceUrl }}</div>
      </article>
    </div>

    <div v-if="hasMore" class="more">
      <button class="ghost" :disabled="loading" @click="$emit('loadMore')">
        {{ t("market.loadMore") }}
      </button>
    </div>
  </section>
  
  <MarketSettingsModal
    :show="showSettings"
    :configs="marketConfigs"
    :enabled="enabledMarkets"
    :statuses="marketStatuses"
    @close="showSettings = false"
    @save="(configs, enabled) => $emit('saveConfigs', configs, enabled)"
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
</style>
