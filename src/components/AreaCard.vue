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
  <div class="card" @click="emit('edit', area)">
    <div class="card-top">
      <h3 class="area-name">{{ area.name }}</h3>
      <span class="byte-badge">최대 {{ area.byte_limit.toLocaleString() }} Bytes</span>
      <div class="name-divider"></div>
      <div class="chip-row" v-if="area.activities.length > 0">
        <span v-for="act in visibleActivities" :key="act.id" class="chip">{{ act.name }}</span>
        <span v-if="hiddenCount > 0" class="chip chip--more">+{{ hiddenCount }}개 더</span>
      </div>
      <p v-else class="no-activity">등록된 활동 없음</p>
    </div>

    <div class="card-bottom">
      <button class="btn-assign" @click.stop="emit('assign-students', area)">
        <Users :size="14"/>
        학생 배정
      </button>
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
  border-color: rgba(168, 85, 247, 0.5);
  box-shadow: 0 4px 20px rgba(168, 85, 247, 0.1);
}

.card-top {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
}

.byte-badge {
  align-self: flex-start;
  font-size: 15px;
  font-weight: 600;
  color: #fbbf24;
  background-color: rgba(251, 191, 36, 0.1);
  border: 1px solid rgba(251, 191, 36, 0.25);
  border-radius: 5px;
  padding: 2px 9px;
  white-space: nowrap;
}

.area-name {
  font-size: 18px;
  font-weight: 700;
  color: #f1f5f9;
  margin: 0;
  line-height: 1.4;
}

.name-divider {
  height: 1px;
  background: #1a2035;
}

.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chip {
  font-size: 15px;
  color: #93c5fd;
  background-color: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.3);
  border-radius: 20px;
  padding: 2px 10px;
}

.chip--more {
  font-size: 15px;
  color: #7ba3d4;
  background-color: rgba(255, 255, 255, 0.04);
  border-color: #1a2035;
}

.no-activity {
  font-size: 15px;
  color: #3d5070;
  margin: 0;
}

.card-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 10px;
  border-top: 1px solid #1a2035;
}

.btn-assign {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 10px;
  border-radius: 8px;
  border: 1px solid rgba(168, 85, 247, 0.25);
  background: rgba(168, 85, 247, 0.08);
  color: #c084fc;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s;
}

.btn-assign:hover {
  background: rgba(168, 85, 247, 0.15);
  border-color: rgba(168, 85, 247, 0.45);
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
