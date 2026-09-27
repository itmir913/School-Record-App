import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { DEFAULT_REPLACE_RULES_VERSION } from '../data/defaultReplaceRules'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args) => invokeMock(...args) }))

const { useReplaceRuleStore } = await import('./replaceRule')

/** 커맨드 이름별로 응답하는 가짜 백엔드. 규칙 목록과 버전 키를 상태로 들고 있다. */
function fakeBackend({ rules = [], version = null } = {}) {
  const db = { rules: [...rules], version }
  invokeMock.mockImplementation(async (cmd, args) => {
    switch (cmd) {
      case 'get_replace_rules': return [...db.rules]
      case 'get_config': return db.version
      case 'set_config': db.version = args.value; return
      case 'apply_default_replace_rules':
        db.rules = args.rules.map((r, i) => ({ id: 100 + i, old_text: r.oldText }))
        return
      case 'delete_replace_rule': db.rules = db.rules.filter(r => r.id !== args.id); return
      default: throw new Error(`예상하지 못한 커맨드: ${cmd}`)
    }
  })
  return db
}

const seedCalls = () => invokeMock.mock.calls.filter(([cmd]) => cmd === 'apply_default_replace_rules')

describe('fetchRules — 기본 규칙 시드 조건', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
  })

  it('버전 키가 없고 목록이 비었으면 기본 규칙을 넣는다', async () => {
    const db = fakeBackend()
    const store = useReplaceRuleStore()
    await store.fetchRules()

    expect(seedCalls()).toHaveLength(1)
    expect(db.version).toBe(String(DEFAULT_REPLACE_RULES_VERSION))
    expect(store.rules.length).toBeGreaterThan(0)
    expect(store.needsRuleUpdate).toBe(false)
    expect(store.error).toBe('')
  })

  it('마지막 규칙을 지운 뒤 재조회해도 기본 규칙을 되살리지 않는다', async () => {
    fakeBackend({ rules: [{ id: 1, old_text: 'a' }], version: String(DEFAULT_REPLACE_RULES_VERSION) })
    const store = useReplaceRuleStore()
    await store.fetchRules()
    await store.deleteRule(1)

    expect(seedCalls()).toHaveLength(0)
    expect(store.rules).toEqual([])
    expect(store.error).toBe('')
  })

  it('버전 키가 없어도 규칙이 이미 있으면 넣지 않고 갱신 표시만 켠다', async () => {
    fakeBackend({ rules: [{ id: 1, old_text: 'a' }] })
    const store = useReplaceRuleStore()
    await store.fetchRules()

    expect(seedCalls()).toHaveLength(0)
    expect(store.rules).toHaveLength(1)
    expect(store.needsRuleUpdate).toBe(true)
  })

  it('비어 있고 버전 키가 옛 값이면 넣지 않고 갱신 표시를 켠다', async () => {
    fakeBackend({ version: 'old' })
    const store = useReplaceRuleStore()
    await store.fetchRules()

    expect(seedCalls()).toHaveLength(0)
    expect(store.needsRuleUpdate).toBe(true)
  })
})
