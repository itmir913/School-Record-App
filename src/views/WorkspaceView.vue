<script setup>
import {computed, onMounted, ref} from 'vue'
import {getCurrentWindow} from '@tauri-apps/api/window'
import {LogicalSize} from '@tauri-apps/api/dpi'
import {useProjectStore} from '../stores/project'
import {useConfigStore} from '../stores/configStore'
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

const project = useProjectStore()
const config = useConfigStore()
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
}

const currentSection = computed(() => sectionMap[activeSection.value])

onMounted(async () => {
  try {
    const win = getCurrentWindow()
    await win.setResizable(true)
    await win.setMinSize(new LogicalSize(900, 600))
    await win.setSize(new LogicalSize(1280, 720))
    await win.center()
  } catch {
    // 창 리사이즈 실패는 비치명적이므로 무시
  }
  try { await config.loadAll() } catch { }
})
</script>

<template>
  <div class="workspace">
    <WorkspaceSidebar
        v-model:collapsed="collapsed"
        :active-section="activeSection"
        :file-path="project.filePath"
        @select="activeSection = $event"
        @openSnapshot="showSnapshotModal = true"
    />
    <main class="workspace-main">
      <component :is="currentSection" :key="sectionKey" @navigate="activeSection = $event"/>
    </main>
    <SnapshotModal
        v-if="showSnapshotModal"
        @close="showSnapshotModal = false"
        @restored="sectionKey++"
    />
  </div>
</template>

<style scoped>
.workspace {
  display: flex;
  height: 100vh;
  background-color: #080b14;
  overflow: hidden;
}

.workspace-main {
  flex: 1;
  overflow-y: auto;
  background-color: #080b14;
}
</style>
