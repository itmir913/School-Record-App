import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { DEFAULT_SYNONYMS, DEFAULT_SYNONYMS_VERSION } from '../data/defaultSynonyms'

const SYNONYM_VERSION_KEY = 'synonym_version'

function defaultSynonymGroups() {
  return Object.entries(DEFAULT_SYNONYMS).map(([name, words]) => ({ name, words }))
}

export const useSynonymStore = defineStore('synonym', () => {
  const groups = ref([])
  const records = ref([])
  const loading = ref(false)
  const error = ref('')
  const needsSynonymUpdate = ref(false)

  async function fetchGroups() {
    loading.value = true
    error.value = ''
    try {
      const fetched = await invoke('get_synonym_groups')
      const stored = await invoke('get_config', { key: SYNONYM_VERSION_KEY })
      // 시드는 "기본 유의어를 넣은 적이 없는 파일"에만 한다. 버전 키가 그 표식이다.
      // 목록이 비었다는 것만으로 판정하면, 사용자가 그룹을 전부 지운 순간
      // 재조회가 기본 그룹을 모두 되살린다.
      if (stored == null && fetched.length === 0) {
        await invoke('apply_default_synonyms', { groups: defaultSynonymGroups() })
        await invoke('set_config', { key: SYNONYM_VERSION_KEY, value: String(DEFAULT_SYNONYMS_VERSION) })
        groups.value = await invoke('get_synonym_groups')
        needsSynonymUpdate.value = false
      } else {
        groups.value = fetched
        needsSynonymUpdate.value = stored !== String(DEFAULT_SYNONYMS_VERSION)
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function applySynonymUpdate() {
    await invoke('apply_default_synonyms', { groups: defaultSynonymGroups() })
    await invoke('set_config', { key: SYNONYM_VERSION_KEY, value: String(DEFAULT_SYNONYMS_VERSION) })
    needsSynonymUpdate.value = false
    await fetchGroups()
  }

  async function fetchRecords(scopeType = 'all', areaIds = []) {
    try {
      records.value = await invoke('get_all_records_for_inspect', {
        scopeType,
        areaIds,
      })
    } catch (e) {
      error.value = String(e)
      throw e
    }
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

  async function addWordsBatch(groupId, words) {
    await invoke('add_synonym_words_batch', { groupId, words })
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
    needsSynonymUpdate,
    fetchGroups,
    applySynonymUpdate,
    fetchRecords,
    createGroup,
    deleteGroup,
    addWord,
    addWordsBatch,
    deleteWord,
  }
})
