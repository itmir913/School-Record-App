<script setup>
import {computed, onMounted, ref} from 'vue'
import {BookOpen, Plus} from 'lucide-vue-next'
import {useActivityStore} from '../stores/activity'
import {useAreaStore} from '../stores/area'
import ActivityCard from '../components/ActivityCard.vue'
import ActivityModal from '../components/ActivityModal.vue'

const activityStore = useActivityStore()
const areaStore = useAreaStore()

const sortedActivities = computed(() =>
    [...activityStore.activities].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

// 모달 상태
const modalVisible = ref(false)
const modalMode = ref('add')       // 'add' | 'edit'
const selectedActivity = ref(null)
const activityModalRef = ref(null)
const isSubmitting = ref(false)

onMounted(() => {
  activityStore.fetchActivities()
  areaStore.fetchAreas()
})

function openAddModal() {
  selectedActivity.value = null
  modalMode.value = 'add'
  modalVisible.value = true
}

function openEditModal(activity) {
  selectedActivity.value = activity
  modalMode.value = 'edit'
  modalVisible.value = true
}

function closeModal() {
  modalVisible.value = false
  selectedActivity.value = null
}

async function handleSaved({name, areaIds}) {
  if (isSubmitting.value) return
  isSubmitting.value = true
  try {
    await activityStore.saveActivity({
      mode: modalMode.value,
      id: selectedActivity.value?.id,
      name,
      areaIds,
    })
    await areaStore.fetchAreas()
    closeModal()
  } catch (e) {
    activityModalRef.value?.setServerError(String(e))
  } finally {
    isSubmitting.value = false
  }
}

async function handleDeleted() {
  if (isSubmitting.value) return
  isSubmitting.value = true
  try {
    await activityStore.deleteActivity(selectedActivity.value.id)
    closeModal()
  } catch (e) {
    activityModalRef.value?.setServerError(String(e))
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex flex-col h-full overflow-hidden box-border">

      <!-- 섹션 헤더 -->
      <div class="flex items-start justify-between px-10 py-9 border-b border-line shrink-0 gap-4">
        <div>
          <h2 class="text-[22px] font-bold text-ink m-0 mb-[6px]">활동(Activity) 관리</h2>
          <div class="text-base text-ink-3">
            <p class="m-0">생기부 각 활동을 설정하고 해당 영역에 연결합니다.</p>
            <p class="m-0">예: 현재 탭에서 '학생자치회', '교내캠페인', '안전교육' 활동 생성 → '자율활동' 영역으로 연결</p>
          </div>
        </div>
        <button
          class="flex items-center gap-2 px-5 py-[10px] rounded-xl bg-blue border-none text-white text-base font-semibold cursor-pointer whitespace-nowrap shrink-0 transition-colors duration-150 hover:bg-blue-2 shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)]"
          @click="openAddModal"
        >
          <Plus :size="18"/>
          활동 추가
        </button>
      </div>

      <div class="flex-1 overflow-y-auto px-10 pt-8 pb-12">
        <!-- 로딩 -->
        <div v-if="activityStore.loading" class="state-box">
          <p class="text-base text-ink-3 m-0">불러오는 중...</p>
        </div>

        <!-- 에러 -->
        <div v-else-if="activityStore.error" class="state-box state-box--error">
          <p class="text-base text-ink-3 m-0">{{ activityStore.error }}</p>
        </div>

        <!-- 빈 상태 -->
        <div v-else-if="activityStore.activities.length === 0" class="flex flex-col items-center justify-center gap-3 py-20 px-10 border border-dashed border-line rounded-[20px]">
          <BookOpen :size="40" class="text-ink-5"/>
          <p class="text-lg font-semibold text-ink-3 m-0">등록된 활동이 없습니다</p>
          <p class="text-base text-ink-4 m-0 mb-2">활동을 추가한 후 영역에 연결하세요.</p>
          <button
            class="flex items-center gap-2 px-5 py-[10px] rounded-xl bg-blue border-none text-white text-base font-semibold cursor-pointer whitespace-nowrap shrink-0 transition-colors duration-150 hover:bg-blue-2 shadow-[0_4px_16px_color-mix(in_srgb,var(--c-blue)_20%,transparent)]"
            @click="openAddModal"
          >
            <Plus :size="18"/>
            첫 활동 추가하기
          </button>
        </div>

        <!-- 카드 그리드 -->
        <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4">
          <ActivityCard
              v-for="activity in sortedActivities"
              :key="activity.id"
              :activity="activity"
              @edit="openEditModal"
          />
        </div>
      </div>
    </div>

    <!-- 모달 -->
    <transition name="modal">
      <ActivityModal
          ref="activityModalRef"
          v-if="modalVisible"
          :mode="modalMode"
          :activity="selectedActivity"
          :all-areas="areaStore.areas"
          :submitting="isSubmitting"
          @close="closeModal"
          @saved="handleSaved"
          @deleted="handleDeleted"
      />
    </transition>
  </div>
</template>
