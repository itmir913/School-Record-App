import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

export const useStudentStore = defineStore('student', () => {
    const students = ref([])
    const loading = ref(false)
    const error = ref('')

    async function fetchStudents() {
        loading.value = true
        error.value = ''
        try {
            students.value = await invoke('get_students')
        } catch (e) {
            error.value = String(e)
        } finally {
            loading.value = false
        }
    }

    async function createStudent(grade, classNum, number, name) {
        await invoke('create_student', {grade, classNum, number, name})
        await fetchStudents()
    }

    async function updateStudent(id, grade, classNum, number, name) {
        await invoke('update_student', {id, grade, classNum, number, name})
        await fetchStudents()
    }

    async function deleteStudent(id) {
        await invoke('delete_student', {id})
        await fetchStudents()
    }

    return {students, loading, error, fetchStudents, createStudent, updateStudent, deleteStudent}
})
