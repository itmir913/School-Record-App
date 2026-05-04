import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

const RECORD_CELL_SIZE_KEY = 'record_section_cell_text_size'
const DEFAULT_CELL_SIZE = 14

export const useConfigStore = defineStore('config', () => {
    const recordCellFontSize = ref(DEFAULT_CELL_SIZE)
    const encryptionEnabled = ref(false)
    const encryptionUnlocked = ref(false)

    async function loadAll() {
        await loadPreferences()
        await refreshEncryptionStatus()
    }

    async function loadPreferences() {
        const val = await invoke('get_config', { key: RECORD_CELL_SIZE_KEY })
        if (val !== null && val !== undefined) {
            const parsed = parseInt(val, 10)
            if (!isNaN(parsed)) recordCellFontSize.value = parsed
        }
    }

    async function refreshEncryptionStatus() {
        const status = await invoke('get_encryption_status')
        encryptionEnabled.value = status.enabled
        encryptionUnlocked.value = status.unlocked
    }

    async function setRecordCellFontSize(size) {
        recordCellFontSize.value = size
        await invoke('set_config', { key: RECORD_CELL_SIZE_KEY, value: String(size) })
    }

    async function unlockEncryption(password) {
        await invoke('unlock_encryption', { password })
        await refreshEncryptionStatus()
    }

    async function enableEncryption(password) {
        await invoke('enable_encryption', { password })
        await refreshEncryptionStatus()
    }

    async function disableEncryption() {
        await invoke('disable_encryption')
        await refreshEncryptionStatus()
    }

    async function changeEncryptionPassword(oldPassword, newPassword) {
        await invoke('change_encryption_password', { oldPassword, newPassword })
        await refreshEncryptionStatus()
    }

    return {
        recordCellFontSize,
        encryptionEnabled,
        encryptionUnlocked,
        loadAll,
        loadPreferences,
        refreshEncryptionStatus,
        setRecordCellFontSize,
        unlockEncryption,
        enableEncryption,
        disableEncryption,
        changeEncryptionPassword,
    }
})
