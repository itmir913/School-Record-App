<script setup>
import {ref, watch} from 'vue'
import {AlertTriangle, Trash2} from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  student: {type: Object, default: null},
  submitting: {type: Boolean, default: false},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const grade = ref('')
const classNum = ref('')
const number = ref('')
const name = ref('')
const error = ref('')
const confirmDelete = ref(false)

watch(
    () => props.student,
    (s) => {
      grade.value = s ? String(s.grade) : ''
      classNum.value = s ? String(s.class_num) : ''
      number.value = s ? String(s.number) : ''
      name.value = s ? s.name : ''
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

function validate() {
  if (!grade.value || isNaN(Number(grade.value)) || Number(grade.value) < 1) {
    error.value = '올바른 학년을 입력해주세요.'
    return false
  }
  if (!classNum.value || isNaN(Number(classNum.value)) || Number(classNum.value) < 1) {
    error.value = '올바른 반을 입력해주세요.'
    return false
  }
  if (!number.value || isNaN(Number(number.value)) || Number(number.value) < 1) {
    error.value = '올바른 번호를 입력해주세요.'
    return false
  }
  if (!name.value.trim()) {
    error.value = '이름을 입력해주세요.'
    return false
  }
  return true
}

function submit() {
  if (!validate()) return
  error.value = ''
  emit('saved', {
    grade: Number(grade.value),
    classNum: Number(classNum.value),
    number: Number(number.value),
    name: name.value.trim(),
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
      :title="mode === 'add' ? '학생 추가' : '학생 수정'"
      max-width="460px"
      @close="emit('close')"
  >
    <!-- 바디 -->
    <div class="flex flex-col gap-4 px-6 pt-5 pb-2">

      <div class="flex gap-3">
        <div class="flex flex-col gap-[6px] flex-1 min-w-0">
          <label class="text-base font-semibold text-ink-3">학년 <span class="text-red">*</span></label>
          <input
              v-model="grade"
              class="ui-input"
              type="number"
              min="1"
              placeholder="3"
              @keydown.enter="submit"
          />
        </div>
        <div class="flex flex-col gap-[6px] flex-1 min-w-0">
          <label class="text-base font-semibold text-ink-3">반 <span class="text-red">*</span></label>
          <input
              v-model="classNum"
              class="ui-input"
              type="number"
              min="1"
              placeholder="2"
              @keydown.enter="submit"
          />
        </div>
        <div class="flex flex-col gap-[6px] flex-1 min-w-0">
          <label class="text-base font-semibold text-ink-3">번호 <span class="text-red">*</span></label>
          <input
              v-model="number"
              class="ui-input"
              type="number"
              min="1"
              placeholder="15"
              @keydown.enter="submit"
          />
        </div>
      </div>

      <div class="flex flex-col gap-[6px]">
        <label class="text-base font-semibold text-ink-3">이름 <span class="text-red">*</span></label>
        <input
            v-model="name"
            class="ui-input"
            placeholder="홍길동"
            @keydown.enter="submit"
        />
      </div>

      <!-- 삭제 경고 -->
      <div v-if="mode === 'edit' && confirmDelete" class="bg-red/7 border border-red/25 rounded-btn p-4 flex flex-col gap-2">
        <div class="flex items-center gap-2">
          <AlertTriangle :size="16" class="text-red shrink-0"/>
          <span class="text-base font-semibold text-red">정말 삭제하시겠습니까?</span>
        </div>
        <p class="text-base text-red/80 m-0 leading-[1.6]">
          이 학생을 삭제하면 <strong class="text-red font-semibold">모든 활동 기록과 스냅샷 정보가 함께 삭제</strong>되며, 이후 스냅샷을 이용한 복구도 불가능합니다.
        </p>
      </div>

      <p v-if="error" class="msg-error">{{ error }}</p>
    </div>

    <!-- 푸터 -->
    <template #footer>
      <div class="flex items-center gap-2">
        <template v-if="mode === 'edit'">
          <button v-if="!confirmDelete" class="btn-danger" @click="handleDelete">
            <Trash2 :size="15"/>
            삭제
          </button>
          <div v-else class="flex gap-2">
            <button
              class="px-[14px] py-2 rounded-lg border border-line bg-transparent text-ink-3 text-base cursor-pointer transition-colors duration-150 hover:bg-line"
              @click="confirmDelete = false"
            >취소</button>
            <button
              class="btn-delete-confirm px-[14px] py-2 rounded-lg border-none bg-red text-white text-base font-semibold cursor-pointer transition-colors duration-150"
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

<style scoped>
.ui-input::placeholder { color: var(--c-ink-5); }
.btn-delete-confirm:hover { background-color: color-mix(in srgb, var(--c-red) 80%, #000 20%); }
</style>
