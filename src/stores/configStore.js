import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

const RECORD_CELL_SIZE_KEY = 'record_section_cell_text_size'
const DEFAULT_CELL_SIZE = 14

export const useConfigStore = defineStore('config', () => {
    const recordCellFontSize = ref(DEFAULT_CELL_SIZE)

    async function loadAll() {
        const val = await invoke('get_config', { key: RECORD_CELL_SIZE_KEY })
        if (val !== null && val !== undefined) {
            const parsed = parseInt(val, 10)
            if (!isNaN(parsed)) recordCellFontSize.value = parsed
        }
    }

    async function setRecordCellFontSize(size) {
        recordCellFontSize.value = size
        await invoke('set_config', { key: RECORD_CELL_SIZE_KEY, value: String(size) })
    }

    return { recordCellFontSize, loadAll, setRecordCellFontSize }
})
