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
  <span class="diff-view">
    <template v-for="(part, i) in parts" :key="i">
      <span
        v-if="part.added"
        class="diff-added"
      >{{ part.value }}</span>
      <span
        v-else-if="part.removed"
        class="diff-removed"
      >{{ part.value }}</span>
      <span v-else>{{ part.value }}</span>
    </template>
  </span>
</template>

<style scoped>
.diff-view {
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.diff-added {
  background-color: rgba(52, 211, 153, 0.25);
  color: #6ee7b7;
}

.diff-removed {
  background-color: rgba(239, 68, 68, 0.25);
  color: #fca5a5;
  text-decoration: line-through;
}
</style>
