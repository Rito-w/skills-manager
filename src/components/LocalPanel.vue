<script setup lang="ts">
import type { LocalSkill } from "../composables/useSkillsManager";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  localSkills: LocalSkill[];
  localLoading: boolean;
  localError: string | null;
  installMessage: string | null;
  installingId: string | null;
}>();

defineEmits<{
  (e: "install", skill: LocalSkill): void;
}>();
</script>

<template>
  <section class="panel">
    <div class="panel-title">{{ t("local.title") }}</div>
    <div class="hint">{{ t("local.hint") }}</div>
    <div class="actions">
      <div class="hint">{{ t("local.actionsHint") }}</div>
    </div>
    <div v-if="installMessage" class="message success">{{ installMessage }}</div>
    <div v-if="localError" class="message error">{{ localError }}</div>
    <div v-if="localLoading" class="hint">{{ t("local.scanning") }}</div>
    <div v-if="!localLoading && localSkills.length === 0" class="hint">{{ t("local.emptyHint") }}</div>
    <div v-if="localSkills.length > 0" class="cards">
      <article v-for="skill in localSkills" :key="skill.id" class="card">
        <div class="card-header">
          <div>
            <div class="card-title">{{ skill.name }}</div>
            <div class="card-meta">
              Skills Manager ·
              {{
                skill.usedBy.length > 0
                  ? t("local.usedBy", { ideList: skill.usedBy.join(", ") })
                  : t("local.unused")
              }}
            </div>
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
