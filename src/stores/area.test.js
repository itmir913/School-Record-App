import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args) => invokeMock(...args) }))

const { useAreaStore } = await import('./area')

describe('saveArea는 저장과 활동 연결을 한 번의 호출로 끝낸다', () => {
  // 따로 부르면 영역만 만들어진 채 연결에서 실패할 수 있다. 원자성은 백엔드
  // save_area의 트랜잭션이 보장하므로, 스토어는 그 커맨드 하나만 불러야 한다.
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
  })

  it('추가 모드는 id 없이 save_area 하나만 부르고 새 id를 돌려준다', async () => {
    invokeMock.mockResolvedValueOnce(5).mockResolvedValueOnce([])
    const id = await useAreaStore().saveArea({
      mode: 'add', id: undefined, name: '자율', byteLimit: 500, activityIds: [1],
    })

    expect(id).toBe(5)
    expect(invokeMock.mock.calls).toEqual([
      ['save_area', { id: null, name: '자율', byteLimit: 500, activityIds: [1] }],
      ['get_areas'],
    ])
  })

  it('수정 모드는 기존 id를 넘긴다', async () => {
    invokeMock.mockResolvedValueOnce(2).mockResolvedValueOnce([])
    await useAreaStore().saveArea({ mode: 'edit', id: 2, name: '자율', byteLimit: 300, activityIds: [] })

    expect(invokeMock.mock.calls[0]).toEqual(['save_area', { id: 2, name: '자율', byteLimit: 300, activityIds: [] }])
    expect(invokeMock).toHaveBeenCalledTimes(2)
  })

  it('저장 실패는 그대로 던지고 목록을 다시 읽지 않는다', async () => {
    invokeMock.mockRejectedValueOnce('이미 같은 이름의 영역이 있습니다: 자율')

    await expect(useAreaStore().saveArea({ mode: 'add', name: '자율', byteLimit: 500, activityIds: [] }))
      .rejects.toBe('이미 같은 이름의 영역이 있습니다: 자율')
    expect(invokeMock).toHaveBeenCalledTimes(1)
  })
})
