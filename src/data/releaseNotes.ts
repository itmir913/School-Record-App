export interface ReleaseNote {
  version: string
  date: string
  features?: string[]
  improvements?: string[]
  bugFixes?: string[]
  breaking?: string[]
}

export const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: '0.2.12',
    date: '2026-05-07',
    features: [
      '업데이트 후 첫 실행 시 릴리즈 노트 자동 표시',
    ],
    improvements: [
      'DB 암호화 엣지 케이스 처리 강화',
      '내부 코드 구조 개선 및 안정성 향상',
    ],
  },
]

export function getReleaseNote(version: string): ReleaseNote | undefined {
  return RELEASE_NOTES.find(note => note.version === version)
}

/**
 * 사용자가 마지막으로 실행한 버전(storedVersion) 이후의 노트를 반환한다.
 * - storedVersion이 null/빈 문자열이면 → 전체 반환 (이전 레코드 없음)
 * - 배열에서 storedVersion을 찾으면 그 인덱스 직전까지만 반환
 * - 배열에서 storedVersion을 못 찾으면 → 전체 반환 (안전 실패)
 * RELEASE_NOTES는 [0]이 최신인 스택 구조여야 한다.
 */
export function getNotesToShow(storedVersion: string | null): ReleaseNote[] {
  if (!storedVersion) return [...RELEASE_NOTES]

  for (let i = 0; i < RELEASE_NOTES.length; i++) {
    if (RELEASE_NOTES[i].version === storedVersion) {
      return RELEASE_NOTES.slice(0, i)
    }
  }

  return [...RELEASE_NOTES]
}
