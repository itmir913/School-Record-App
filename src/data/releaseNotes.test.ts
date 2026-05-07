import { describe, it, expect } from 'vitest'
import { getNotesToShow, ReleaseNote } from './releaseNotes'

const MOCK: ReleaseNote[] = [
  { version: '0.3.0',  date: '2026-06-01', features: ['F3'] },
  { version: '0.2.12', date: '2026-05-07', features: ['F2'] },
  { version: '0.2.11', date: '2026-04-01', features: ['F1'] },
]

describe('getNotesToShow', () => {
  it('null → 전체 반환', () => {
    expect(getNotesToShow(null, MOCK)).toEqual(MOCK)
  })

  it('빈 문자열 → 전체 반환', () => {
    expect(getNotesToShow('', MOCK)).toEqual(MOCK)
  })

  it('최신 버전(0.3.0) → 빈 배열', () => {
    expect(getNotesToShow('0.3.0', MOCK)).toEqual([])
  })

  it('0.2.12 → 0.3.0 1개', () => {
    const result = getNotesToShow('0.2.12', MOCK)
    expect(result).toHaveLength(1)
    expect(result[0].version).toBe('0.3.0')
  })

  it('0.2.11 → 0.3.0·0.2.12 2개', () => {
    const result = getNotesToShow('0.2.11', MOCK)
    expect(result).toHaveLength(2)
    expect(result[0].version).toBe('0.3.0')
    expect(result[1].version).toBe('0.2.12')
  })

  it('배열에 없는 버전(0.1.0) → 전체 반환 (안전 실패)', () => {
    expect(getNotesToShow('0.1.0', MOCK)).toEqual(MOCK)
  })

  it('null + 빈 배열 주입 → 빈 배열 반환', () => {
    expect(getNotesToShow(null, [])).toEqual([])
  })
})
