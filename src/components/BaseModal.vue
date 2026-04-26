<script setup>
import { useSlots } from 'vue'
import { X } from 'lucide-vue-next'

defineProps({
  title: { type: String, required: true },
  maxWidth: { type: String, default: '640px' },
  maxHeight: { type: String, default: '85vh' },
  label: { type: String, default: null },
})

const emit = defineEmits(['close'])
const slots = useSlots()
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div class="modal-overlay">
        <div class="modal-container" :style="{ maxWidth, maxHeight }">
          <div class="modal-hdr" :class="{ 'modal-hdr--top': label }">
            <div v-if="label" class="base-header-text">
              <span class="base-header-label">{{ label }}</span>
              <h2 class="modal-title">{{ title }}</h2>
            </div>
            <h2 v-else class="modal-title">{{ title }}</h2>
            <button class="modal-close" @click="emit('close')">
              <X :size="18" />
            </button>
          </div>

          <slot />

          <div v-if="slots.footer" class="modal-ftr">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.base-header-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.base-header-label {
  font-size: 13px;
  color: #7ba8f0;
  font-weight: 500;
}
</style>
