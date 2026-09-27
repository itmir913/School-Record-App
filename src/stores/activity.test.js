import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args) => invokeMock(...args) }))

const { useActivityStore } = await import('./activity')

describe('쓰기 실패는 목록 상태(error)를 건드리지 않는다', () => {
  // error는 활동 목록 화면이 목록 대신 띄우는 값이다. 쓰기 실패를 거기 담으면
  // 모달 뒤의 목록이 오류 문구로 바뀐 채 남는다.
  const listed = [{ id: 1, name: '봉사', areas: [], record_count: 0 }]

  beforeEach(async () => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
    invokeMock.mockResolvedValueOnce(listed)
    await useActivityStore().fetchActivities()
  })

  it('saveActivity 실패는 던지고 목록은 그대로 둔다', async () => {
    invokeMock.mockRejectedValueOnce('UNIQUE constraint failed: Activity.name')
    const store = useActivityStore()

    await expect(store.saveActivity({ mode: 'add', name: '봉사', areaIds: [] }))
      .rejects.toBe('UNIQUE constraint failed: Activity.name')
    expect(store.error).toBe('')
    expect(store.loading).toBe(false)
    expect(store.activities).toEqual(listed)
  })

  it('createActivitiesBatch 실패도 던지기만 한다', async () => {
    invokeMock.mockRejectedValueOnce('DB 잠김')
    const store = useActivityStore()

    await expect(store.createActivitiesBatch(['새 활동'])).rejects.toBe('DB 잠김')
    expect(store.error).toBe('')
    expect(store.activities).toEqual(listed)
  })

  it('목록 읽기 실패는 error에 담긴다', async () => {
    invokeMock.mockRejectedValueOnce('읽기 실패')
    const store = useActivityStore()

    await expect(store.fetchActivities()).rejects.toBe('읽기 실패')
    expect(store.error).toBe('읽기 실패')
  })
})

describe('saveActivity는 저장과 영역 연결을 한 번의 호출로 끝낸다', () => {
  // 따로 부르면 활동만 만들어진 채 연결에서 실패할 수 있다. 원자성은 백엔드
  // save_activity의 트랜잭션이 보장하므로, 스토어는 그 커맨드 하나만 불러야 한다.
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeMock.mockReset()
  })

  it('추가 모드는 id 없이 save_activity 하나만 부른다', async () => {
    invokeMock.mockResolvedValueOnce(7).mockResolvedValueOnce([])
    await useActivityStore().saveActivity({ mode: 'add', id: 3, name: '봉사', areaIds: [1, 2] })

    expect(invokeMock.mock.calls).toEqual([
      ['save_activity', { id: null, name: '봉사', areaIds: [1, 2] }],
      ['get_activities'],
    ])
  })

  it('수정 모드는 기존 id를 넘긴다', async () => {
    invokeMock.mockResolvedValueOnce(3).mockResolvedValueOnce([])
    await useActivityStore().saveActivity({ mode: 'edit', id: 3, name: '봉사', areaIds: [] })

    expect(invokeMock.mock.calls[0]).toEqual(['save_activity', { id: 3, name: '봉사', areaIds: [] }])
    expect(invokeMock).toHaveBeenCalledTimes(2)
  })
})
