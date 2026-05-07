import { describe, it, expect } from 'vitest'
import { getNotesToShow, RELEASE_NOTES, ReleaseNote } from './releaseNotes'

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

  it('기본 인자 미전달(null) → RELEASE_NOTES 전체 반환', () => {
    const result = getNotesToShow(null)
    expect(result).toEqual(RELEASE_NOTES)
  })

  it('단일 아이템 배열 + 매칭 → 빈 배열', () => {
    const single: ReleaseNote[] = [{ version: '0.3.0', date: '2026-06-01' }]
    expect(getNotesToShow('0.3.0', single)).toEqual([])
  })

  it('단일 아이템 배열 + 미매칭 → 전체 반환', () => {
    const single: ReleaseNote[] = [{ version: '0.3.0', date: '2026-06-01' }]
    expect(getNotesToShow('0.2.0', single)).toEqual(single)
  })

  it('빈 문자열 + 빈 배열 → 빈 배열 반환', () => {
    expect(getNotesToShow('', [])).toEqual([])
  })
})

describe('RELEASE_NOTES 구조 무결성', () => {
  it('length ≥ 1, 날짜 내림차순, [0]이 최신', () => {
    expect(RELEASE_NOTES.length).toBeGreaterThanOrEqual(1)
    for (let i = 0; i < RELEASE_NOTES.length - 1; i++) {
      expect(new Date(RELEASE_NOTES[i].date).getTime()).toBeGreaterThanOrEqual(
        new Date(RELEASE_NOTES[i + 1].date).getTime()
      )
    }
  })

  it('버전 중복 없음', () => {
    const versions = RELEASE_NOTES.map(n => n.version)
    expect(new Set(versions).size).toBe(versions.length)
  })
})
