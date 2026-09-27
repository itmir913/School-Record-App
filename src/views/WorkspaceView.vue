<script setup>
import {computed, onMounted, ref} from 'vue'
import {getCurrentWindow} from '@tauri-apps/api/window'
import {LogicalSize} from '@tauri-apps/api/dpi'
import {useProjectStore} from '../stores/project'
import {useConfigStore} from '../stores/configStore'
import {useRecordStore} from '../stores/record'
import WorkspaceSidebar from '../components/WorkspaceSidebar.vue'
import OverviewSection from '../sections/OverviewSection.vue'
import AreaSection from '../sections/AreaSection.vue'
import ActivitySection from '../sections/ActivitySection.vue'
import StudentSection from '../sections/StudentSection.vue'
import RecordSection from '../sections/RecordSection.vue'
import ImportSection from '../sections/ImportSection.vue'
import ExportSection from '../sections/ExportSection.vue'
import ChecklistSection from '../sections/ChecklistSection.vue'
import ReplaceSection from '../sections/ReplaceSection.vue'
import InspectSection from '../sections/InspectSection.vue'
import SnapshotModal from '../components/SnapshotModal.vue'
import SettingsSection from '../sections/SettingsSection.vue'
import UpdateSection from '../sections/UpdateSection.vue'

const project = useProjectStore()
const config = useConfigStore()
const recordStore = useRecordStore()
const collapsed = ref(false)
const activeSection = ref('overview')
const sectionKey = ref(0)
const showSnapshotModal = ref(false)

const sectionMap = {
  overview: OverviewSection,
  area: AreaSection,
  activity: ActivitySection,
  student: StudentSection,
  record: RecordSection,
  import: ImportSection,
  export: ExportSection,
  checklist: ChecklistSection,
  replace: ReplaceSection,
  inspect: InspectSection,
  settings: SettingsSection,
  update: UpdateSection,
}

const currentSection = computed(() => sectionMap[activeSection.value])

onMounted(async () => {
  try {
    const win = getCurrentWindow()
    await win.setResizable(true)
    await win.setMinSize(new LogicalSize(900, 600))
    // 1280은 툴바가 한 줄에 안 들어간다. 실측: 툴바에 필요한 폭이 1074px,
    // 사이드바(240px)를 더하면 1314px이라 1280에서는 34px이 모자라 두 줄이 됐다.
    // 1360으로 두면 46px 여유가 생긴다. 논리 픽셀이므로 배율 100%의 1366×768까지
    // 들어가고, 그 위(125% 등)에서는 1280도 이미 넘쳤다 — 이 값이 만든 제약이 아니다.
    await win.setSize(new LogicalSize(1360, 720))
    await win.center()
  } catch {
    // 창 리사이즈 실패는 비치명적이므로 무시
  }
  try {
    await config.loadAll()
  } catch (e) {
    // 환경설정 로드 실패는 loadAll이 preferencesError에 담고 삼킨다. 여기까지
    // 오는 것은 암호화 상태 조회 실패다. 새로 만든 파일은 홈에서 이 값을 읽지
    // 않으므로, 알리지 않으면 설정 화면이 이전 파일의 암호화 상태를 그대로 보여준다.
    project.openWarnings.push(
        `암호화 상태를 확인하지 못했습니다. 설정 화면의 암호화 표시가 실제와 다를 수 ` +
        `있으니 파일을 다시 열어 주세요. ${e}`
    )
  }
})
</script>

<template>
  <div class="flex h-screen bg-base overflow-hidden">
    <WorkspaceSidebar
        v-model:collapsed="collapsed"
        :active-section="activeSection"
        :file-path="project.filePath"
        @select="activeSection = $event"
        @openSnapshot="showSnapshotModal = true"
    />
    <main class="flex-1 overflow-y-auto bg-surface flex flex-col">
      <!-- 화면을 떠나는 순간 저장에 실패한 경우. 그 배너를 띄우던 컴포넌트는 이미
           사라졌으므로 여기서 대신 보여준다. 사용자가 닫을 때까지 남는다. -->
      <div
          v-if="recordStore.pendingSaveError"
          class="px-6 py-2 border-b border-line-2 shrink-0 bg-red/[0.08] flex items-center gap-3"
      >
        <p class="text-base text-red m-0 flex-1">{{ recordStore.pendingSaveError }}</p>
        <button
            class="bg-transparent border-none p-0 text-base text-red/70 cursor-pointer hover:text-red hover:underline"
            @click="recordStore.pendingSaveError = ''"
        >닫기</button>
      </div>
      <!-- 파일을 여는 도중 나온 경고. 홈 화면에서 띄우면 곧바로 이 화면으로
           넘어가며 사라지므로 여기서 보여준다. 사용자가 닫을 때까지 남는다. -->
      <div
          v-for="(warning, i) in project.openWarnings"
          :key="i"
          class="px-6 py-2 border-b border-line-2 shrink-0 bg-amber/[0.08] flex items-center gap-3"
      >
        <p class="text-base text-amber m-0 flex-1">{{ warning }}</p>
        <button
            class="bg-transparent border-none p-0 text-base text-amber/70 cursor-pointer hover:text-amber hover:underline"
            @click="project.openWarnings.splice(i, 1)"
        >닫기</button>
      </div>
      <component :is="currentSection" :key="sectionKey" @navigate="activeSection = $event"/>
    </main>
    <SnapshotModal
        v-if="showSnapshotModal"
        @close="showSnapshotModal = false"
        @restored="sectionKey++"
    />
  </div>
</template>
