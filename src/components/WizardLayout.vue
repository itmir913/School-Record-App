<script setup>
import {ref, watch} from 'vue'
import {ArrowLeft, ArrowRight} from 'lucide-vue-next'

const props = defineProps({
  stepCount: {type: Number, required: true},
  currentStep: {type: Number, required: true},
  canGoNext: {type: Boolean, required: true},
  isNavigating: {type: Boolean, required: true},
  showFooter: {type: Boolean, default: true},
  nextLabel: {type: String, default: '다음'},
})

const emit = defineEmits(['prev', 'next'])
const bodyRef = ref(null)

watch(() => props.currentStep, () => {
  bodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})
</script>

<template>
  <div class="wizard-layout">
    <div class="step-indicator">
      <div
          v-for="n in stepCount"
          :key="n"
          class="step-dot"
          :class="{ 'step-dot--active': currentStep === n, 'step-dot--done': currentStep > n }"
      >
        {{ currentStep > n ? '✓' : n }}
      </div>
    </div>
    <div class="wizard-body" ref="bodyRef">
      <slot/>
    </div>
    <div v-if="showFooter" class="wizard-footer">
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

<style scoped>
.wizard-layout {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.step-indicator {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px 40px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
}

.step-dot {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 600;
  border: 1px solid #1a2035;
  color: var(--clr-text-hint);
  background: transparent;
  transition: all 0.2s;
}

.step-dot--active {
  border-color: rgba(59, 91, 219, 0.8);
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.12);
}

.step-dot--done {
  border-color: rgba(52, 211, 153, 0.5);
  color: #34d399;
  background: rgba(52, 211, 153, 0.08);
}

.wizard-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 40px;
}

.wizard-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 40px;
  border-top: 1px solid #1a2035;
  flex-shrink: 0;
}

.wizard-footer .btn-secondary,
.wizard-footer .btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
