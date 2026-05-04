<script setup>
import {computed} from 'vue'
import {revealItemInDir} from '@tauri-apps/plugin-opener'
import {
  BookOpen,
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

const navGroups = [
  {
    items: [
      {id: 'overview', label: '개요', icon: LayoutDashboard},
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
  {
    items: [
      {id: 'settings', label: '설정(Settings)', icon: Settings},
    ],
  },
]
</script>

<template>
  <aside :class="['sidebar', collapsed ? 'sidebar--collapsed' : '']">

    <!-- 상단: 타이틀 + 토글 -->
    <div class="sidebar-header">
      <div v-if="!collapsed" class="sidebar-title">
        <span class="title-badge">생기부</span>
        <span class="title-text">학교생활기록부</span>
      </div>
      <button class="toggle-btn" @click="toggle" :title="collapsed ? '사이드바 열기' : '사이드바 접기'">
        <ChevronLeft v-if="!collapsed" :size="18"/>
        <ChevronRight v-else :size="18"/>
      </button>
    </div>

    <!-- 네비게이션 -->
    <nav class="sidebar-nav">
      <template v-for="(group, gi) in navGroups" :key="gi">
        <div class="nav-divider" v-if="gi > 0"/>
        <button
            v-for="item in group.items"
            :key="item.id"
            :class="['nav-item', activeSection === item.id ? 'nav-item--active' : '']"
            @click="select(item.id)"
            :title="collapsed ? item.label : ''"
        >
          <component :is="item.icon" :size="20" class="nav-icon"/>
          <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
        </button>
      </template>
    </nav>

    <!-- 하단: 파일 정보 + 저장 -->
    <div class="sidebar-footer">
      <div class="footer-divider"/>

      <!-- 파일 경로 버튼 -->
      <button
          v-if="fileName"
          class="file-btn"
          @click="openFolder"
          :title="filePath"
      >
        <FolderOpen :size="20" class="file-icon"/>
        <span v-if="!collapsed" class="file-name">{{ fileName }}</span>
      </button>

      <!-- 스냅샷 버튼 -->
      <button
          v-if="fileName"
          class="autosave-indicator"
          :class="{ 'autosave-indicator--icon': collapsed }"
          @click="$emit('openSnapshot')"
          title="스냅샷 관리"
      >
        <GitBranch :size="20" class="autosave-icon"/>
        <span v-if="!collapsed" class="autosave-text">스냅샷(Snapshot)</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: 240px;
  min-height: 100vh;
  background-color: #0b0f1c;
  border-right: 1px solid #1a2035;
  transition: width 0.25s ease;
  overflow: hidden;
  flex-shrink: 0;
}

.sidebar--collapsed {
  width: 60px;
}

/* 헤더 */
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 12px 12px;
  border-bottom: 1px solid #1a2035;
  min-height: 60px;
  gap: 8px;
}

.sidebar-title {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  white-space: nowrap;
}

.title-badge {
  font-size: 11px;
  font-weight: 700;
  color: #fbbf24;
  background-color: rgba(251, 191, 36, 0.12);
  border: 1px solid rgba(251, 191, 36, 0.25);
  border-radius: 5px;
  padding: 1px 6px;
  flex-shrink: 0;
}

.title-text {
  font-size: 15px;
  font-weight: 600;
  color: #c8d8f0;
  white-space: nowrap;
}

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 8px;
  background: none;
  border: none;
  color: var(--clr-text-hint);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
  margin-left: auto;
}

.toggle-btn:hover {
  background-color: #1a2035;
  color: #7ba3d4;
}

/* 네비게이션 */
.sidebar-nav {
  flex: 1;
  padding: 10px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
}

.nav-divider {
  height: 1px;
  background-color: #263246;
  margin: 6px 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 10px;
  border-radius: 10px;
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 15px;
  font-weight: 500;
  text-align: left;
  white-space: nowrap;
  transition: background-color 0.15s, color 0.15s;
}

.nav-item:hover {
  background-color: #1e293b;
  color: #e2e8f0;
}

.nav-item--active {
  background-color: rgba(59, 91, 219, 0.2);
  color: #93c5fd;
}

.nav-item--active:hover {
  background-color: rgba(59, 91, 219, 0.3);
  color: #bfdbfe;
}

.nav-icon {
  flex-shrink: 0;
}

.sidebar--collapsed .nav-item {
  justify-content: center;
  padding: 9px;
}

/* 하단 */
.sidebar-footer {
  padding: 8px;
}

.footer-divider {
  height: 1px;
  background-color: #1a2035;
  margin-bottom: 8px;
}

.file-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 10px;
  background: none;
  border: none;
  color: #829ab1;
  cursor: pointer;
  font-size: 14px;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  transition: background-color 0.15s, color 0.15s;
}

.file-btn:hover {
  background-color: #1e293b;
  color: #c0d6f0;
}

.file-icon {
  flex-shrink: 0;
}

.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 15px;
}

.autosave-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 10px;
  background: none;
  border: none;
  color: #829ab1;
  cursor: pointer;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  transition: background-color 0.15s, color 0.15s;
}

.autosave-indicator:hover {
  background-color: #1e293b;
  color: #c0d6f0;
}

.autosave-indicator--icon {
  justify-content: center;
  padding: 8px;
}

.autosave-icon {
  flex-shrink: 0;
  color: #8aaaf8;
  opacity: 0.6;
}

.autosave-text {
  font-size: 15px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar--collapsed .file-btn {
  justify-content: center;
  padding: 8px;
}
</style>
