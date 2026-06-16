<script setup>
import {computed, onMounted, ref} from 'vue'
import {Pencil, Plus, TableProperties, Users} from 'lucide-vue-next'
import {useStudentStore} from '../stores/student'
import StudentModal from '../components/StudentModal.vue'
import StudentBulkImportModal from '../components/StudentBulkImportModal.vue'

const studentStore = useStudentStore()

const modalVisible = ref(false)
const modalMode = ref('add')
const selectedStudent = ref(null)
const studentModalRef = ref(null)
const bulkModalVisible = ref(false)
const saving = ref(false)

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
  if (saving.value) return
  saving.value = true
  try {
    if (modalMode.value === 'add') {
      await studentStore.createStudent(grade, classNum, number, name)
    } else {
      await studentStore.updateStudent(selectedStudent.value.id, grade, classNum, number, name)
    }
    closeModal()
  } catch (e) {
    studentModalRef.value?.setServerError(String(e))
  } finally {
    saving.value = false
  }
}

async function handleDeleted() {
  try {
    await studentStore.deleteStudent(selectedStudent.value.id)
    closeModal()
  } catch (e) {
    studentModalRef.value?.setServerError(String(e))
  }
}
</script>

<template>
  <div>
    <div class="flex flex-col h-full overflow-hidden box-border">

      <!-- 섹션 헤더 -->
      <div class="flex items-start justify-between px-10 py-9 border-b border-line flex-shrink-0 gap-4">
        <div>
          <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">학생(Students) 관리</h2>
          <div class="text-base text-ink-3">
            <p>학교생활기록부 작성을 위한 학생 명단을 설정합니다.</p>
            <p>학생 정보를 등록하신 후, '영역(Area)' 탭에서 각 학생을 배정해 주세요.</p>
          </div>
        </div>
        <div class="flex items-center gap-2 flex-shrink-0">
          <button
              class="flex items-center gap-[7px] px-[18px] py-2.5 rounded-xl bg-blue/10 border border-blue/30 text-[#7ba8f0] text-base font-semibold cursor-pointer whitespace-nowrap transition-colors enabled:hover:bg-blue/[0.18]"
              @click="bulkModalVisible = true"
          >
            <TableProperties :size="16"/>
            일괄 추가
          </button>
          <button
              class="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue border-0 text-white text-base font-semibold cursor-pointer whitespace-nowrap flex-shrink-0 transition-colors shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)] hover:bg-blue-2"
              @click="openAddModal"
          >
            <Plus :size="18"/>
            학생 추가
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto px-10 py-8 pb-12">

        <!-- 로딩 -->
        <div v-if="studentStore.loading">
          <p class="text-base text-ink-3 m-0">불러오는 중...</p>
        </div>

        <!-- 에러 -->
        <div v-else-if="studentStore.error">
          <p class="text-base text-red m-0">{{ studentStore.error }}</p>
        </div>

        <!-- 빈 상태 -->
        <div v-else-if="studentStore.students.length === 0"
             class="flex flex-col items-center justify-center gap-3 px-10 py-20 border border-dashed border-line rounded-[20px]">
          <Users :size="40" class="text-ink-5"/>
          <p class="text-lg font-semibold text-ink-3 m-0">등록된 학생이 없습니다</p>
          <p class="text-base text-ink-5 m-0 mb-2">학생을 추가한 후 영역에 배정하세요.</p>
          <button
              class="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue border-0 text-white text-base font-semibold cursor-pointer whitespace-nowrap flex-shrink-0 transition-colors shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)] hover:bg-blue-2"
              @click="openAddModal"
          >
            <Plus :size="18"/>
            첫 학생 추가하기
          </button>
        </div>

        <!-- 학생 테이블 -->
        <div v-else class="border border-line rounded-2xl overflow-hidden">
          <table class="w-full border-collapse student-table">
            <thead>
            <tr>
              <th class="text-sm font-semibold text-ink-5 text-left px-4 py-3 bg-base border-b border-line tracking-[0.04em] uppercase">학년</th>
              <th class="text-sm font-semibold text-ink-5 text-left px-4 py-3 bg-base border-b border-line tracking-[0.04em] uppercase">반</th>
              <th class="text-sm font-semibold text-ink-5 text-left px-4 py-3 bg-base border-b border-line tracking-[0.04em] uppercase">번호</th>
              <th class="text-sm font-semibold text-ink-5 text-left px-4 py-3 bg-base border-b border-line tracking-[0.04em] uppercase">이름</th>
              <th class="text-sm font-semibold text-ink-5 text-left px-4 py-3 bg-base border-b border-line tracking-[0.04em] uppercase"></th>
            </tr>
            </thead>
            <tbody>
            <template v-for="group in groupedStudents" :key="`${group.grade}-${group.classNum}`">
              <tr>
                <td colspan="5" class="text-sm font-semibold text-[#7ba3d4] px-4 py-2.5 bg-blue/[0.05] border-t border-line border-b border-line">
                  {{ group.grade }}학년 {{ group.classNum }}반
                  <span class="text-sm font-normal text-ink-5 ml-2">{{ group.students.length }}명</span>
                </td>
              </tr>
              <tr
                  v-for="student in group.students"
                  :key="student.id"
                  class="group student-row"
              >
                <td class="text-base text-ink-2 px-4 py-[11px] border-b border-line/60 group-hover:bg-blue/[0.04]">{{ student.grade }}</td>
                <td class="text-base text-ink-2 px-4 py-[11px] border-b border-line/60 group-hover:bg-blue/[0.04]">{{ student.class_num }}</td>
                <td class="text-base text-ink-2 px-4 py-[11px] border-b border-line/60 group-hover:bg-blue/[0.04]">{{ student.number }}</td>
                <td class="text-base text-ink-2 px-4 py-[11px] border-b border-line/60 group-hover:bg-blue/[0.04]">{{ student.name }}</td>
                <td class="text-right w-12 px-4 py-[11px] border-b border-line/60 group-hover:bg-blue/[0.04]">
                  <button
                      class="inline-flex items-center justify-center w-[30px] h-[30px] rounded-md border-0 bg-transparent text-ink-5 cursor-pointer transition-[background-color,color] hover:bg-line hover:text-ink-3"
                      @click="openEditModal(student)"
                  >
                    <Pencil :size="14"/>
                  </button>
                </td>
              </tr>
            </template>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 일괄 추가 모달 -->
    <transition
        enter-from-class="opacity-0"
        leave-to-class="opacity-0"
        enter-active-class="transition-opacity duration-200"
        leave-active-class="transition-opacity duration-200"
    >
      <StudentBulkImportModal
          v-if="bulkModalVisible"
          @close="bulkModalVisible = false"
          @imported="studentStore.fetchStudents()"
      />
    </transition>

    <!-- 학생 추가/수정 모달 -->
    <transition
        enter-from-class="opacity-0"
        leave-to-class="opacity-0"
        enter-active-class="transition-opacity duration-200"
        leave-active-class="transition-opacity duration-200"
    >
      <StudentModal
          ref="studentModalRef"
          v-if="modalVisible"
          :mode="modalMode"
          :student="selectedStudent"
          :submitting="saving"
          @close="closeModal"
          @saved="handleSaved"
          @deleted="handleDeleted"
      />
    </transition>
  </div>
</template>

<style scoped>
.student-row:last-child td {
  border-bottom: none;
}
</style>
