import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

export const useActivityStore = defineStore('activity', () => {
    const activities = ref([])  // ActivityDetail[]
    const loading = ref(false)
    const error = ref('')

    const activityRecords = ref([])  // ActivityRecordItem[]
    const recordsLoading = ref(false)
    const recordsError = ref('')
    let fetchRecordsGen = 0

    async function fetchActivities() {
        loading.value = true
        error.value = ''
        try {
            activities.value = await invoke('get_activities')
        } catch (e) {
            error.value = String(e)
            // record.js와 같은 계약: error에 담고 **다시 던진다.**
            // 삼키면 호출부의 try/catch가 죽은 코드가 되고, 읽기가 실패해도
            // "데이터가 없음"과 구분되지 않는 빈 목록이 그대로 보인다.
            throw e
        } finally {
            loading.value = false
        }
    }

    async function deleteActivity(id) {
        await invoke('delete_activity', {id})
        await fetchActivities()
    }

    // 아래 쓰기 함수들은 `loading`·`error`를 건드리지 않고 **던지기만 한다.**
    // 둘은 활동 목록 화면의 상태라, 쓰기 실패를 거기 담으면 모달 뒤의 목록 전체가
    // 오류 문구로 바뀌고 모달을 닫아도 다시 불러올 때까지 목록이 사라졌다.
    // 실패는 호출한 쪽(모달·가져오기 화면)이 표시한다. 뒤이은 fetchActivities의
    // 실패는 목록 읽기 실패이므로 거기서 error에 담긴다.
    async function saveActivity({mode, id, name, areaIds}) {
        let activityId
        if (mode === 'add') {
            activityId = await invoke('create_activity', {name})
        } else {
            activityId = id
            await invoke('update_activity', {id: activityId, name})
        }
        await invoke('set_activity_areas', {activityId, areaIds})
        await fetchActivities()
    }

    async function createActivitiesBatch(names) {
        const nameToId = await invoke('create_activities_batch', {names})
        await fetchActivities()
        return nameToId
    }

    async function createActivity(name) {
        const id = await invoke('create_activity', {name})
        await fetchActivities()
        return id
    }

    async function fetchActivityRecords(activityId) {
        const gen = ++fetchRecordsGen
        recordsLoading.value = true
        recordsError.value = ''
        activityRecords.value = []
        try {
            const result = await invoke('get_activity_records', {activityId})
            if (gen === fetchRecordsGen) activityRecords.value = result
        } catch (e) {
            if (gen === fetchRecordsGen) recordsError.value = String(e)
        } finally {
            if (gen === fetchRecordsGen) recordsLoading.value = false
        }
    }

    return {
        activities, loading, error,
        fetchActivities, deleteActivity, saveActivity, createActivity, createActivitiesBatch,
        activityRecords, recordsLoading, recordsError, fetchActivityRecords,
    }
})
