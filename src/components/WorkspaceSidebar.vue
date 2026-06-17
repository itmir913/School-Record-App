<script setup>
import {computed} from 'vue'
import {revealItemInDir} from '@tauri-apps/plugin-opener'
import {
  BookOpen,
  BookMarked,
  ChevronLeft,
  ChevronRight,
  ClipboardList,
  Download,
  FolderOpen,
  GitBranch,
  Layers,
  LayoutDashboard,
  PenLine,
  Replace,
  ScanSearch,
  Settings,
  Upload,
  Users,
} from 'lucide-vue-next'
import {WebviewWindow} from '@tauri-apps/api/webviewWindow'

const props = defineProps({
  collapsed: Boolean,
  activeSection: String,
  filePath: String,
})

const emit = defineEmits(['update:collapsed', 'select', 'openSnapshot'])

const fileName = computed(() => {
  if (!props.filePath) return ''
  return props.filePath.replace(/\\/g, '/').split('/').pop()
})

function toggle() {
  emit('update:collapsed', !props.collapsed)
}

function select(section) {
  emit('select', section)
}

async function openFolder() {
  if (props.filePath) {
    await revealItemInDir(props.filePath)
  }
}

async function openManual() {
  const existing = await WebviewWindow.getByLabel('manual')
  if (existing) {
    await existing.setFocus()
    return
  }
  new WebviewWindow('manual', {
    url: '/manual.html',
    title: '사용 매뉴얼 — All-in-One 학교생활기록부 에디터',
    width: 960,
    height: 720,
    resizable: true,
    center: true,
  })
}

const navGroups = [
  {
    items: [
      {id: 'overview', label: '개요', icon: LayoutDashboard},
      {id: 'manual', label: '매뉴얼', icon: BookMarked},
    ],
  },
  {
    items: [
      {id: 'student', label: '학생(Students)', icon: Users},
      {id: 'area', label: '영역(Area)', icon: Layers},
      {id: 'activity', label: '활동(Activity)', icon: BookOpen},
    ],
  },
  {
    items: [
      {id: 'record', label: '생기부 작성(Write)', icon: PenLine},
    ],
  },
  {
    items: [
      {id: 'replace', label: '텍스트 치환(Replace)', icon: Replace},
      {id: 'inspect', label: '유의어 점검(Inspect)', icon: ScanSearch},
    ],
  },
  {
    items: [
      {id: 'import', label: '가져오기(Import)', icon: Download},
      {id: 'export', label: '내보내기(Export)', icon: Upload},
      {id: 'checklist', label: '체크리스트(Checklist)', icon: ClipboardList},
    ],
  },
]
</script>

<template>
  <aside
      class="flex flex-col min-h-screen bg-sidebar border-r border-line-2 text-ink-3 transition-[width] duration-250 ease-linear overflow-hidden shrink-0"
      :class="collapsed ? 'w-15' : 'w-60'"
  >

    <!-- 상단: 타이틀 + 토글 -->
    <div class="flex items-center justify-between px-3 border-b border-line-2 h-15 gap-2">
      <div v-if="!collapsed" class="flex items-center gap-2.5 overflow-hidden whitespace-nowrap">
        <span class="text-base font-bold text-amber bg-amber/12 border border-amber/25 rounded-[5px] py-px px-1.5 shrink-0">에디터</span>
        <span class="text-lg font-semibold text-ink-2 whitespace-nowrap">학교생활기록부</span>
      </div>
      <button
          class="flex items-center justify-center w-8 h-8 shrink-0 rounded-lg bg-transparent border-none text-ink-5 cursor-pointer transition-colors ml-auto hover:bg-line hover:text-ink-3"
          @click="toggle"
          :title="collapsed ? '사이드바 열기' : '사이드바 접기'"
      >
        <ChevronLeft v-if="!collapsed" :size="18"/>
        <ChevronRight v-else :size="18"/>
      </button>
    </div>

    <!-- 네비게이션 -->
    <nav class="flex-1 py-2.5 px-2 flex flex-col gap-0.5 overflow-y-auto">
      <template v-for="(group, gi) in navGroups" :key="gi">
        <div v-if="gi > 0" class="sidebar-divider my-2 mx-1"/>
        <button
            v-for="item in group.items"
            :key="item.id"
            class="flex items-center gap-2.5 w-full rounded-btn bg-transparent border-none cursor-pointer text-base font-medium text-left whitespace-nowrap transition-colors"
            :class="[
              collapsed ? 'justify-center p-2.25' : 'py-2.25 px-2.5',
              activeSection === item.id
                ? 'bg-blue/20 text-blue-2 hover:bg-blue/30 hover:text-ink-2'
                : 'text-ink-3 hover:bg-line hover:text-ink'
            ]"
            @click="item.id === 'manual' ? openManual() : select(item.id)"
            :title="collapsed ? item.label : ''"
        >
          <component :is="item.icon" :size="20" class="shrink-0"/>
          <span v-if="!collapsed">{{ item.label }}</span>
        </button>
      </template>
    </nav>

    <!-- 하단: 파일 정보 + 저장 -->
    <div class="flex flex-col px-2 pb-2 pt-0 gap-1.25">
      <div class="h-px bg-line-2 mb-2"/>

      <!-- 파일 경로 버튼 -->
      <button
          v-if="fileName"
          class="flex items-center gap-2 w-full rounded-btn bg-transparent border-none text-ink-3 font-medium cursor-pointer text-left whitespace-nowrap overflow-hidden transition-colors hover:bg-line hover:text-ink-2"
          :class="collapsed ? 'justify-center p-2' : 'py-2 px-2.5'"
          @click="openFolder"
          :title="filePath"
      >
        <FolderOpen :size="20" class="shrink-0"/>
        <span v-if="!collapsed">{{ fileName }}</span>
      </button>

      <!-- 스냅샷 버튼 -->
      <button
          v-if="fileName"
          class="flex items-center gap-2 w-full rounded-btn bg-transparent border-none text-ink-3 font-medium cursor-pointer text-left whitespace-nowrap overflow-hidden transition-colors hover:bg-line hover:text-ink-2"
          :class="collapsed ? 'justify-center p-2' : 'py-2 px-2.5'"
          @click="$emit('openSnapshot')"
          title="스냅샷 관리"
      >
        <GitBranch :size="20" class="shrink-0 text-blue-2 opacity-60"/>
        <span v-if="!collapsed">스냅샷(Snapshot)</span>
      </button>

      <!-- 설정 버튼 -->
      <button
          class="flex items-center gap-2 w-full rounded-btn bg-transparent border-none font-medium cursor-pointer text-left whitespace-nowrap overflow-hidden transition-colors"
          :class="[
            collapsed ? 'justify-center p-2' : 'py-2 px-2.5',
            activeSection === 'settings'
              ? 'bg-blue/20 text-blue-2 hover:bg-blue/30 hover:text-ink-2'
              : 'text-ink-3 hover:bg-line hover:text-ink-2'
          ]"
          @click="select('settings')"
          title="설정(Settings)"
      >
        <Settings :size="20"
                  class="shrink-0"
                  :class="activeSection === 'settings' ? 'text-blue-2 opacity-100' : 'text-blue-2 opacity-60'"
        />
        <span v-if="!collapsed">설정(Settings)</span>
      </button>
    </div>
  </aside>
</template>
