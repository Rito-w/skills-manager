<script setup lang="ts">
import { computed } from "vue";
import type { IdeOption } from "../composables/useSkillsManager";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  visible: boolean;
  ideOptions: IdeOption[];
  selected: string[];

}>();

const emit = defineEmits<{
  (e: "update:selected", value: string[]): void;
  (e: "confirm"): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();

const allSelected = computed(
  () => props.ideOptions.length > 0 && props.selected.length === props.ideOptions.length
);

const toggleSelectAll = (checked: boolean) => {
  if (checked) {
    const next = props.ideOptions.map((option) => option.label);
    emit("update:selected", next);
  } else {
    emit("update:selected", []);
  }
};
</script>

<template>
  <div v-if="visible" class="modal-backdrop">
    <div class="modal">
      <div class="modal-title">{{ t("installModal.title") }}</div>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="allSelected"
          @change="toggleSelectAll(($event.target as HTMLInputElement).checked)"
        />
        {{ t("installModal.selectAll") }}
      </label>
      <div class="grid">
        <label v-for="option in ideOptions" :key="option.id" class="checkbox">
          <input
            type="checkbox"
            :value="option.label"
            :checked="selected.includes(option.label)"
            @change="
              $emit(
                'update:selected',
                ($event.target as HTMLInputElement).checked
                  ? [...selected, option.label]
                  : selected.filter((item) => item !== option.label)
              )
            "
          />
          {{ option.label }}
        </label>
      </div>

      <div class="modal-actions">
        <button class="ghost" @click="$emit('cancel')">{{ t("installModal.cancel") }}</button>
        <button class="primary" :disabled="selected.length === 0" @click="$emit('confirm')">
          {{ t("installModal.confirm") }}
        </button>
      </div>
    </div>
  </div>
</template>
