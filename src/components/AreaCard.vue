<script setup>
import {computed} from 'vue'
import {Users} from 'lucide-vue-next'

const props = defineProps({
  area: {type: Object, required: true},
})

const emit = defineEmits(['edit', 'assign-students'])

const CHIP_MAX = 4

const visibleActivities = computed(() =>
    props.area.activities.slice(0, CHIP_MAX)
)

const hiddenCount = computed(() =>
    Math.max(0, props.area.activities.length - CHIP_MAX)
)
</script>

<template>
  <div
    class="bg-surface border border-line rounded-[14px] py-[18px] px-5 cursor-pointer transition-all duration-150 flex flex-col gap-3 hover:border-violet/50 hover:shadow-[0_4px_20px_color-mix(in_srgb,var(--c-violet)_10%,transparent)] group"
    @click="emit('edit', area)"
  >
    <div class="flex flex-col gap-2 flex-1">
      <h3 class="text-lg font-bold text-ink m-0 leading-[1.4]">{{ area.name }}</h3>
      <span class="self-start text-sm font-semibold text-amber bg-amber/10 border border-amber/25 rounded px-[9px] py-0.5 whitespace-nowrap">
        최대 {{ area.byte_limit.toLocaleString() }} Bytes
      </span>
      <div class="h-px bg-line"></div>
      <div v-if="area.activities.length > 0" class="flex flex-wrap gap-[6px]">
        <span
          v-for="act in visibleActivities"
          :key="act.id"
          class="text-sm text-ink-3 bg-blue/15 border border-blue/30 rounded-full px-[10px] py-0.5"
        >{{ act.name }}</span>
        <span
          v-if="hiddenCount > 0"
          class="text-sm text-ink-4 bg-white/4 border border-line rounded-full px-[10px] py-0.5"
        >+{{ hiddenCount }}개 더</span>
      </div>
      <p v-else class="text-sm text-ink-5 m-0">등록된 활동 없음</p>
    </div>

    <div class="flex items-center justify-between pt-[10px] border-t border-line">
      <button
        class="inline-flex items-center gap-[5px] px-[10px] py-[5px] rounded-lg border border-violet/25 bg-violet/8 text-violet text-sm font-medium cursor-pointer transition-colors duration-150 hover:bg-violet/15 hover:border-violet/45"
        @click.stop="emit('assign-students', area)"
      >
        <Users :size="14"/>
        학생 배정
      </button>
      <span class="text-sm text-transparent transition-colors duration-150 group-hover:text-ink-5">편집</span>
    </div>
  </div>
</template>
