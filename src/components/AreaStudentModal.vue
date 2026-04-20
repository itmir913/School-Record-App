<script setup>
import {computed, ref, watch} from 'vue'
import {ChevronDown, ChevronRight, X} from 'lucide-vue-next'

const props = defineProps({
  area: {type: Object, required: true},
  allStudents: {type: Array, default: () => []},
  initialStudentIds: {type: Array, default: () => []},
})

const emit = defineEmits(['close', 'saved'])

const selectedIds = ref(new Set())
const expandedGroups = ref(new Set())

watch(
    () => props.initialStudentIds,
    (ids) => {
      selectedIds.value = new Set(ids)
    },
    {immediate: true}
)

// (학년, 반) 기준으로 그룹핑
const groups = computed(() => {
  const map = new Map()
  for (const s of props.allStudents) {
    const key = `${s.grade}-${s.class_num}`
    if (!map.has(key)) {
      map.set(key, {grade: s.grade, classNum: s.class_num, students: []})
    }
    map.get(key).students.push(s)
  }
  return [...map.values()]
})

function isGroupExpanded(key) {
  return expandedGroups.value.has(key)
}

function toggleGroup(key) {
  const next = new Set(expandedGroups.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedGroups.value = next
}

function groupKey(g) {
  return `${g.grade}-${g.classNum}`
}

function isGroupAllSelected(g) {
  return g.students.every(s => selectedIds.value.has(s.id))
}

function isGroupPartialSelected(g) {
  const count = g.students.filter(s => selectedIds.value.has(s.id)).length
  return count > 0 && count < g.students.length
}

function toggleGroupAll(g) {
  const next = new Set(selectedIds.value)
  if (isGroupAllSelected(g)) {
    g.students.forEach(s => next.delete(s.id))
  } else {
    g.students.forEach(s => next.add(s.id))
  }
  selectedIds.value = next
}

function toggleStudent(id) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}

function submit() {
  emit('saved', [...selectedIds.value])
}
</script>

<template>
  <div class="overlay">
    <div class="modal">

      <!-- 헤더 -->
      <div class="modal-header">
        <div class="header-text">
          <span class="area-label">{{ area.name }}</span>
          <h2 class="modal-title">학생 배정</h2>
        </div>
        <button class="close-btn" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 바디 -->
      <div class="modal-body">
        <p v-if="allStudents.length === 0" class="empty-hint">
          등록된 학생이 없습니다.<br>학생 관리에서 먼저 추가하세요.
        </p>

        <div v-else class="group-list">
          <div
              v-for="g in groups"
              :key="groupKey(g)"
              class="group"
          >
            <!-- 그룹 헤더 -->
            <div class="group-header" @click="toggleGroup(groupKey(g))">
              <div class="group-header-left">
                <input
                    type="checkbox"
                    class="group-checkbox"
                    :checked="isGroupAllSelected(g)"
                    :indeterminate="isGroupPartialSelected(g)"
                    @change.stop="toggleGroupAll(g)"
                    @click.stop
                />
                <span class="group-name">{{ g.grade }}학년 {{ g.classNum }}반</span>
                <span class="group-count">{{ g.students.length }}명</span>
                <span v-if="isGroupPartialSelected(g) || isGroupAllSelected(g)" class="group-selected-count">
                  {{ g.students.filter(s => selectedIds.has(s.id)).length }}명 선택
                </span>
              </div>
              <ChevronDown v-if="isGroupExpanded(groupKey(g))" :size="16" class="chevron"/>
              <ChevronRight v-else :size="16" class="chevron"/>
            </div>

            <!-- 학생 목록 (펼쳐졌을 때) -->
            <div v-if="isGroupExpanded(groupKey(g))" class="student-list">
              <label
                  v-for="s in g.students"
                  :key="s.id"
                  class="student-item"
              >
                <input
                    type="checkbox"
                    class="student-checkbox"
                    :checked="selectedIds.has(s.id)"
                    @change="toggleStudent(s.id)"
                />
                <span class="student-number">{{ s.number }}번</span>
                <span class="student-name">{{ s.name }}</span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- 푸터 -->
      <div class="modal-footer">
        <span class="selected-count">{{ selectedIds.size }}명 선택됨</span>
        <div class="footer-right">
          <button class="btn-cancel" @click="emit('close')">취소</button>
          <button class="btn-submit" @click="submit">저장</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(4, 6, 12, 0.75);
  backdrop-filter: blur(6px);
}

.modal {
  width: 100%;
  max-width: 480px;
  max-height: 80vh;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
}

.header-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.area-label {
  font-size: 13px;
  color: #7ba8f0;
  font-weight: 500;
}

.modal-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: none;
  border: none;
  color: #5a7aaa;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
  flex-shrink: 0;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #93afd4;
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 0;
}

.modal-body::-webkit-scrollbar {
  width: 4px;
}

.modal-body::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 2px;
}

.empty-hint {
  font-size: 15px;
  color: #7ba3d4;
  line-height: 1.7;
  padding: 24px;
  margin: 0;
}

.group-list {
  display: flex;
  flex-direction: column;
}

.group {
  border-bottom: 1px solid #1a2035;
}

.group:last-child {
  border-bottom: none;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  cursor: pointer;
  transition: background-color 0.12s;
  user-select: none;
}

.group-header:hover {
  background-color: rgba(59, 91, 219, 0.05);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.group-checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: #3b5bdb;
  flex-shrink: 0;
}

.group-name {
  font-size: 15px;
  font-weight: 600;
  color: #c8d8f0;
}

.group-count {
  font-size: 13px;
  color: #5a7aaa;
}

.group-selected-count {
  font-size: 13px;
  color: #7ba8f0;
  background-color: rgba(59, 91, 219, 0.12);
  border-radius: 4px;
  padding: 1px 6px;
}

.chevron {
  color: #5a7aaa;
  flex-shrink: 0;
}

.student-list {
  display: flex;
  flex-direction: column;
  padding: 4px 0 8px 20px;
  background-color: rgba(8, 11, 20, 0.4);
}

.student-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 20px 8px 0;
  cursor: pointer;
  transition: background-color 0.1s;
}

.student-item:hover {
  background-color: rgba(59, 91, 219, 0.04);
}

.student-checkbox {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: #3b5bdb;
  flex-shrink: 0;
}

.student-number {
  font-size: 14px;
  color: #5a7aaa;
  width: 36px;
  flex-shrink: 0;
}

.student-name {
  font-size: 15px;
  color: #c8d8f0;
}

/* 푸터 */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 24px 18px;
  border-top: 1px solid #1a2035;
  flex-shrink: 0;
}

.selected-count {
  font-size: 15px;
  color: #7ba3d4;
}

.footer-right {
  display: flex;
  gap: 8px;
}

.btn-cancel {
  padding: 10px 20px;
  border-radius: 10px;
  border: 1px solid #1a2035;
  background: none;
  color: #7ba3d4;
  font-size: 16px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-cancel:hover {
  background-color: #1a2035;
}

.btn-submit {
  padding: 10px 24px;
  border-radius: 10px;
  border: none;
  background-color: #3b5bdb;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
  box-shadow: 0 4px 16px rgba(59, 91, 219, 0.2);
}

.btn-submit:hover {
  background-color: #4c6ef5;
}
</style>
