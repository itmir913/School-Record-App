export interface ReleaseNote {
  version: string
  date: string

  breaking?: string[]
  features?: string[]
  improvements?: string[]
  bugFixes?: string[]
}

export const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: '0.2.18',
    date: '2026-06-26',
    bugFixes: [
      '한셀 등 MS Excel 이외 프로그램에서 저장한 xlsx 파일 가져오기 오류 수정',
    ],
  },
  {
    version: '0.2.17',
    date: '2026-06-22',
    features: [
      '학생별 작성 내용을 한눈에 확인하는 미리보기 열 추가',
      '학년·반·번호 열을 숨겨 작업 공간을 넓게 쓸 수 있는 기능 추가',
    ],
    improvements: [
      '바이트 초과·미기록 학생 강조 표시가 미리보기 열에도 동일하게 적용',
      '클릭 가능한 헤더에 밑줄 표시 추가',
      '툴바 버튼 디자인 및 기록 입력 반응 속도 개선',
    ],
    bugFixes: [
      '미리보기에서 접힌 활동 항목을 클릭하면 해당 칸으로 바로 이동하도록 수정',
      '좌우 스크롤 시 강조 배경색이 번져 보이는 현상 수정',
    ],
  },
  {
    version: '0.2.16',
    date: '2026-06-17',
    bugFixes: [
      '라이트 모드 색상 오류 전반 수정',
      '페이지 전환 트랜지션 및 레이아웃 정렬 수정',
    ],
  },
  {
    version: '0.2.15',
    date: '2026-06-17',
    breaking: [
      '라이트 / 다크 테마 전환 기능 추가',
    ],
  },
  {
    version: '0.2.14',
    date: '2026-06-16',
    features: [
      '영역별 학생 일괄 선택 및 엑셀 업로드 자동 배정 기능 추가',
      '오프라인 사용 매뉴얼 내장 (인터넷 없이 매뉴얼 열람 가능)',
    ],
  },
  {
    version: '0.2.13',
    date: '2026-05-08',
    breaking: [
      '데이터베이스 보안 강화를 위한 암호화 기능 도입 (Thanks to @donginssam)',
    ],
    improvements: [
      '업데이트 후 첫 실행 시 릴리즈 노트 자동 표시',
      '생기부 작성 영역 선택 시 이름 순 정렬',
    ],
    bugFixes: [
      'DB 마이그레이션 코드 안정성 향상',
    ],
  },
  {
    version: '0.2.12',
    date: '2026-04-30',
    features: [
      '생기부 작성 화면 글자 크기 조절 기능 추가',
      'macOS Universal(M1/M2 등 애플 실리콘) 지원 및 DMG 배포 방식 추가',
    ],
    improvements: [
      'CSV 파일 인코딩 자동 감지 적용',
      '학생 명렬표 샘플 엑셀(XLSX)로 변경',
      '스냅샷 창 실수로 닫힘 방지 적용',
      '학생 삭제 시 스냅샷까지 영구 삭제된다는 경고 강화',
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
export function getNotesToShow(storedVersion: string | null, notes: ReleaseNote[] = RELEASE_NOTES): ReleaseNote[] {
  if (!storedVersion) return [...notes]

  for (let i = 0; i < notes.length; i++) {
    if (notes[i].version === storedVersion) {
      return notes.slice(0, i)
    }
  }

  return [...notes]
}