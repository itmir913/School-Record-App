<script setup>
import {ref, watch} from 'vue'
import {ArrowLeft, ArrowRight} from 'lucide-vue-next'

const props = defineProps({
  stepCount:    {type: Number,  required: true},
  currentStep:  {type: Number,  required: true},
  canGoNext:    {type: Boolean, required: true},
  isNavigating: {type: Boolean, required: true},
  showFooter:   {type: Boolean, default: true},
  nextLabel:    {type: String,  default: '다음'},
})

const emit = defineEmits(['prev', 'next'])
const bodyRef = ref(null)

watch(() => props.currentStep, () => {
  bodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})
</script>

<template>
  <div class="flex-1 overflow-hidden flex flex-col">
    <div class="flex items-center justify-end gap-2 px-10 py-2.5 border-b border-line shrink-0">
      <div
          v-for="n in stepCount"
          :key="n"
          class="flex items-center justify-center w-7 h-7 rounded-full text-sm font-semibold border border-line text-ink-5 bg-transparent transition-all duration-200"
          :class="[
            currentStep === n ? 'border-blue/80 text-ink-3 bg-blue/12' : '',
            currentStep > n  ? 'border-green/50 text-green bg-green/8' : '',
          ]"
      >
        {{ currentStep > n ? '✓' : n }}
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-10 py-8" ref="bodyRef">
      <slot/>
    </div>

    <div v-if="showFooter" class="flex items-center justify-between px-10 py-4 border-t border-line shrink-0">
      <button class="btn-secondary" :disabled="currentStep === 1" @click="emit('prev')">
        <ArrowLeft :size="15"/>
        이전
      </button>
      <button
          v-if="currentStep < stepCount"
          class="btn-primary"
          :disabled="!canGoNext || isNavigating"
          @click="emit('next')"
      >
        {{ isNavigating ? '불러오는 중…' : nextLabel }}
        <ArrowRight :size="15"/>
      </button>
    </div>
  </div>
</template>
