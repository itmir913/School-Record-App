import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

export const useActivityStore = defineStore('activity', () => {
    const activities = ref([])  // ActivityDetail[]
    const loading = ref(false)
    const error = ref('')

    async function fetchActivities() {
        loading.value = true
        error.value = ''
        try {
            activities.value = await invoke('get_activities')
        } catch (e) {
            error.value = String(e)
        } finally {
            loading.value = false
        }
    }

    async function createActivity(name) {
        await invoke('create_activity', {name})
        await fetchActivities()
    }

    async function updateActivity(id, name) {
        await invoke('update_activity', {id, name})
        await fetchActivities()
    }

    async function deleteActivity(id) {
        await invoke('delete_activity', {id})
        await fetchActivities()
    }

    async function saveActivity({mode, id, name, areaIds}) {
        loading.value = true
        error.value = ''
        try {
            let activityId
            if (mode === 'add') {
                activityId = await invoke('create_activity', {name})
            } else {
                activityId = id
                await invoke('update_activity', {id: activityId, name})
            }
            await invoke('set_activity_areas', {activityId, areaIds})
            await fetchActivities()
        } catch (e) {
            error.value = String(e)
            throw e
        } finally {
            loading.value = false
        }
    }

    return {activities, loading, error, fetchActivities, createActivity, updateActivity, deleteActivity, saveActivity}
})
