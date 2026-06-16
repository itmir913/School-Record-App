<script setup>
import { computed } from 'vue'
import { diffChars } from 'diff'

const props = defineProps({
  before: { type: String, default: '' },
  after:  { type: String, default: '' },
})

const parts = computed(() => diffChars(props.before, props.after))
</script>

<template>
  <span class="text-sm leading-relaxed whitespace-pre-wrap break-all">
    <template v-for="(part, i) in parts" :key="i">
      <span v-if="part.added"   class="bg-green/25 text-green">{{ part.value }}</span>
      <span v-else-if="part.removed" class="bg-red/25 text-red line-through">{{ part.value }}</span>
      <span v-else>{{ part.value }}</span>
    </template>
  </span>
</template>
