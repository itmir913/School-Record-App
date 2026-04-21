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
  if (count === 0) return 'area-count-badge--empty'
  if (count > 1) return 'area-count-badge--multi'
  return ''
})

const recordCountText = computed(() => {
  const n = props.activity.record_count ?? 0
  if (n === 0) return '기록된 학생 없음'
  return `학생 기록 ${n}명`
})

const recordCountEmpty = computed(() => (props.activity.record_count ?? 0) === 0)
</script>

<template>
  <div class="card" @click="emit('edit', activity)">
    <div class="card-header">
      <h3 class="activity-name">{{ activity.name }}</h3>
      <span
          class="area-count-badge"
          :class="areaBadgeClass"
      >
        {{ areaBadgeText }}
      </span>
    </div>

    <div class="record-count" :class="recordCountEmpty ? 'record-count--empty' : ''">
      {{ recordCountText }}
    </div>

    <div class="edit-hint">클릭하여 편집</div>
  </div>
</template>

<style scoped>
.card {
  background-color: #0e1220;
  border: 1px solid #1e293b;
  border-radius: 16px;
  padding: 22px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.card:hover {
  border-color: rgba(59, 91, 219, 0.5);
  box-shadow: 0 4px 24px rgba(59, 91, 219, 0.08);
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.activity-name {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
  line-height: 1.3;
}

.area-count-badge {
  font-size: 15px;
  font-weight: 600;
  color: #7ba8f0;
  background-color: rgba(59, 91, 219, 0.12);
  border: 1px solid rgba(59, 91, 219, 0.25);
  border-radius: 6px;
  padding: 3px 8px;
  white-space: nowrap;
  flex-shrink: 0;
}

.area-count-badge--empty {
  color: #fbbf24;
  background-color: rgba(251, 191, 36, 0.1);
  border-color: rgba(251, 191, 36, 0.3);
}

.area-count-badge--multi {
  color: #f87171;
  background-color: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.3);
}

.record-count {
  font-size: 15px;
  color: #93c5fd;
  font-weight: 500;
}

.record-count--empty {
  color: #7ba3d4;
}

.edit-hint {
  font-size: 13px;
  color: var(--clr-text-subtle);
  text-align: right;
  margin-top: auto;
}

.card:hover .edit-hint {
  color: #7ba3d4;
}
</style>
