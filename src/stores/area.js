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
            // record.js와 같은 계약: error에 담고 **다시 던진다.**
            // 삼키면 호출부의 try/catch가 죽은 코드가 되고, 읽기가 실패해도
            // "데이터가 없음"과 구분되지 않는 빈 목록이 그대로 보인다.
            throw e
        } finally {
            loading.value = false
        }
    }

    // 영역 저장과 활동 연결은 save_area 한 번으로 끝난다. 둘을 따로 부르면 앞 단계만
    // 반영된 채 실패할 수 있어, 백엔드가 한 트랜잭션으로 묶는다. id가 없으면 추가다.
    async function saveArea({mode, id, name, byteLimit, activityIds}) {
        const areaId = await invoke('save_area', {
            id: mode === 'add' ? null : id,
            name,
            byteLimit,
            activityIds,
        })
        await fetchAreas()
        return areaId
    }

    async function deleteArea(id) {
        await invoke('delete_area', {id})
        await fetchAreas()
    }

    async function getAreaStudents(areaId) {
        return await invoke('get_area_students', {areaId})
    }

    async function setAreaStudents(areaId, studentIds) {
        await invoke('set_area_students', {areaId, studentIds})
    }

    return {areas, loading, error, fetchAreas, saveArea, deleteArea, getAreaStudents, setAreaStudents}
})
