<script setup>
import {ref} from 'vue'
import {AlertTriangle, KeyRound, Moon, Shield, ShieldOff, Sun} from 'lucide-vue-next'
import {useConfigStore} from '../stores/configStore'
import PasswordModal from '../components/PasswordModal.vue'

const config = useConfigStore()

const showPasswordModal = ref(false)
const passwordModalMode = ref('setup')
const passwordError = ref('')
const passwordLoading = ref(false)
const statusEncryptMessage = ref('')
const confirmEncryptDisable = ref(false)
const disableEncryptLoading = ref(false)

function openSetup() {
  passwordModalMode.value = 'setup'
  passwordError.value = ''
  statusEncryptMessage.value = ''
  showPasswordModal.value = true
}

function openChange() {
  passwordModalMode.value = 'change'
  passwordError.value = ''
  statusEncryptMessage.value = ''
  showPasswordModal.value = true
}

async function handleDisable() {
  if (!confirmEncryptDisable.value) {
    confirmEncryptDisable.value = true
    return
  }
  confirmEncryptDisable.value = false
  disableEncryptLoading.value = true
  statusEncryptMessage.value = ''
  try {
    await config.disableEncryption()
    statusEncryptMessage.value = '암호화가 비활성화되었습니다.'
  } catch (e) {
    statusEncryptMessage.value = '오류: ' + String(e)
  } finally {
    disableEncryptLoading.value = false
  }
}

async function handlePasswordSubmit(payload) {
  passwordError.value = ''
  passwordLoading.value = true
  try {
    if (passwordModalMode.value === 'setup') {
      await config.enableEncryption(payload.password)
      statusEncryptMessage.value = '암호화가 활성화되었습니다.'
    } else {
      await config.changeEncryptionPassword(payload.oldPassword, payload.newPassword)
      statusEncryptMessage.value = '비밀번호가 변경되었습니다.'
    }
    showPasswordModal.value = false
  } catch (e) {
    passwordError.value = String(e)
  } finally {
    passwordLoading.value = false
  }
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden box-border">

    <!-- 섹션 헤더 -->
    <div class="flex items-start justify-between px-10 py-9 border-b border-line flex-shrink-0 gap-4">
      <div>
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">설정(Settings)</h2>
        <p class="text-base text-ink-3 m-0">프로그램의 기본 설정을 관리합니다.</p>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-10 py-8 pb-12">

      <!-- 테마 설정 카드 -->
      <div class="bg-surface border border-line rounded-2xl p-6 mb-5">
        <div class="flex items-center gap-4 mb-5">
          <div class="flex items-center justify-center w-11 h-11 rounded-xl flex-shrink-0 bg-blue/[0.12] border border-blue/30 text-blue">
            <Sun v-if="config.theme === 'light'" :size="20"/>
            <Moon v-else :size="20"/>
          </div>
          <div>
            <h3 class="text-lg font-semibold text-ink m-0">테마</h3>
            <p class="text-base text-ink-4 m-0">화면 색상 모드를 선택합니다.</p>
          </div>
        </div>

        <div class="flex gap-2.5">
          <button
              :class="[
                'flex items-center gap-2 px-[18px] py-[9px] rounded-btn text-base font-medium cursor-pointer transition-[background-color,transform] border active:scale-[0.97]',
                config.theme === 'dark'
                  ? 'bg-raised border-line-2 text-ink'
                  : 'bg-transparent border-line text-ink-4 hover:bg-raised/60'
              ]"
              @click="config.setTheme('dark')"
          >
            <Moon :size="16"/>
            다크
          </button>
          <button
              :class="[
                'flex items-center gap-2 px-[18px] py-[9px] rounded-btn text-base font-medium cursor-pointer transition-[background-color,transform] border active:scale-[0.97]',
                config.theme === 'light'
                  ? 'bg-raised border-line-2 text-ink'
                  : 'bg-transparent border-line text-ink-4 hover:bg-raised/60'
              ]"
              @click="config.setTheme('light')"
          >
            <Sun :size="16"/>
            라이트
          </button>
        </div>
      </div>

      <!-- 암호화 설정 카드 -->
      <div class="bg-surface border border-line rounded-2xl p-6">

        <!-- 카드 헤더 -->
        <div class="flex items-center gap-4 mb-4">
          <div
              :class="[
                'flex items-center justify-center w-11 h-11 rounded-xl flex-shrink-0',
                config.encryptionEnabled
                  ? 'bg-green/[0.12] border border-green/30 text-green'
                  : 'bg-[rgba(100,116,139,0.12)] border border-[rgba(100,116,139,0.3)] text-[#64748b]'
              ]"
          >
            <Shield v-if="config.encryptionEnabled" :size="20"/>
            <ShieldOff v-else :size="20"/>
          </div>
          <div>
            <h3 class="text-lg font-semibold text-ink m-0">데이터 암호화</h3>
            <p class="text-base text-ink-4 m-0">학생 개인정보와 학교생활기록부 내용을 암호화하여 보호합니다.</p>
          </div>
          <div
              :class="[
                'ml-auto px-3 py-1 rounded-[20px] text-sm font-semibold flex-shrink-0',
                config.encryptionEnabled
                  ? 'bg-green/[0.12] border border-green/30 text-green'
                  : 'bg-[rgba(100,116,139,0.10)] border border-[rgba(100,116,139,0.25)] text-[#64748b]'
              ]"
          >
            {{ config.encryptionEnabled ? '활성화됨' : '비활성화됨' }}
          </div>
        </div>

        <!-- 경고 문구 -->
        <div class="flex items-center gap-2 px-3.5 py-2.5 rounded-btn bg-amber/[0.07] border border-amber/20 text-base text-amber leading-relaxed mb-[18px]">
          <AlertTriangle :size="16" class="flex-shrink-0 mt-px"/>
          <span>
            비밀번호 분실 시
            <strong><span style="text-decoration: underline;">어떠한 방법으로도</span></strong>
            데이터를 복구할 수 없습니다.
            비밀번호를 잊지 않도록 반드시 주의해 주세요.
          </span>
        </div>

        <!-- 버튼 -->
        <div class="flex gap-2.5 flex-wrap">
          <button
              v-if="!config.encryptionEnabled"
              class="flex items-center gap-2 px-[18px] py-[9px] rounded-btn text-base font-medium cursor-pointer transition-[background-color,transform] border bg-green/15 border-green/35 text-green enabled:hover:bg-green/25 active:scale-[0.97] disabled:opacity-40 disabled:cursor-not-allowed"
              @click="openSetup"
          >
            <Shield :size="16"/>
            암호화 활성화
          </button>
          <template v-else>
            <button
                class="flex items-center gap-2 px-[18px] py-[9px] rounded-btn text-base font-medium cursor-pointer transition-[background-color,transform] border bg-blue/15 border-blue/35 text-[#6ea8fe] enabled:hover:bg-blue/25 active:scale-[0.97] disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none"
                :disabled="disableEncryptLoading"
                @click="openChange"
            >
              <KeyRound :size="16"/>
              비밀번호 변경
            </button>
            <button
                v-if="!confirmEncryptDisable"
                class="flex items-center gap-2 px-[18px] py-[9px] rounded-btn text-base font-medium cursor-pointer transition-[background-color,transform] border bg-red/10 border-red/30 text-[#fca5a5] enabled:hover:bg-red/[0.18] active:scale-[0.97] disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none"
                :disabled="disableEncryptLoading"
                @click="handleDisable"
            >
              <ShieldOff :size="16"/>
              암호화 비활성화
            </button>
            <div v-else class="flex flex-row items-center gap-2.5">
              <div class="flex items-center gap-2 bg-red/[0.07] border border-red/25 rounded-btn px-[18px] py-[9px]">
                <AlertTriangle :size="14" class="text-red flex-shrink-0"/>
                <span class="text-base font-semibold text-red">암호화를 정말 비활성화하시겠습니까?</span>
              </div>
              <div class="flex items-center gap-2">
                <button
                    class="px-[18px] py-[9px] rounded-lg border border-[rgba(100,116,139,0.4)] bg-transparent text-[#94a3b8] text-base cursor-pointer box-border enabled:hover:bg-[rgba(100,116,139,0.12)] disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none"
                    :disabled="disableEncryptLoading"
                    @click="confirmEncryptDisable = false"
                >
                  취소
                </button>
                <button
                    class="px-[18px] py-[9px] rounded-lg border border-transparent bg-[#ef4444] text-white text-base font-semibold cursor-pointer transition-colors box-border enabled:hover:bg-[#dc2626] disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none"
                    :disabled="disableEncryptLoading"
                    @click="handleDisable"
                >
                  {{ disableEncryptLoading ? '처리 중…' : '비활성화' }}
                </button>
              </div>
            </div>
          </template>
        </div>

        <!-- 상태 메시지 -->
        <transition
            enter-from-class="opacity-0"
            leave-to-class="opacity-0"
            enter-active-class="transition-opacity duration-300"
            leave-active-class="transition-opacity duration-300"
        >
          <p
              v-if="statusEncryptMessage"
              :class="['mt-3.5 text-base font-medium m-0', statusEncryptMessage.startsWith('오류') ? 'text-[#fca5a5]' : 'text-green']"
          >
            {{ statusEncryptMessage }}
          </p>
        </transition>
      </div>
    </div>

    <!-- 비밀번호 모달 -->
    <PasswordModal
        v-if="showPasswordModal"
        :mode="passwordModalMode"
        :error="passwordError"
        :loading="passwordLoading"
        @submit="handlePasswordSubmit"
        @cancel="showPasswordModal = false"
    />
  </div>
</template>
