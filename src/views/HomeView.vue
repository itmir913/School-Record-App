<script setup>
import {onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {open, save} from '@tauri-apps/plugin-dialog'
import {openUrl} from '@tauri-apps/plugin-opener'
import {useProjectStore} from '../stores/project'
import {useConfigStore} from '../stores/configStore'
import PasswordModal from '../components/PasswordModal.vue'
import ReleaseNotesModal from '../components/ReleaseNotesModal.vue'
import UpdateModal from '../components/UpdateModal.vue'
import {useUpdateStore} from '../stores/updateStore.js'
import {getNotesToShow} from '../data/releaseNotes'

const router = useRouter()
const project = useProjectStore()
const config = useConfigStore()
const update = useUpdateStore()
const error = ref('')

const showUpdateModal = ref(false)

const showPasswordModal = ref(false)
const passwordError = ref('')
const passwordLoading = ref(false)
const showReleaseNotesModal = ref(false)
const releaseNotesToShow = ref([])

// 열기·만들기가 진행 중인지. 두 번 누르면 두 흐름이 겹쳐, 뒤의 open_project가
// 연결을 바꿔 끼운 뒤 앞 흐름의 백업·버전 기록이 다른 파일에 실행된다.
const busy = ref(false)

// 화면 아래 버전 표기에 쓴다. 네트워크 요청이 아니라 앱 자신의 버전을 읽는 것이다.
onMounted(() => {
  update.loadCurrentVersion()
})

async function handleNew() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try {
    const path = await save({
      title: '새 학생부 파일 위치 선택',
      defaultPath: 'school_record.db',
      filters: [{name: 'SQLite DB', extensions: ['db']}],
    })
    if (!path) return
    await project.newProject(path)
    router.push('/workspace')
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

async function handleOpen() {
  if (busy.value) return
  busy.value = true
  try {
    await openSelectedFile()
  } finally {
    busy.value = false
  }
}

async function openSelectedFile() {
  error.value = ''
  // 대화상자 실패는 열기 실패와 따로 받는다. 아래 catch로 보내면 아무것도 열지
  // 않았는데 closeProject를 부르게 된다.
  let path
  try {
    path = await open({
      title: '기존 학생부 파일 선택',
      filters: [{name: 'SQLite DB', extensions: ['db']}],
      multiple: false,
    })
  } catch (e) {
    error.value = `파일 선택 창을 열지 못했습니다: ${String(e)}`
    return
  }
  if (!path) return
  try {
    await project.openProject(path)
    await config.refreshEncryptionStatus()
    if (config.encryptionEnabled) {
      passwordError.value = ''
      showPasswordModal.value = true
    } else {
      await showReleaseNotesOrNavigate()
    }
  } catch (e) {
    // 열기에 실패했으면 "열린 파일"로 남겨 두지 않는다. isOpen이 참인 채로 남으면
    // 라우터 가드는 통과시키므로, 작업 화면으로 가는 진입점이 하나만 더 생겨도
    // 마이그레이션에 실패한 파일로 화면이 열린다.
    project.closeProject()
    error.value = String(e)
  }
}

async function showReleaseNotesOrNavigate() {
  // 백업은 반드시 마이그레이션 **뒤에** 한다.
  //
  // 앞에 두면 메모 암호화로 넘어가는 그 한 번의 열기에서, 사본에 평문 메모가 담긴
  // 채로 본 DB만 암호화된다. 앱은 백업을 지우지 않으므로 비밀번호 없이 읽히는
  // 사본이 영구히 남는다. 마이그레이션 각 단계는 원자적이라 실패해도 그 단계의
  // 변경이 남지 않으므로, 변환 전 사본이 막아줄 사고가 없다.
  await project.migrateSchema()

  // 변환 뒤에 파일 안의 옛 흔적이 정리됐는지 확인한다. 정리는 실패해도 열기를
  // 막지 않지만(다음에 열 때 다시 시도한다), 실패한 동안 파일에는 암호화되기 전의
  // 메모가 그대로 남아 있다. 알리지 않으면 사용자는 알 방법이 없다.
  try {
    await config.refreshEncryptionStatus()
    if (config.purgePending) {
      project.openWarnings.push(
          '파일 정리가 끝나지 않아, 암호화되기 전의 내용이 파일 안에 남아 있을 수 있습니다. ' +
          '설정 화면에서 "지금 정리"를 눌러 다시 시도하실 수 있습니다.'
      )
    }
  } catch (e) {
    // 상태 조회 실패로 열기를 막지는 않는다. 다만 조용히 넘기면 안 된다 —
    // 이 값이 정리 미완료(평문 잔존)를 알리는 유일한 통로이고, 설정 화면은
    // 이 값을 다시 읽지 않는다. 못 읽었다는 사실 자체를 알린다.
    project.openWarnings.push(
        `파일 정리 상태를 확인하지 못했습니다. 암호화되기 전의 내용이 파일 안에 남아 ` +
        `있을 수 있으니, 설정 화면에서 "지금 정리"를 한 번 눌러 주세요. ${e}`
    )
  }

  // 백업 실패로 열기를 막으면 안 된다.
  //
  // 마이그레이션은 이미 끝났고 파일은 정상이다. 여기서 막으면, 디스크 여유가
  // 빠듯해 사본을 못 뜨는 사용자는 파일이 이미 새 형식이라 **이전 버전으로도**
  // 열 수 없게 되어 갇힌다. 백업은 없어도 파일을 쓰는 데 지장이 없다.
  try {
    await project.backupProject()
  } catch (e) {
    project.openWarnings.push(`이번에는 백업 파일을 만들지 못했습니다. ${e}`)
  }

  // 버전 기록도 열기를 막으면 안 된다. 백업과 같은 이유다 — 이건 릴리즈 노트를
  // 한 번 띄우기 위한 기록일 뿐인데, DB에 쓰기를 하므로 읽기 전용 매체(USB·공유
  // 폴더)나 다른 인스턴스의 잠금에 걸려 실패할 수 있다. 막아 버리면 멀쩡한 파일이
  // 안 열리고, 암호화 파일이라면 그 오류가 비밀번호 모달에 떠 **비밀번호가 틀린
  // 것으로 오해**하게 된다.
  let oldVersion = null
  try {
    oldVersion = await project.checkAndUpdateVersion()
  } catch (e) {
    project.openWarnings.push(
        `이번에는 파일에 버전 기록을 남기지 못했습니다. 다음에 열 때 새 소식이 ` +
        `한 번 더 표시될 수 있습니다. ${e}`
    )
  }
  if (oldVersion !== null) {
    releaseNotesToShow.value = getNotesToShow(oldVersion)
    showReleaseNotesModal.value = true
  } else {
    router.push('/workspace')
  }
}

async function handlePasswordSubmit({password}) {
  if (busy.value) return
  busy.value = true
  passwordError.value = ''
  passwordLoading.value = true
  try {
    await config.unlockEncryption(password)
    // 모달을 먼저 닫으면 이후 단계(백업·마이그레이션·버전 확인)가 실패했을 때
    // passwordError를 렌더할 모달이 이미 사라져 아무것도 표시되지 않는다.
    // 열기 절차를 모두 끝낸 뒤에 닫는다.
    await showReleaseNotesOrNavigate()
    showPasswordModal.value = false
  } catch (e) {
    passwordError.value = String(e)
  } finally {
    passwordLoading.value = false
    busy.value = false
  }
}

function handleReleaseNotesClose() {
  showReleaseNotesModal.value = false
  router.push('/workspace')
}

function handlePasswordCancel() {
  showPasswordModal.value = false
  project.closeProject()
}

</script>

<template>
  <div class="activity-section-wrapper">
    <div class="page">
      <!-- ambient glow -->
      <div class="glow"/>

      <!-- 플로팅 카드 -->
      <div class="card">

        <!-- 로고 -->
        <div class="logo-wrap">
          <div class="logo-icon">
            <img class="logo-glyph" src="/app-icon.svg" alt="">
            <span class="logo-badge"/>
          </div>
          <div class="logo-text">
            <h1>All-in-One 학교생활기록부 에디터</h1>
            <p>학생부를 체계적으로 작성하기 위한 교육용 프로그램</p>
          </div>
        </div>

        <!-- 구분선 -->
        <div class="divider">
          <div class="divider-line"/>
          <span class="divider-dot"/>
          <div class="divider-line"/>
        </div>

        <!-- 버튼 -->
        <div class="actions">
          <button class="btn-primary" @click="handleNew">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 4v16m8-8H4"/>
            </svg>
            새 학생부 만들기
            <svg class="arrow" width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 5l7 7-7 7"/>
            </svg>
          </button>

          <button class="btn-secondary" @click="handleOpen">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path
                  d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
            </svg>
            기존 파일 열기
            <svg class="arrow" width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 5l7 7-7 7"/>
            </svg>
          </button>

          <div class="action-row">
            <button class="btn-update" @click="showUpdateModal = true">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="1 4 1 10 7 10"/>
                <polyline points="23 20 23 14 17 14"/>
                <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
              </svg>
              업데이트 확인
            </button>

            <button class="btn-update" @click="openUrl('https://luminousky.com/teacher-utility-kit/neis-editor/')">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                <polyline points="15 3 21 3 21 9"></polyline>
                <line x1="10" y1="14" x2="21" y2="3"></line>
              </svg>
              프로그램 소개
            </button>
          </div>
        </div>

        <!-- 에러 -->
        <transition name="err">
          <div v-if="error" class="error-box">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                 class="shrink-0 mt-px">
              <path
                  d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
            </svg>
            {{ error }}
          </div>
        </transition>

        <p class="version">
          v{{ update.currentVersion }} |
          <a href="https://github.com/itmir913/School-Record-App/#%EB%9D%BC%EC%9D%B4%EC%84%A0%EC%8A%A4" target="_blank">
            <u>Educational Use Only</u>
          </a>
        </p>
      </div>
    </div>

    <!-- 비밀번호 모달 -->
    <PasswordModal
        v-if="showPasswordModal"
        mode="unlock"
        :error="passwordError"
        :loading="passwordLoading"
        @submit="handlePasswordSubmit"
        @cancel="handlePasswordCancel"
    />

    <!-- 릴리즈 노트 모달 -->
    <ReleaseNotesModal
        v-if="showReleaseNotesModal"
        :notes="releaseNotesToShow"
        @close="handleReleaseNotesClose"
    />

    <!-- 업데이트 확인 모달 (작업 화면 사이드바와 같은 컴포넌트) -->
    <UpdateModal v-if="showUpdateModal" @close="showUpdateModal = false"/>
  </div>
</template>

<style scoped>
/* ── 전체 페이지 ── */
.page {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  overflow: hidden;
  background-color: #080b14;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

/* 배경 glow */
.glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse 60% 50% at 50% 50%, rgba(59, 91, 219, 0.12), transparent);
  pointer-events: none;
}

/* ── 카드 ── */
.card {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 440px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  padding: 40px 36px 32px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.03);
}

/* ── 로고 ── */
.logo-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  margin-bottom: 32px;
}

.logo-icon {
  position: relative;
  width: 68px;
  height: 68px;
  /* 라운드는 app-icon.svg의 22%와 맞춘다 — 여기선 box-shadow 모양만 따라간다 */
  border-radius: 15px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 0 1px rgba(251, 191, 36, 0.2), 0 8px 32px rgba(76, 110, 245, 0.35);
}

.logo-glyph {
  display: block;
  width: 68px;
  height: 68px;
}

.logo-badge {
  position: absolute;
  top: -5px;
  right: -5px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: #fbbf24;
  border: 2px solid #0e1220;
  box-shadow: 0 0 8px rgba(251, 191, 36, 0.5);
}

.logo-text {
  text-align: center;
}

.logo-text h1 {
  font-size: 21px;
  font-weight: 700;
  color: #e2e8f0;
  letter-spacing: -0.02em;
  margin: 0;
}

.logo-text p {
  font-size: 15px;
  color: var(--clr-text-hint);
  margin: 5px 0 0;
}

/* ── 구분선 ── */
.divider {
  display: flex;
  align-items: center;
  margin-bottom: 26px;
}

.divider-line {
  flex: 1;
  height: 1px;
  background-color: #1a2035;
}

.divider-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #fbbf24;
  opacity: 0.5;
  margin: 0 12px;
}

/* ── 버튼 ── */
.actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.btn-primary,
.btn-secondary {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 15px 20px;
  border-radius: 14px;
  font-size: 17px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background-color 0.15s, transform 0.1s;
  text-align: left;
}

.btn-primary:active,
.btn-secondary:active {
  transform: scale(0.98);
}

.btn-primary {
  background-color: #3b5bdb;
  color: #ffffff;
}

.btn-primary:hover {
  background-color: #4c6ef5;
}

.btn-secondary {
  background-color: #131c30;
  border-color: #2e3f60;
  color: #7ba3d4;
}

.btn-secondary:hover {
  background-color: #1a2640;
  border-color: #4a6090;
}

.arrow {
  margin-left: auto;
  opacity: 0;
  transition: opacity 0.15s, transform 0.15s;
}

.btn-primary:hover .arrow,
.btn-secondary:hover .arrow {
  opacity: 1;
  transform: translateX(2px);
}

.action-row {
  display: flex;
  gap: 12px;
  width: 100%;
}

.btn-update {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  flex: 1;
  padding: 10px;
  background: none;
  border: 1px solid #2e3f60;
  border-radius: 10px;
  font-size: 15px;
  color: var(--clr-text-hint);
  cursor: pointer;
  margin-top: 2px;
  transition: color 0.15s, border-color 0.15s, background-color 0.15s;
}

.btn-update:hover {
  color: #a8c4e8;
  border-color: #4a6090;
  background-color: #0d1525;
}

/* ── 에러 ── */
.error-box {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-top: 18px;
  padding: 14px 18px;
  border-radius: 12px;
  background-color: #2a1020;
  border: 1px solid #4a1a28;
  font-size: 15px;
  color: #fca5a5;
  line-height: 1.5;
}

.err-enter-from, .err-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.err-enter-active, .err-leave-active {
  transition: all 0.2s;
}

/* ── 버전 ── */
.version {
  margin-top: 24px;
  text-align: center;
  font-size: 13px;
  color: var(--clr-text-hint);
}

</style>
