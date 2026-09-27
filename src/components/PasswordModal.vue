<script setup>
import {ref, computed, watch} from 'vue'
import {Lock, Eye, EyeOff, AlertTriangle} from '@lucide/vue'
import {useEscapeKey} from '../composables/useEscapeKey.js'

const props = defineProps({
  // 'unlock' | 'setup' | 'change'
  mode: {type: String, required: true},
  error: {type: String, default: ''},
  loading: {type: Boolean, default: false},
})

const emit = defineEmits(['submit', 'cancel'])

// 진행 중에는 취소를 막는다 — 취소 버튼도 같은 조건으로 비활성화되어 있다.
// ESC만 열어두면 잠금 해제 도중에 파일이 닫혀 열기 절차가 중간에 끊긴다.
useEscapeKey(() => {
  if (!props.loading) emit('cancel')
})

const password = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const showNewPassword = ref(false)
const showConfirmPassword = ref(false)
const localError = ref('')

const isSetupMode = computed(() => props.mode === 'setup')
const isChangeMode = computed(() => props.mode === 'change')

// 새로 설정하는 비밀번호의 최소 길이. 백엔드(crypto.rs MIN_PASSWORD_LEN)와 같은 값이며,
// 최종 판정도 백엔드가 한다. 여기서 먼저 막는 것은 왕복 없이 바로 알려주기 위해서다.
const MIN_PASSWORD_LENGTH = 4

// 코드 포인트로 센다. "가나다"는 9바이트지만 3글자이므로, 백엔드의 chars().count()와
// 세는 기준을 맞춰야 화면과 서버의 판정이 갈리지 않는다.
function charLength(value) {
  return [...value].length
}

const title = computed(() => {
  if (props.mode === 'unlock') return '비밀번호 확인'
  if (props.mode === 'setup') return '암호화 비밀번호 설정'
  return '비밀번호 변경'
})

const passwordPlaceholder = computed(() => {
  if (props.mode === 'change') return '현재 비밀번호'
  if (props.mode === 'setup') return `비밀번호 입력 (최소 ${MIN_PASSWORD_LENGTH}자)`
  return '비밀번호 입력'
})

const submitLabel = computed(() => {
  if (props.mode === 'unlock') return '잠금 해제'
  if (props.mode === 'setup') return '암호화 활성화'
  return '비밀번호 변경'
})

watch(() => props.error, (val) => {
  if (val) localError.value = val
})

function validate() {
  localError.value = ''
  if (!password.value) {
    localError.value = '비밀번호를 입력해주세요.'
    return false
  }

  if (isSetupMode.value) {
    // 하한은 새로 정하는 비밀번호에만 건다. 잠금 해제(unlock)에 걸면 이 규칙이
    // 생기기 전에 짧은 비밀번호로 암호화한 파일을 열 수 없게 된다.
    if (charLength(password.value) < MIN_PASSWORD_LENGTH) {
      localError.value = `비밀번호는 최소 ${MIN_PASSWORD_LENGTH}자 이상이어야 합니다.`
      return false
    }
    if (password.value !== confirmPassword.value) {
      localError.value = '비밀번호와 확인 비밀번호가 일치하지 않습니다.'
      return false
    }
  }

  if (isChangeMode.value) {
    if (!newPassword.value) {
      localError.value = '새 비밀번호를 입력해주세요.'
      return false
    }
    if (charLength(newPassword.value) < MIN_PASSWORD_LENGTH) {
      localError.value = `새 비밀번호는 최소 ${MIN_PASSWORD_LENGTH}자 이상이어야 합니다.`
      return false
    }
    if (newPassword.value !== confirmPassword.value) {
      localError.value = '새 비밀번호와 확인 비밀번호가 일치하지 않습니다.'
      return false
    }
  }

  return true
}

function handleSubmit() {
  // 버튼은 loading 중 비활성이지만 입력칸의 Enter는 막히지 않는다. 두 번 보내면
  // 열기 절차가 두 번 돌고, 비밀번호 변경은 두 번째 요청이 옛 비밀번호로 실패한다.
  if (props.loading) return
  if (!validate()) return
  if (props.mode === 'unlock') {
    emit('submit', {password: password.value})
  } else if (props.mode === 'setup') {
    emit('submit', {password: password.value})
  } else {
    emit('submit', {oldPassword: password.value, newPassword: newPassword.value})
  }
}

function handleCancel() {
  emit('cancel')
}
</script>

<template>
  <div class="fixed inset-0 z-[100] flex items-center justify-center bg-overlay backdrop-blur-[6px]">
    <div class="w-full max-w-[520px] bg-surface border border-line rounded-modal p-8 shadow-[0_24px_80px_rgba(0,0,0,0.7)]">

      <div class="flex items-center gap-3.5 mb-5">
        <div class="flex items-center justify-center w-[42px] h-[42px] rounded-xl bg-blue/15 border border-blue/30 text-blue-2 shrink-0 mt-0.5">
          <Lock :size="20"/>
        </div>
        <div>
          <h2 class="text-lg font-semibold text-ink m-0">{{ title }}</h2>
          <p v-if="mode === 'unlock'" class="text-base text-ink-4 m-0">이 파일은 암호화되어 있습니다.</p>
          <p v-else-if="mode === 'setup'" class="text-base text-ink-4 m-0">암호화 비밀번호를 입력하세요.</p>
          <p v-else class="text-base text-ink-4 m-0">현재 비밀번호와 새 비밀번호를 입력하세요.</p>
        </div>
      </div>

      <!-- 비밀번호 분실 경고 (setup 모드) -->
      <div v-if="mode === 'setup'"
           class="flex items-center gap-2.5 px-3.5 py-3 rounded-btn bg-amber/[8%] border border-amber/25 text-base text-amber leading-[1.5] mb-[18px]">
        <AlertTriangle :size="16" class="shrink-0 mt-[1px]"/>
        <span>비밀번호를 분실하면 데이터를 <strong><span class="underline">절대로</span></strong>
          복구할 수 없습니다. 반드시 안전한 곳에 보관하세요.</span>
      </div>

      <div class="flex flex-col gap-3.5 mb-[22px]">
        <!-- 현재/기존 비밀번호 -->
        <div class="flex flex-col gap-1.5">
          <label class="text-base font-medium text-ink-3">{{ mode === 'change' ? '현재 비밀번호' : '비밀번호' }}</label>
          <div class="relative">
            <input
                :type="showPassword ? 'text' : 'password'"
                v-model="password"
                :placeholder="passwordPlaceholder"
                class="w-full py-2.5 pr-10 pl-3.5 bg-base border border-line-2 rounded-btn text-ink text-base outline-none transition-colors focus:border-blue-2 box-border placeholder:text-ink-5"
                @keydown.enter="handleSubmit"
                autofocus
            />
            <button type="button"
                    class="absolute right-2.5 top-1/2 -translate-y-1/2 bg-transparent border-none text-ink-5 cursor-pointer p-1 flex items-center transition-colors hover:text-ink-3"
                    @click="showPassword = !showPassword">
              <Eye v-if="!showPassword" :size="16"/>
              <EyeOff v-else :size="16"/>
            </button>
          </div>
        </div>

        <!-- 새 비밀번호 (change 모드) -->
        <div v-if="mode === 'change'" class="flex flex-col gap-1.5">
          <label class="text-base font-medium text-ink-3">새 비밀번호</label>
          <div class="relative">
            <input
                :type="showNewPassword ? 'text' : 'password'"
                v-model="newPassword"
                :placeholder="`새 비밀번호 입력 (최소 ${MIN_PASSWORD_LENGTH}자)`"
                class="w-full py-2.5 pr-10 pl-3.5 bg-base border border-line-2 rounded-btn text-ink text-base outline-none transition-colors focus:border-blue-2 box-border placeholder:text-ink-5"
                @keydown.enter="handleSubmit"
            />
            <button type="button"
                    class="absolute right-2.5 top-1/2 -translate-y-1/2 bg-transparent border-none text-ink-5 cursor-pointer p-1 flex items-center transition-colors hover:text-ink-3"
                    @click="showNewPassword = !showNewPassword">
              <Eye v-if="!showNewPassword" :size="16"/>
              <EyeOff v-else :size="16"/>
            </button>
          </div>
        </div>

        <!-- 비밀번호 확인 (setup/change 모드) -->
        <div v-if="mode === 'setup' || mode === 'change'" class="flex flex-col gap-1.5">
          <label class="text-base font-medium text-ink-3">{{ mode === 'setup' ? '비밀번호 확인' : '새 비밀번호 확인' }}</label>
          <div class="relative">
            <input
                :type="showConfirmPassword ? 'text' : 'password'"
                v-model="confirmPassword"
                :placeholder="mode === 'setup' ? '비밀번호 재입력' : '새 비밀번호 재입력'"
                class="w-full py-2.5 pr-10 pl-3.5 bg-base border border-line-2 rounded-btn text-ink text-base outline-none transition-colors focus:border-blue-2 box-border placeholder:text-ink-5"
                @keydown.enter="handleSubmit"
            />
            <button type="button"
                    class="absolute right-2.5 top-1/2 -translate-y-1/2 bg-transparent border-none text-ink-5 cursor-pointer p-1 flex items-center transition-colors hover:text-ink-3"
                    @click="showConfirmPassword = !showConfirmPassword">
              <Eye v-if="!showConfirmPassword" :size="16"/>
              <EyeOff v-else :size="16"/>
            </button>
          </div>
        </div>

        <!-- 오류 메시지 -->
        <transition
            enter-from-class="opacity-0 translate-y-1"
            enter-active-class="transition-all duration-200"
            leave-to-class="opacity-0 translate-y-1"
            leave-active-class="transition-all duration-200"
        >
          <div v-if="localError"
               class="flex items-center gap-2 px-3.5 py-3 rounded-btn bg-red/10 border border-red/30 text-base text-red/80 leading-[1.5]">
            <AlertTriangle :size="15" class="shrink-0 mt-[1px]"/>
            {{ localError }}
          </div>
        </transition>
      </div>

      <div class="flex gap-2.5 justify-end">
        <button class="btn-secondary" @click="handleCancel" :disabled="loading">
          {{ mode === 'unlock' ? '뒤로 가기' : '취소' }}
        </button>
        <button class="btn-primary" @click="handleSubmit" :disabled="loading">
          <span v-if="loading" class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"/>
          <span v-else>{{ submitLabel }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
