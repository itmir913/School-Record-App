<script setup>
import {computed, ref, watch} from 'vue'
import {AlertTriangle, Info, Trash2} from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  activity: {type: Object, default: null},
  allAreas: {type: Array, default: () => []},
  submitting: {type: Boolean, default: false},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const name = ref('')
const error = ref('')
const confirmDelete = ref(false)
const selectedAreaIds = ref(new Set())

watch(
    () => props.activity,
    (a) => {
      name.value = a ? a.name : ''
      selectedAreaIds.value = new Set(a ? a.areas.map(x => x.id) : [])
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

const multiAreaWarning = computed(() => selectedAreaIds.value.size >= 2)
const sortedAreas = computed(() =>
    [...props.allAreas].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

function toggleArea(id) {
  const next = new Set(selectedAreaIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedAreaIds.value = next
}

function validate() {
  if (!name.value.trim()) {
    error.value = '활동 이름을 입력해주세요.'
    return false
  }
  return true
}

function submit() {
  if (!validate()) return
  error.value = ''
  emit('saved', {
    name: name.value.trim(),
    areaIds: [...selectedAreaIds.value],
  })
}

function setServerError(msg) {
  error.value = msg
}

defineExpose({ setServerError })

function handleDelete() {
  if (!confirmDelete.value) {
    confirmDelete.value = true
    return
  }
  emit('deleted')
}
</script>

<template>
  <BaseModal
      :title="mode === 'add' ? '활동 추가' : '활동 수정'"
      max-width="920px"
      @close="emit('close')"
  >
    <!-- 2단 바디 -->
    <div class="flex items-stretch pt-5 pb-1 min-h-[380px]">

      <!-- 좌측: 기본 정보 -->
      <div class="flex flex-col gap-4 flex-1 px-6 pb-4">
        <p class="text-sm font-semibold text-ink-5 tracking-[0.06em] uppercase m-0">기본 정보</p>

        <div class="flex flex-col gap-1.5">
          <label class="text-base font-semibold text-ink-3">활동 이름 <span class="text-red">*</span></label>
          <input
              v-model="name"
              class="ui-input placeholder:text-ink-5"
              placeholder="예: 학급자치활동"
              @keydown.enter="submit"
          />
          <p class="text-base text-ink-5 m-0 leading-relaxed">
            영역(Area) 안에 포함될 세부 활동명입니다.
          </p>
        </div>

        <!-- 삭제 경고 (편집 + 확인 단계) -->
        <div v-if="mode === 'edit' && confirmDelete"
             class="bg-red/[7%] border border-red/25 rounded-btn px-4 py-3.5 flex flex-col gap-2 mt-auto">
          <div class="flex items-center gap-2">
            <AlertTriangle :size="16" class="text-red shrink-0"/>
            <span class="text-base font-semibold text-red">정말 삭제하시겠습니까?</span>
          </div>
          <p class="text-base text-red/80 m-0 leading-relaxed">
            이 활동을 삭제하면 이 활동에 속한 <strong class="text-red font-semibold">학생의 생기부 문장과 스냅샷 정보가 모두 삭제</strong>되며, 스냅샷으로도 복구할 수 없습니다.
          </p>
        </div>

        <!-- 에러 -->
        <p v-if="error" class="msg-error">{{ error }}</p>
      </div>

      <!-- 구분선 -->
      <div class="w-px bg-line shrink-0 my-1 mb-5"/>

      <!-- 우측: 영역 선택 -->
      <div class="flex flex-col flex-1 px-6 pb-4 gap-4">
        <div class="flex items-center justify-between">
          <p class="text-sm font-semibold text-ink-5 tracking-[0.06em] uppercase m-0">포함할 영역</p>
          <span v-if="allAreas.length > 0" class="text-sm text-ink-5">
            {{ selectedAreaIds.size }}개 선택
          </span>
        </div>

        <p v-if="allAreas.length === 0" class="text-base text-ink-5 leading-[1.7] m-0">
          등록된 영역이 없습니다.<br>영역 관리에서 먼저 추가하세요.
        </p>
        <div v-else class="flex flex-wrap content-start gap-2 flex-1 overflow-y-auto pr-1">
          <button
              v-for="area in sortedAreas"
              :key="area.id"
              type="button"
              class="px-4 py-[7px] rounded-full text-base font-medium cursor-pointer border transition-colors whitespace-nowrap"
              :class="selectedAreaIds.has(area.id)
                ? 'border-blue/45 bg-blue/15 text-blue-2 hover:bg-blue/[22%]'
                : 'border-line bg-base text-ink-5 hover:border-ink-4 hover:text-blue-2'"
              @click="toggleArea(area.id)"
          >{{ area.name }}
          </button>
        </div>

        <!-- 복수 영역 선택 시 안내 -->
        <div v-if="multiAreaWarning"
             class="flex items-start gap-2 bg-amber/[7%] border border-amber/20 rounded-btn px-3.5 py-3 mt-1">
          <Info :size="15" class="text-amber shrink-0 mt-[1px]"/>
          <p class="text-base text-amber m-0 leading-relaxed">
            일반적으로 하나의 활동은 하나의 영역에만 포함됩니다. 여러 영역에 중복 배치하는 경우는 드문 편이므로, 의도된 구성인지 확인하세요.
          </p>
        </div>
      </div>
    </div>

    <!-- 푸터 -->
    <template #footer>
      <div class="flex items-center">
        <template v-if="mode === 'edit'">
          <button
              v-if="!confirmDelete"
              class="btn-danger flex items-center gap-1.5"
              @click="handleDelete"
          >
            <Trash2 :size="15"/>
            삭제
          </button>
          <div v-else class="flex items-center gap-2">
            <button
                class="px-3 py-1.5 rounded-lg bg-line border-none text-ink-3 cursor-pointer text-base transition-colors hover:bg-line-2"
                @click="confirmDelete = false"
            >취소</button>
            <button
                class="px-3.5 py-1.5 rounded-lg bg-red/85 border-none text-white cursor-pointer text-base font-semibold transition-colors hover:bg-red"
                @click="handleDelete"
            >영구 삭제</button>
          </div>
        </template>
      </div>

      <div class="flex items-center gap-2">
        <button class="btn-secondary" @click="emit('close')">취소</button>
        <button class="btn-primary" :disabled="submitting" @click="submit">
          {{ mode === 'add' ? '추가' : '저장' }}
        </button>
      </div>
    </template>
  </BaseModal>
</template>
