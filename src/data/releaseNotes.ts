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
    date: '2026-04-30',
    features: [
      '생기부 작성 화면 글자 크기 조절 기능 추가',
      'macOS Universal(M1/M2 등 애플 실리콘) 지원 및 DMG 배포 방식 추가',
      '업데이트 후 첫 실행 시 릴리즈 노트 자동 표시',
    ],
    improvements: [
      'CSV 파일 인코딩 자동 감지 적용 및 학생 명렬표 샘플 엑셀(XLSX)로 변경',
      '스냅샷 창 실수로 닫힘 방지 적용',
      '학생 삭제 시 스냅샷까지 영구 삭제된다는 경고 강화',
      '엑셀 파일 불러오기 방식 개선',
      '내부 코드 구조 개선 및 시스템 전반의 안정성 향상',
    ],
  },
  {
    version: '0.2.11',
    date: '2026-04-27',
    features: [
      '초기 릴리즈',
    ],
  },
]

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