<script setup>
import {computed, ref, watch} from 'vue'
import {AlertTriangle, Trash2} from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  area: {type: Object, default: null},
  allActivities: {type: Array, default: () => []},
  submitting: {type: Boolean, default: false},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const name = ref('')
const byteLimit = ref(1500)
const error = ref('')
const confirmDelete = ref(false)
const selectedIds = ref(new Set())
const sortedActivities = computed(() =>
    [...props.allActivities].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

watch(
    () => props.area,
    (a) => {
      if (a) {
        name.value = a.name
        byteLimit.value = a.byte_limit
        selectedIds.value = new Set(a.activities.map(x => x.id))
      } else {
        name.value = ''
        byteLimit.value = 1500
        selectedIds.value = new Set()
      }
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

function validate() {
  if (!name.value.trim()) {
    error.value = '영역 이름을 입력해주세요.'
    return false
  }
  if (!byteLimit.value || byteLimit.value <= 0) {
    error.value = '바이트 수 제한은 1 이상이어야 합니다.'
    return false
  }
  return true
}

function toggleActivity(id) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}

function submit() {
  if (!validate()) return
  error.value = ''
  emit('saved', {
    name: name.value.trim(),
    byteLimit: Number(byteLimit.value),
    activityIds: [...selectedIds.value],
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
      :title="mode === 'add' ? '영역 추가' : '영역 수정'"
      max-width="920px"
      @close="emit('close')"
  >
    <!-- 2단 바디 -->
    <div class="flex items-stretch pt-5 pb-1 min-h-[380px]">

      <!-- 좌측: 기본 정보 -->
      <div class="flex flex-col gap-[18px] flex-1 px-6 pb-4">
        <p class="text-base font-semibold text-ink-5 tracking-[0.04em] uppercase m-0">기본 정보</p>

        <div class="flex flex-col gap-1.5">
          <label class="text-base font-semibold text-ink-3">영역 이름 <span class="text-red">*</span></label>
          <input
              v-model="name"
              class="ui-input placeholder:text-ink-5"
              placeholder="예: 자율활동, 진로활동"
              @keydown.enter="submit"
          />
        </div>

        <div class="flex flex-col gap-1.5">
          <label class="text-base font-semibold text-ink-3">바이트 수 제한 <span class="text-red">*</span></label>
          <div class="flex items-center gap-2">
            <input
                v-model.number="byteLimit"
                type="number"
                min="1"
                class="ui-input flex-1 placeholder:text-ink-5"
                placeholder="1500"
                @keydown.enter="submit"
            />
            <span class="text-base text-ink-5 whitespace-nowrap">Bytes</span>
          </div>
          <p class="text-base text-ink-5 m-0 text-right">나이스 기준 최대 입력 가능한 바이트 수</p>
        </div>

        <!-- 삭제 경고 (편집 + 확인 단계) -->
        <div v-if="mode === 'edit' && confirmDelete"
             class="bg-red/[7%] border border-red/25 rounded-btn px-4 py-3.5 flex flex-col gap-2 mt-auto">
          <div class="flex items-center gap-2">
            <AlertTriangle :size="16" class="text-red shrink-0"/>
            <span class="text-base font-semibold text-red">정말 삭제하시겠습니까?</span>
          </div>
          <p class="text-base text-red/80 m-0 leading-relaxed">
            이 영역을 삭제하시겠습니까? 영역 정보만 삭제되며, 이 영역과 연결된 활동과 학생의 생기부 문장은 그대로 유지됩니다.
          </p>
        </div>

        <!-- 에러 -->
        <p v-if="error" class="msg-error">{{ error }}</p>
      </div>

      <!-- 구분선 -->
      <div class="w-px bg-line shrink-0 my-1 mb-5"/>

      <!-- 우측: 활동 선택 -->
      <div class="flex flex-col flex-1 px-6 pb-4 gap-4">
        <div class="flex items-center justify-between">
          <p class="text-base font-semibold text-ink-5 tracking-[0.04em] uppercase m-0">포함할 활동</p>
          <span v-if="allActivities.length > 0" class="text-base text-ink-5">
            {{ selectedIds.size }}개 선택됨
          </span>
        </div>

        <p v-if="allActivities.length === 0" class="text-base text-ink-5 leading-[1.7] m-0">
          등록된 활동이 없습니다.<br>활동 관리에서 먼저 추가하세요.
        </p>
        <div v-else class="flex flex-wrap content-start gap-2 flex-1 overflow-y-auto pr-1">
          <button
              v-for="act in sortedActivities"
              :key="act.id"
              type="button"
              class="px-4 py-[7px] rounded-full text-base font-medium cursor-pointer border transition-colors whitespace-nowrap"
              :class="selectedIds.has(act.id)
                ? 'border-blue/45 bg-blue/15 text-blue-2 hover:bg-blue/[22%]'
                : 'border-line bg-base text-ink-5 hover:border-ink-4 hover:text-blue-2'"
              @click="toggleActivity(act.id)"
          >{{ act.name }}
          </button>
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
                class="px-3 py-1.5 rounded-lg bg-red/80 border-none text-white cursor-pointer text-base font-semibold transition-colors hover:bg-red"
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
