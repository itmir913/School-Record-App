<script setup>
import {onMounted, ref} from 'vue'
import {Layers, Plus} from 'lucide-vue-next'
import {invoke} from '@tauri-apps/api/core'
import {useAreaStore} from '../stores/area'
import {useActivityStore} from '../stores/activity'
import {useStudentStore} from '../stores/student'
import AreaCard from '../components/AreaCard.vue'
import AreaModal from '../components/AreaModal.vue'
import AreaStudentModal from '../components/AreaStudentModal.vue'

const areaStore = useAreaStore()
const activityStore = useActivityStore()
const studentStore = useStudentStore()

// 영역 편집 모달 상태
const modalVisible = ref(false)
const modalMode = ref('add')       // 'add' | 'edit'
const selectedArea = ref(null)

// 학생 배정 모달 상태
const studentModalVisible = ref(false)
const studentModalArea = ref(null)
const studentModalInitialIds = ref([])

onMounted(() => {
  areaStore.fetchAreas()
  activityStore.fetchActivities()
  studentStore.fetchStudents()
})

function openAddModal() {
  selectedArea.value = null
  modalMode.value = 'add'
  modalVisible.value = true
}

function openEditModal(area) {
  selectedArea.value = area
  modalMode.value = 'edit'
  modalVisible.value = true
}

function closeModal() {
  modalVisible.value = false
  selectedArea.value = null
}

async function handleSaved({name, byteLimit, activityIds}) {
  try {
    let areaId
    if (modalMode.value === 'add') {
      areaId = await invoke('create_area', {name, byteLimit})
    } else {
      areaId = selectedArea.value.id
      await invoke('update_area', {id: areaId, name, byteLimit})
    }
    await invoke('set_area_activities', {areaId, activityIds})
    await areaStore.fetchAreas()
    await activityStore.fetchActivities()  // ActivityDetail.areas 갱신
    closeModal()
  } catch (e) {
    console.error(e)
  }
}

async function handleDeleted() {
  try {
    await areaStore.deleteArea(selectedArea.value.id)
    closeModal()
  } catch (e) {
    console.error(e)
  }
}

async function openStudentModal(area) {
  studentModalArea.value = area
  try {
    studentModalInitialIds.value = await invoke('get_area_students', {areaId: area.id})
  } catch (e) {
    studentModalInitialIds.value = []
    console.error(e)
  }
  studentModalVisible.value = true
}

function closeStudentModal() {
  studentModalVisible.value = false
  studentModalArea.value = null
}

async function handleStudentSaved(studentIds) {
  try {
    await invoke('set_area_students', {areaId: studentModalArea.value.id, studentIds})
    closeStudentModal()
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
        <h2 class="section-title">영역(Area) 관리</h2>
        <p class="section-desc">자율활동, 동아리 등 생기부 대분류 영역을 설정합니다.</p>
      </div>
      <button class="btn-add" @click="openAddModal">
        <Plus :size="18"/>
        영역 추가
      </button>
    </div>

    <!-- 로딩 -->
    <div v-if="areaStore.loading" class="state-box">
      <p class="state-text">불러오는 중...</p>
    </div>

    <!-- 에러 -->
    <div v-else-if="areaStore.error" class="state-box state-box--error">
      <p class="state-text">{{ areaStore.error }}</p>
    </div>

    <!-- 빈 상태 -->
    <div v-else-if="areaStore.areas.length === 0" class="empty-state">
      <Layers :size="40" color="#2a3a58"/>
      <p class="empty-title">등록된 영역이 없습니다</p>
      <p class="empty-desc">영역을 추가하여 학생부 구성을 시작하세요.</p>
      <button class="btn-add" @click="openAddModal">
        <Plus :size="18"/>
        첫 영역 추가하기
      </button>
    </div>

    <!-- 카드 그리드 -->
    <div v-else class="card-grid">
      <AreaCard
          v-for="area in areaStore.areas"
          :key="area.id"
          :area="area"
          @edit="openEditModal"
          @assign-students="openStudentModal"
      />
    </div>
  </div>

  <!-- 영역 편집 모달 -->
  <transition name="modal">
    <AreaModal
        v-if="modalVisible"
        :mode="modalMode"
        :area="selectedArea"
        :all-activities="activityStore.activities"
        @close="closeModal"
        @saved="handleSaved"
        @deleted="handleDeleted"
    />
  </transition>

  <!-- 학생 배정 모달 -->
  <transition name="modal">
    <AreaStudentModal
        v-if="studentModalVisible"
        :area="studentModalArea"
        :all-students="studentStore.students"
        :initial-student-ids="studentModalInitialIds"
        @close="closeStudentModal"
        @saved="handleStudentSaved"
    />
  </transition>
</template>

<style scoped>
.section {
  padding: 36px 40px;
  height: 100%;
  box-sizing: border-box;
}

/* 헤더 */
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

/* 상태 박스 */
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

/* 빈 상태 */
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

/* 카드 그리드 */
.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

/* 모달 트랜지션 */
.modal-enter-from, .modal-leave-to {
  opacity: 0;
}

.modal-enter-active, .modal-leave-active {
  transition: opacity 0.2s;
}
</style>
