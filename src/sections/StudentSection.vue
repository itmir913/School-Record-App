<script setup>
import {computed, onMounted, ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {Pencil, Plus, TableProperties, Users} from 'lucide-vue-next'
import {useStudentStore} from '../stores/student'
import StudentModal from '../components/StudentModal.vue'
import BulkStudentImportModal from '../components/BulkStudentImportModal.vue'

const studentStore = useStudentStore()

const modalVisible = ref(false)
const modalMode = ref('add')
const selectedStudent = ref(null)
const bulkModalVisible = ref(false)

onMounted(() => {
  studentStore.fetchStudents()
})

// (학년, 반) 기준으로 그룹핑
const groupedStudents = computed(() => {
  const groups = []
  const seen = new Map()
  for (const s of studentStore.students) {
    const key = `${s.grade}-${s.class_num}`
    if (!seen.has(key)) {
      const group = {grade: s.grade, classNum: s.class_num, students: []}
      seen.set(key, group)
      groups.push(group)
    }
    seen.get(key).students.push(s)
  }
  return groups
})

function openAddModal() {
  selectedStudent.value = null
  modalMode.value = 'add'
  modalVisible.value = true
}

function openEditModal(student) {
  selectedStudent.value = student
  modalMode.value = 'edit'
  modalVisible.value = true
}

function closeModal() {
  modalVisible.value = false
  selectedStudent.value = null
}

async function handleSaved({grade, classNum, number, name}) {
  try {
    if (modalMode.value === 'add') {
      await studentStore.createStudent(grade, classNum, number, name)
    } else {
      await invoke('update_student', {
        id: selectedStudent.value.id,
        grade, classNum, number, name,
      })
      await studentStore.fetchStudents()
    }
    closeModal()
  } catch (e) {
    console.error(e)
  }
}

async function handleDeleted() {
  try {
    await studentStore.deleteStudent(selectedStudent.value.id)
    closeModal()
  } catch (e) {
    console.error(e)
  }
}
</script>

<template>
  <div class="section">

    <!-- 섹션 헤더 -->
    <div class="section-header">
      <div>
        <h2 class="section-title">학생(Students) 관리</h2>
        <p class="section-desc">
          <p>학교생활기록부 작성을 위한 학생 명단을 설정합니다.</p>
          <p>학생 정보를 등록하신 후, '영역(Area)' 탭에서 각 학생을 배정해 주세요.</p>
        </p>
      </div>
      <div class="header-actions">
        <button class="btn-bulk" @click="bulkModalVisible = true">
          <TableProperties :size="16"/>
          일괄 추가
        </button>
        <button class="btn-add" @click="openAddModal">
          <Plus :size="18"/>
          학생 추가
        </button>
      </div>
    </div>

    <!-- 로딩 -->
    <div v-if="studentStore.loading" class="state-box">
      <p class="state-text">불러오는 중...</p>
    </div>

    <!-- 에러 -->
    <div v-else-if="studentStore.error" class="state-box state-box--error">
      <p class="state-text">{{ studentStore.error }}</p>
    </div>

    <!-- 빈 상태 -->
    <div v-else-if="studentStore.students.length === 0" class="empty-state">
      <Users :size="40" color="#2a3a58"/>
      <p class="empty-title">등록된 학생이 없습니다</p>
      <p class="empty-desc">학생을 추가한 후 영역에 배정하세요.</p>
      <button class="btn-add" @click="openAddModal">
        <Plus :size="18"/>
        첫 학생 추가하기
      </button>
    </div>

    <!-- 학생 테이블 -->
    <div v-else class="table-wrap">
      <table class="student-table">
        <thead>
        <tr>
          <th>학년</th>
          <th>반</th>
          <th>번호</th>
          <th>이름</th>
          <th></th>
        </tr>
        </thead>
        <tbody>
        <template v-for="group in groupedStudents" :key="`${group.grade}-${group.classNum}`">
          <tr class="group-header-row">
            <td colspan="5">
              {{ group.grade }}학년 {{ group.classNum }}반
              <span class="group-count">{{ group.students.length }}명</span>
            </td>
          </tr>
          <tr
              v-for="student in group.students"
              :key="student.id"
              class="student-row"
          >
            <td>{{ student.grade }}</td>
            <td>{{ student.class_num }}</td>
            <td>{{ student.number }}</td>
            <td>{{ student.name }}</td>
            <td class="action-cell">
              <button class="btn-edit" @click="openEditModal(student)">
                <Pencil :size="14"/>
              </button>
            </td>
          </tr>
        </template>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 일괄 추가 모달 -->
  <transition name="modal">
    <BulkStudentImportModal
        v-if="bulkModalVisible"
        @close="bulkModalVisible = false"
        @imported="studentStore.fetchStudents()"
    />
  </transition>

  <!-- 학생 추가/수정 모달 -->
  <transition name="modal">
    <StudentModal
        v-if="modalVisible"
        :mode="modalMode"
        :student="selectedStudent"
        @close="closeModal"
        @saved="handleSaved"
        @deleted="handleDeleted"
    />
  </transition>
</template>

<style scoped>
.section {
  padding: 36px 40px;
  height: 100%;
  box-sizing: border-box;
  overflow-y: auto;
}

.section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 28px;
  gap: 16px;
}

.section-title {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
}

.section-desc {
  font-size: 16px;
  color: #7ba3d4;
  margin: 0;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.btn-bulk {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 10px 18px;
  border-radius: 12px;
  background: rgba(59, 91, 219, 0.1);
  border: 1px solid rgba(59, 91, 219, 0.3);
  color: #7ba8f0;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color 0.15s;
}

.btn-bulk:hover {
  background: rgba(59, 91, 219, 0.18);
}

.btn-add {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  border-radius: 12px;
  background-color: #3b5bdb;
  border: none;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: background-color 0.15s;
  box-shadow: 0 4px 16px rgba(59, 91, 219, 0.2);
}

.btn-add:hover {
  background-color: #4c6ef5;
}

.state-box {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px;
  border: 1px solid #1a2035;
  border-radius: 16px;
}

.state-box--error {
  border-color: rgba(239, 68, 68, 0.3);
}

.state-text {
  font-size: 16px;
  color: #7ba3d4;
  margin: 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 80px 40px;
  border: 1px dashed #1a2035;
  border-radius: 20px;
}

.empty-title {
  font-size: 18px;
  font-weight: 600;
  color: #7ba3d4;
  margin: 0;
}

.empty-desc {
  font-size: 16px;
  color: #5a7aaa;
  margin: 0 0 8px;
}

/* 테이블 */
.table-wrap {
  border: 1px solid #1a2035;
  border-radius: 16px;
  overflow: hidden;
}

.student-table {
  width: 100%;
  border-collapse: collapse;
}

.student-table th {
  font-size: 13px;
  font-weight: 600;
  color: #5a7aaa;
  text-align: left;
  padding: 12px 16px;
  background-color: #080b14;
  border-bottom: 1px solid #1a2035;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.group-header-row td {
  font-size: 14px;
  font-weight: 600;
  color: #7ba3d4;
  padding: 10px 16px;
  background-color: rgba(59, 91, 219, 0.05);
  border-top: 1px solid #1a2035;
  border-bottom: 1px solid #1a2035;
}

.group-count {
  font-size: 13px;
  font-weight: 400;
  color: #5a7aaa;
  margin-left: 8px;
}

.student-row td {
  font-size: 15px;
  color: #c8d8f0;
  padding: 11px 16px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.6);
}

.student-row:last-child td {
  border-bottom: none;
}

.student-row:hover td {
  background-color: rgba(59, 91, 219, 0.04);
}

.action-cell {
  text-align: right;
  width: 48px;
}

.btn-edit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 6px;
  border: none;
  background: none;
  color: #5a7aaa;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-edit:hover {
  background-color: #1a2035;
  color: #93afd4;
}

/* 모달 트랜지션 */
.modal-enter-from, .modal-leave-to {
  opacity: 0;
}

.modal-enter-active, .modal-leave-active {
  transition: opacity 0.2s;
}
</style>
