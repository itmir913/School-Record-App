import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { DEFAULT_SYNONYMS } from '../data/defaultSynonyms'

export const useSynonymStore = defineStore('synonym', () => {
  const groups = ref([])
  const records = ref([])
  const loading = ref(false)
  const error = ref('')

  async function fetchGroups() {
    loading.value = true
    error.value = ''
    try {
      const fetched = await invoke('get_synonym_groups')
      if (fetched.length === 0) {
        await invoke('seed_default_synonyms', {
          groups: Object.entries(DEFAULT_SYNONYMS).map(([name, words]) => ({ name, words })),
        })
        groups.value = await invoke('get_synonym_groups')
      } else {
        groups.value = fetched
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchRecords() {
    records.value = await invoke('get_all_records_for_inspect')
  }

  async function createGroup(name) {
    await invoke('create_synonym_group', { name })
    await fetchGroups()
  }

  async function deleteGroup(id) {
    await invoke('delete_synonym_group', { id })
    await fetchGroups()
  }

  async function addWord(groupId, word) {
    await invoke('add_synonym_word', { groupId, word })
    await fetchGroups()
  }

  async function deleteWord(id) {
    await invoke('delete_synonym_word', { id })
    await fetchGroups()
  }

  return {
    groups,
    records,
    loading,
    error,
    fetchGroups,
    fetchRecords,
    createGroup,
    deleteGroup,
    addWord,
    deleteWord,
  }
})
