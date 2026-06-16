<script setup>
import {computed} from 'vue'

const props = defineProps({
  activity: {type: Object, required: true},
})

const emit = defineEmits(['edit'])

const areaBadgeText = computed(() => {
  const count = props.activity.areas.length
  if (count === 0) return '미배정'
  if (count === 1) return props.activity.areas[0].name
  return '여러 영역'
})

const areaBadgeClass = computed(() => {
  const count = props.activity.areas.length
  if (count === 0) return 'text-amber bg-amber/10 border-amber/30'
  if (count > 1) return 'text-red bg-red/10 border-red/30'
  return 'text-ink-3 bg-blue/12 border-blue/25'
})

const recordCountText = computed(() => {
  const n = props.activity.record_count ?? 0
  if (n === 0) return '기록된 학생 없음'
  return `학생 기록 ${n}명`
})

const recordCountEmpty = computed(() => (props.activity.record_count ?? 0) === 0)
</script>

<template>
  <div
    class="bg-surface border border-line rounded-[14px] py-[18px] px-5 cursor-pointer transition-all duration-150 flex flex-col gap-3 hover:border-blue/50 hover:shadow-[0_4px_20px_color-mix(in_srgb,var(--c-blue)_10%,transparent)] group"
    @click="emit('edit', activity)"
  >
    <div class="flex flex-col gap-2 flex-1">
      <span
        class="self-start text-sm font-semibold rounded px-[9px] py-0.5 whitespace-nowrap border"
        :class="areaBadgeClass"
      >{{ areaBadgeText }}</span>
      <h3 class="text-lg font-bold text-ink m-0 leading-[1.4]">{{ activity.name }}</h3>
    </div>
    <div class="flex items-center justify-between pt-[10px] border-t border-line">
      <span
        class="text-sm font-medium"
        :class="recordCountEmpty ? 'text-ink-5' : 'text-ink-3'"
      >{{ recordCountText }}</span>
      <span class="text-sm text-transparent transition-colors duration-150 group-hover:text-ink-5">편집</span>
    </div>
  </div>
</template>
