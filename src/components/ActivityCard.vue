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
    <div class="card-top">
      <span class="area-count-badge" :class="areaBadgeClass">{{ areaBadgeText }}</span>
      <h3 class="activity-name">{{ activity.name }}</h3>
    </div>
    <div class="card-bottom">
      <span class="record-count" :class="recordCountEmpty ? 'record-count--empty' : ''">
        {{ recordCountText }}
      </span>
      <span class="edit-hint">편집</span>
    </div>
  </div>
</template>

<style scoped>
.card {
  background-color: #0e1220;
  border: 1px solid #1e293b;
  border-radius: 14px;
  padding: 18px 20px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.card:hover {
  border-color: rgba(59, 91, 219, 0.5);
  box-shadow: 0 4px 20px rgba(59, 91, 219, 0.1);
}

.card-top {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
}

.area-count-badge {
  align-self: flex-start;
  font-size: 15px;
  font-weight: 600;
  color: #7ba8f0;
  background-color: rgba(59, 91, 219, 0.12);
  border: 1px solid rgba(59, 91, 219, 0.25);
  border-radius: 5px;
  padding: 2px 9px;
  white-space: nowrap;
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

.activity-name {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
  line-height: 1.4;
}

.card-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 10px;
  border-top: 1px solid #1a2035;
}

.record-count {
  font-size: 15px;
  color: #7ba8f0;
  font-weight: 500;
}

.record-count--empty {
  color: #3d5070;
}

.edit-hint {
  font-size: 13px;
  color: transparent;
  transition: color 0.15s;
}

.card:hover .edit-hint {
  color: #4a6080;
}
</style>
