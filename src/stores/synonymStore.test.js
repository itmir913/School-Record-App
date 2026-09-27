import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { DEFAULT_SYNONYMS_VERSION } from '../data/defaultSynonyms'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args) => invokeMock(...args) }))

const { useSynonymStore } = await import('./synonymStore')

/** 커맨드 이름별로 응답하는 가짜 백엔드. 그룹 목록과 버전 키를 상태로 들고 있다. */
function fakeBackend({ groups = [], version = null } = {}) {
  const db = { groups: [...groups], version }
  invokeMock.mockImplementation(async (cmd, args) => {
    switch (cmd) {
      case 'get_synonym_groups': return [...db.groups]
      case 'get_config': return db.version
      case 'set_config': db.version = args.value; return
      case 'apply_default_synonyms':
        db.groups = args.groups.map((g, i) => ({ id: 100 + i, name: g.name, items: [] }))
        return
      case 'delete_synonym_group': db.groups = db.groups.filter(g => g.id !== args.id); return
      default: throw new Error(`예상하지 못한 커맨드: ${cmd}`)
    }
  })
  return db
}

const seedCalls = () => invokeMock.mock.calls.filter(([cmd]) => cmd === 'apply_default_synonyms')

describe('fetchGroups — 기본 유의어 시드 조건', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
  })

  it('버전 키가 없고 목록이 비었으면 기본 유의어를 넣는다', async () => {
    const db = fakeBackend()
    const store = useSynonymStore()
    await store.fetchGroups()

    expect(seedCalls()).toHaveLength(1)
    expect(db.version).toBe(String(DEFAULT_SYNONYMS_VERSION))
    expect(store.groups.length).toBeGreaterThan(0)
    expect(store.needsSynonymUpdate).toBe(false)
    expect(store.error).toBe('')
  })

  it('마지막 그룹을 지운 뒤 재조회해도 기본 유의어를 되살리지 않는다', async () => {
    fakeBackend({ groups: [{ id: 1, name: 'g', items: [] }], version: String(DEFAULT_SYNONYMS_VERSION) })
    const store = useSynonymStore()
    await store.fetchGroups()
    await store.deleteGroup(1)

    expect(seedCalls()).toHaveLength(0)
    expect(store.groups).toEqual([])
    expect(store.error).toBe('')
  })

  it('버전 키가 없어도 그룹이 이미 있으면 넣지 않고 갱신 표시만 켠다', async () => {
    fakeBackend({ groups: [{ id: 1, name: 'g', items: [] }] })
    const store = useSynonymStore()
    await store.fetchGroups()

    expect(seedCalls()).toHaveLength(0)
    expect(store.groups).toHaveLength(1)
    expect(store.needsSynonymUpdate).toBe(true)
  })
})
