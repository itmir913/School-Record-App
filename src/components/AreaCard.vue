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
    <!-- 상단: 이름 + 글자수 -->
    <div class="card-header">
      <h3 class="area-name">{{ area.name }}</h3>
      <span class="byte-badge">최대 {{ area.byte_limit.toLocaleString() }} Bytes</span>
    </div>

    <!-- 활동 칩 -->
    <div class="chip-row" v-if="area.activities.length > 0">
      <span
          v-for="act in visibleActivities"
          :key="act.id"
          class="chip"
      >{{ act.name }}</span>
      <span v-if="hiddenCount > 0" class="chip chip--more">
        +{{ hiddenCount }}개 더
      </span>
    </div>
    <p v-else class="no-activity">등록된 활동 없음</p>

    <!-- 하단 액션 -->
    <div class="card-footer">
      <button
          class="btn-assign"
          @click.stop="emit('assign-students', area)"
      >
        <Users :size="14"/>
        학생 배정
      </button>
      <div class="edit-hint">클릭하여 편집</div>
    </div>
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

.area-name {
  font-size: 18px;
  font-weight: 700;
  color: #f1f5f9;
  margin: 0;
  line-height: 1.3;
}

.byte-badge {
  font-size: 15px;
  font-weight: 600;
  color: #fbbf24;
  background-color: rgba(251, 191, 36, 0.1);
  border: 1px solid rgba(251, 191, 36, 0.2);
  border-radius: 6px;
  padding: 3px 8px;
  white-space: nowrap;
  flex-shrink: 0;
}

.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chip {
  font-size: 15px;
  color: #93c5fd;
  background-color: rgba(59, 91, 219, 0.2);
  border: 1px solid rgba(59, 91, 219, 0.3);
  border-radius: 20px;
  padding: 3px 10px;
}

.chip--more {
  font-size: 15px;
  color: #7ba3d4;
  background-color: rgba(255, 255, 255, 0.04);
  border-color: #1a2035;
}

.no-activity {
  font-size: 15px;
  color: #7ba3d4;
  font-weight: 500;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: auto;
}

.btn-assign {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 10px;
  border-radius: 8px;
  border: 1px solid rgba(59, 91, 219, 0.25);
  background: rgba(59, 91, 219, 0.08);
  color: #7ba8f0;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s;
}

.btn-assign:hover {
  background: rgba(59, 91, 219, 0.15);
  border-color: rgba(59, 91, 219, 0.4);
}

.edit-hint {
  font-size: 13px;
  color: #5a7aaa;
}

.card:hover .edit-hint {
  color: #7ba3d4;
}
</style>
