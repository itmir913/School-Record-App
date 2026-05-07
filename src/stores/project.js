import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useProjectStore = defineStore('project', () => {
  const isOpen = ref(false)
  const filePath = ref('')

  function setProject(path) {
    filePath.value = path
    isOpen.value = true
  }

  function closeProject() {
    filePath.value = ''
    isOpen.value = false
  }

  async function openProject(path) {
    await invoke('open_project', { path })
    setProject(path)
  }

  async function migrateSchema() {
    await invoke('migrate_schema')
  }

  // 반환값: null = 버전 동일(모달 불필요), string = 이전 버전(모달 표시)
  // migrateSchema() 이후 호출해야 함
  async function checkAndUpdateVersion() {
    return await invoke('check_and_update_app_version')  // null | "" | "0.2.x"
  }

  return { isOpen, filePath, setProject, closeProject, openProject, migrateSchema, checkAndUpdateVersion }
})
