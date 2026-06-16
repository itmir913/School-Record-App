<script setup>
import {ref, computed, watch} from 'vue'
import {Lock, Eye, EyeOff, AlertTriangle} from 'lucide-vue-next'

const props = defineProps({
  // 'unlock' | 'setup' | 'change'
  mode: {type: String, required: true},
  error: {type: String, default: ''},
  loading: {type: Boolean, default: false},
})

const emit = defineEmits(['submit', 'cancel'])

const password = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const showNewPassword = ref(false)
const localError = ref('')

const isSetupMode = computed(() => props.mode === 'setup')
const isChangeMode = computed(() => props.mode === 'change')

const title = computed(() => {
  if (props.mode === 'unlock') return '비밀번호 확인'
  if (props.mode === 'setup') return '암호화 비밀번호 설정'
  return '비밀번호 변경'
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
    if (newPassword.value !== confirmPassword.value) {
      localError.value = '새 비밀번호와 확인 비밀번호가 일치하지 않습니다.'
      return false
    }
  }

  return true
}

function handleSubmit() {
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
                :placeholder="mode === 'change' ? '현재 비밀번호' : '비밀번호 입력'"
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
                placeholder="새 비밀번호 입력"
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
                type="password"
                v-model="confirmPassword"
                :placeholder="mode === 'setup' ? '비밀번호 재입력' : '새 비밀번호 재입력'"
                class="w-full py-2.5 pr-10 pl-3.5 bg-base border border-line-2 rounded-btn text-ink text-base outline-none transition-colors focus:border-blue-2 box-border placeholder:text-ink-5"
                @keydown.enter="handleSubmit"
            />
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
        <button
            class="py-2.5 px-5 rounded-btn bg-transparent border border-line-2 text-ink-3 text-base cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed enabled:hover:bg-line enabled:hover:text-ink"
            @click="handleCancel"
            :disabled="loading"
        >
          {{ mode === 'unlock' ? '뒤로 가기' : '취소' }}
        </button>
        <button
            class="py-2.5 px-6 rounded-btn bg-blue border-none text-white text-base font-medium cursor-pointer flex items-center gap-2 transition-colors min-w-[100px] justify-center disabled:opacity-50 disabled:cursor-not-allowed enabled:hover:bg-blue-2"
            @click="handleSubmit"
            :disabled="loading"
        >
          <span v-if="loading" class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"/>
          <span v-else>{{ submitLabel }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
