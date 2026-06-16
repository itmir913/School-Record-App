<script setup>
import {computed, onMounted, ref} from 'vue'
import {Layers, Plus} from 'lucide-vue-next'
import {useAreaStore} from '../stores/area'
import {useActivityStore} from '../stores/activity'
import {useStudentStore} from '../stores/student'
import AreaCard from '../components/AreaCard.vue'
import AreaModal from '../components/AreaModal.vue'
import AreaStudentModal from '../components/AreaStudentModal.vue'

const areaStore = useAreaStore()
const activityStore = useActivityStore()
const studentStore = useStudentStore()

const sortedAreas = computed(() =>
    [...areaStore.areas].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

// 영역 편집 모달 상태
const modalVisible = ref(false)
const modalMode = ref('add')       // 'add' | 'edit'
const selectedArea = ref(null)
const areaModalRef = ref(null)

// 학생 배정 모달 상태
const studentModalVisible = ref(false)
const studentModalArea = ref(null)
const studentModalInitialIds = ref([])
const areaStudentModalRef = ref(null)

const saving = ref(false)

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
  if (saving.value) return
  saving.value = true
  try {
    let areaId
    if (modalMode.value === 'add') {
      areaId = await areaStore.createArea(name, byteLimit)
    } else {
      areaId = selectedArea.value.id
      await areaStore.updateArea(areaId, name, byteLimit)
    }
    await areaStore.setAreaActivities(areaId, activityIds)
    await activityStore.fetchActivities()  // ActivityDetail.areas 갱신
    closeModal()
  } catch (e) {
    areaModalRef.value?.setServerError(String(e))
  } finally {
    saving.value = false
  }
}

async function handleDeleted() {
  try {
    await areaStore.deleteArea(selectedArea.value.id)
    closeModal()
  } catch (e) {
    areaModalRef.value?.setServerError(String(e))
  }
}

async function openStudentModal(area) {
  studentModalArea.value = area
  try {
    studentModalInitialIds.value = await areaStore.getAreaStudents(area.id)
  } catch (e) {
    studentModalInitialIds.value = []
    areaStore.error = `학생 목록을 불러오지 못했습니다: ${e}`
    return
  }
  studentModalVisible.value = true
}

function closeStudentModal() {
  studentModalVisible.value = false
  studentModalArea.value = null
}

async function handleStudentSaved(studentIds) {
  try {
    await areaStore.setAreaStudents(studentModalArea.value.id, studentIds)
    closeStudentModal()
  } catch (e) {
    areaStudentModalRef.value?.setServerError(String(e))
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex flex-col h-full overflow-hidden box-border">

      <!-- 섹션 헤더 -->
      <div class="flex items-start justify-between px-10 py-9 border-b border-line shrink-0 gap-4">
        <div>
          <h2 class="text-[22px] font-bold text-ink m-0 mb-[6px]">영역(Area) 관리</h2>
          <p class="text-base text-ink-3 m-0">자율활동, 진로활동, 동아리활동, 세부능력특기사항 등 생기부 대분류 영역을 설정합니다.</p>
        </div>
        <button
          class="flex items-center gap-2 px-5 py-[10px] rounded-xl bg-blue border-none text-white text-base font-semibold cursor-pointer whitespace-nowrap shrink-0 transition-colors duration-150 hover:bg-blue-2 shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)]"
          @click="openAddModal"
        >
          <Plus :size="18"/>
          영역 추가
        </button>
      </div>

      <div class="flex-1 overflow-y-auto px-10 pt-8 pb-12">
        <!-- 로딩 -->
        <div v-if="areaStore.loading" class="state-box">
          <p class="text-base text-ink-3 m-0">불러오는 중...</p>
        </div>

        <!-- 에러 -->
        <div v-else-if="areaStore.error" class="state-box state-box--error">
          <p class="text-base text-ink-3 m-0">{{ areaStore.error }}</p>
        </div>

        <!-- 빈 상태 -->
        <div v-else-if="areaStore.areas.length === 0" class="flex flex-col items-center justify-center gap-3 py-20 px-10 border border-dashed border-line rounded-[20px]">
          <Layers :size="40" class="text-ink-5"/>
          <p class="text-lg font-semibold text-ink-3 m-0">등록된 영역이 없습니다</p>
          <p class="text-base text-ink-4 m-0 mb-2">영역을 추가하여 학생부 구성을 시작하세요.</p>
          <button
            class="flex items-center gap-2 px-5 py-[10px] rounded-xl bg-blue border-none text-white text-base font-semibold cursor-pointer whitespace-nowrap shrink-0 transition-colors duration-150 hover:bg-blue-2 shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)]"
            @click="openAddModal"
          >
            <Plus :size="18"/>
            첫 영역 추가하기
          </button>
        </div>

        <!-- 카드 그리드 -->
        <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4">
          <AreaCard
              v-for="area in sortedAreas"
              :key="area.id"
              :area="area"
              @edit="openEditModal"
              @assign-students="openStudentModal"
          />
        </div>
      </div>
    </div>

    <!-- 영역 편집 모달 -->
    <transition name="modal">
      <AreaModal
          ref="areaModalRef"
          v-if="modalVisible"
          :mode="modalMode"
          :area="selectedArea"
          :all-activities="activityStore.activities"
          :submitting="saving"
          @close="closeModal"
          @saved="handleSaved"
          @deleted="handleDeleted"
      />
    </transition>

    <!-- 학생 배정 모달 -->
    <transition name="modal">
      <AreaStudentModal
          ref="areaStudentModalRef"
          v-if="studentModalVisible"
          :area="studentModalArea"
          :all-students="studentStore.students"
          :initial-student-ids="studentModalInitialIds"
          @close="closeStudentModal"
          @saved="handleStudentSaved"
      />
    </transition>
  </div>
</template>
