import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

export const useAreaStore = defineStore('area', () => {
    const areas = ref([])
    const loading = ref(false)
    const error = ref('')

    async function fetchAreas() {
        loading.value = true
        error.value = ''
        try {
            areas.value = await invoke('get_areas')
        } catch (e) {
            error.value = String(e)
        } finally {
            loading.value = false
        }
    }

    async function createArea(name, byteLimit) {
        const id = await invoke('create_area', {name, byteLimit})
        await fetchAreas()
        return id
    }

    async function updateArea(id, name, byteLimit) {
        await invoke('update_area', {id, name, byteLimit})
        await fetchAreas()
    }

    async function deleteArea(id) {
        await invoke('delete_area', {id})
        await fetchAreas()
    }

    async function setAreaActivities(areaId, activityIds) {
        await invoke('set_area_activities', {areaId, activityIds})
        await fetchAreas()
    }

    return {areas, loading, error, fetchAreas, createArea, updateArea, deleteArea, setAreaActivities}
})
