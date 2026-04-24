import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { DEFAULT_REPLACE_RULES } from '../data/defaultReplaceRules'

export const useReplaceRuleStore = defineStore('replaceRule', () => {
  const rules = ref([])
  const loading = ref(false)
  const error = ref('')

  async function fetchRules() {
    loading.value = true
    error.value = ''
    try {
      const fetched = await invoke('get_replace_rules')
      if (fetched.length === 0) {
        await invoke('seed_default_replace_rules', { rules: DEFAULT_REPLACE_RULES })
        rules.value = await invoke('get_replace_rules')
      } else {
        rules.value = fetched
      }
    } catch (e) {
      error.value = e?.toString() ?? '규칙 목록을 불러오지 못했습니다.'
    } finally {
      loading.value = false
    }
  }

  async function createRule(oldText, newText, priority) {
    await invoke('create_replace_rule', { oldText, newText, priority })
    await fetchRules()
  }

  async function updateRule(id, oldText, newText, enabled, priority) {
    await invoke('update_replace_rule', { id, oldText, newText, enabled, priority })
    await fetchRules()
  }

  async function deleteRule(id) {
    await invoke('delete_replace_rule', { id })
    await fetchRules()
  }

  async function previewReplace(scopeType, areaId) {
    return await invoke('preview_replace', { scopeType, areaId: areaId ?? null })
  }

  async function applyReplace(scopeType, areaId) {
    return await invoke('apply_replace', { scopeType, areaId: areaId ?? null })
  }

  return { rules, loading, error, fetchRules, createRule, updateRule, deleteRule, previewReplace, applyReplace }
})
