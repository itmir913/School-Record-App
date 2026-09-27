import { describe, it, expect } from 'vitest'
import { performInspection } from './synonymService'

const group = (id: number, name: string, words: string[]) => ({
  id, name, created_at: '', items: words.map((word, i) => ({id: i + 1, group_id: id, word})),
})

const record = (id: number, content: string) => ({
  id, activity_name: '활동', student_name: '홍길동', area_name: '영역',
  grade: 1, class_num: 1, number: 1, content,
})

describe('performInspection', () => {
  it('선택한 그룹의 단어만 찾는다', () => {
    const groups = [group(1, '노력', ['노력', '열심']), group(2, '기타', ['협동'])]
    const records = [record(1, '노력하고 협동함')]

    const result = performInspection([1], groups, records)
    expect(result).toHaveLength(1)
    expect(result[0].detectedWords).toEqual(['노력'])
  })

  it('선택한 그룹이 없으면 빈 배열', () => {
    expect(performInspection([], [group(1, 'g', ['노력'])], [record(1, '노력함')])).toEqual([])
  })

  it('단어가 없는 그룹만 선택하면 빈 배열', () => {
    expect(performInspection([1], [group(1, 'g', [])], [record(1, '노력함')])).toEqual([])
  })

  it('공백뿐인 단어는 무시한다', () => {
    expect(performInspection([1], [group(1, 'g', ['  '])], [record(1, '노력함')])).toEqual([])
  })

  it('매칭되지 않은 기록은 결과에서 빠진다', () => {
    const result = performInspection([1], [group(1, 'g', ['노력'])], [
      record(1, '노력함'),
      record(2, '전혀 다른 내용'),
    ])
    expect(result.map(r => r.record.id)).toEqual([1])
  })

  it('같은 단어가 여러 번 나와도 한 번만 보고한다', () => {
    const result = performInspection([1], [group(1, 'g', ['노력'])], [record(1, '노력 노력 노력')])
    expect(result[0].detectedWords).toEqual(['노력'])
  })

  it('여러 그룹의 단어를 함께 찾는다', () => {
    const groups = [group(1, 'a', ['노력']), group(2, 'b', ['협동'])]
    const result = performInspection([1, 2], groups, [record(1, '노력하고 협동함')])
    expect(result[0].detectedWords.sort()).toEqual(['노력', '협동'])
  })

  it('정규식 특수문자가 단어에 있어도 문자 그대로 찾는다', () => {
    // escapeRegex가 없으면 '(' 하나로 정규식이 깨지거나 엉뚱하게 매칭된다.
    const result = performInspection([1], [group(1, 'g', ['C++', 'a.b'])], [
      record(1, 'C++를 배우고 a.b를 씀'),
    ])
    expect(result[0].detectedWords.sort()).toEqual(['C++', 'a.b'])
  })

  it('정규식 특수문자가 임의 문자로 매칭되지 않는다', () => {
    // 'a.b'가 이스케이프되지 않으면 'axb'에도 매칭된다.
    expect(performInspection([1], [group(1, 'g', ['a.b'])], [record(1, 'axb')])).toEqual([])
  })

  it('영문 단어는 대소문자를 구분한다', () => {
    expect(performInspection([1], [group(1, 'g', ['AI'])], [record(1, 'ai 활용')])).toEqual([])
  })

  it('중복 단어는 한 번만 검사한다', () => {
    const groups = [group(1, 'a', ['노력']), group(2, 'b', ['노력'])]
    const result = performInspection([1, 2], groups, [record(1, '노력함')])
    expect(result[0].detectedWords).toEqual(['노력'])
  })
})
