import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'

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

  // 반환값: null = 버전 동일(모달 불필요), string = 이전 버전(모달 표시)
  async function openProject(path) {
    await invoke('open_project', { path })
    const version = await getVersion()
    const oldVersion = await invoke('check_and_update_app_version', { currentVersion: version })
    setProject(path)
    return oldVersion  // null | "" | "0.2.x"
  }

  return { isOpen, filePath, setProject, closeProject, openProject }
})
