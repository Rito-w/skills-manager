<script setup lang="ts">
import type { IdeSkill, IdeOption } from "../composables/useSkillsManager";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  ideOptions: IdeOption[];
  selectedIdeFilter: string;
  customIdeName: string;
  customIdeDir: string;
  customIdeOptions: IdeOption[];
  filteredIdeSkills: IdeSkill[];
  localError: string | null;
  localLoading: boolean;
}>();

defineEmits<{
  (e: "update:selectedIdeFilter", value: string): void;
  (e: "update:customIdeName", value: string): void;
  (e: "update:customIdeDir", value: string): void;
  (e: "addCustomIde"): void;
  (e: "removeCustomIde", label: string): void;
  (e: "uninstall", path: string): void;
}>();
</script>

<template>
  <section class="panel">
    <div class="panel-title">{{ t("ide.title") }}</div>
    <div class="row">
      <div class="select">
        <select
          :value="selectedIdeFilter"
          @change="$emit('update:selectedIdeFilter', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="option in ideOptions" :key="option.id" :value="option.label">
            {{ option.label }}
          </option>
        </select>
      </div>
      <div class="hint">{{ t("ide.switchHint") }}</div>
    </div>
    <div class="hint">{{ t("ide.addHint") }}</div>
    <div class="row">
      <input
        :value="customIdeName"
        class="input small"
        :placeholder="t('ide.namePlaceholder')"
        @input="$emit('update:customIdeName', ($event.target as HTMLInputElement).value)"
      />
      <input
        :value="customIdeDir"
        class="input small"
        :placeholder="t('ide.dirPlaceholder')"
        @input="$emit('update:customIdeDir', ($event.target as HTMLInputElement).value)"
      />
      <button class="primary" @click="$emit('addCustomIde')">{{ t("ide.addButton") }}</button>
    </div>
    <div v-if="customIdeOptions.length > 0" class="chips">
      <div v-for="option in customIdeOptions" :key="option.id" class="chip">
        <span>{{ option.label }}</span>
        <button class="ghost" @click="$emit('removeCustomIde', option.label)">{{ t("ide.deleteButton") }}</button>
      </div>
    </div>
    <div v-if="localError" class="message error">{{ localError }}</div>
    <div v-if="localLoading" class="hint">{{ t("ide.loading") }}</div>
    <div v-if="!localLoading && filteredIdeSkills.length === 0" class="hint">{{ t("ide.emptyHint") }}</div>
    <div v-if="filteredIdeSkills.length > 0" class="cards">
      <article v-for="skill in filteredIdeSkills" :key="skill.id" class="card">
        <div class="card-header">
          <div>
            <div class="card-title">{{ skill.name }}</div>
            <div class="card-meta">
              {{ skill.ide }} · {{ skill.source === "link" ? t("ide.sourceLink") : t("ide.sourceLocal") }}
            </div>
          </div>
          <button class="ghost" @click="$emit('uninstall', skill.path)">{{ t("ide.uninstall") }}</button>
        </div>
        <div class="card-link">{{ skill.path }}</div>
      </article>
    </div>
  </section>
</template>
