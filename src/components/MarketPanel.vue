<script setup lang="ts">
import type { RemoteSkill } from "../composables/useSkillsManager";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  query: string;
  loading: boolean;
  error: string | null;
  installMessage: string | null;
  results: RemoteSkill[];
  hasMore: boolean;
  installingId: string | null;
  updatingId: string | null;
  localSkillNameSet: Set<string>;
}>();

defineEmits<{
  (e: "update:query", value: string): void;
  (e: "search"): void;
  (e: "refresh"): void;
  (e: "loadMore"): void;
  (e: "download", skill: RemoteSkill): void;
  (e: "update", skill: RemoteSkill): void;
}>();
</script>

<template>
  <section class="panel">
    <div class="panel-title">{{ t("market.title") }}</div>
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
    <div v-if="error" class="message error">{{ error }}</div>
    <div v-if="installMessage" class="message success">{{ installMessage }}</div>
  </section>

  <section class="panel">
    <div class="panel-title">{{ t("market.resultsTitle") }}</div>
    <div v-if="loading && results.length === 0" class="hint">{{ t("market.loadingHint") }}</div>
    <div v-if="results.length === 0 && !loading" class="hint">{{ t("market.emptyHint") }}</div>

    <div class="cards">
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
            <button class="primary" :disabled="installingId === skill.id" @click="$emit('download', skill)">
              {{ installingId === skill.id ? t("market.downloading") : t("market.download") }}
            </button>
          </template>
        </div>
        <p class="card-desc">{{ skill.description }}</p>
        <div class="card-link">{{ skill.sourceUrl }}</div>
      </article>
    </div>

    <div v-if="hasMore" class="more">
      <button class="ghost" :disabled="loading" @click="$emit('loadMore')">
        {{ t("market.loadMore") }}
      </button>
    </div>
  </section>
</template>
