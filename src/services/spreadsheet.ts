/**
 * CSV·엑셀 파일을 행 배열(string[][])로 읽는다.
 *
 * 활동 기록 가져오기, 학생 일괄 추가, 영역 학생 배정이 같은 파일을 받으므로
 * 읽기 규칙을 한곳에 둔다. 따로 두면 한쪽만 고쳐져 같은 파일이 화면마다
 * 다르게 읽힌다.
 */

import {Workbook} from 'exceljs'
import * as XLSX from 'xlsx'

/**
 * CSV 바이트를 문자열로 푼다. BOM이 있으면 UTF-8로 읽고 떼어낸다(떼지 않으면
 * 첫 헤더가 '﻿학년'이 되어 열 자동 인식에서 빠진다). BOM이 없으면 UTF-8을
 * 먼저 시도하고, 실패하면 한국어 엑셀의 기본 저장 형식인 EUC-KR로 읽는다.
 */
export function decodeCsvBytes(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  if (bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF)
    return new TextDecoder('utf-8').decode(bytes.subarray(3))
  for (const enc of ['utf-8', 'euc-kr'])
    try { return new TextDecoder(enc, { fatal: true }).decode(bytes) } catch {}
  return new TextDecoder('utf-8').decode(bytes)
}

/**
 * CSV 문자열을 행 배열로 나눈다.
 *
 * 문자 단위로 읽어 **따옴표 안의 줄바꿈은 필드 내용으로** 남긴다. 예전에는 줄
 * 단위로 먼저 쪼갠 뒤 따옴표를 봐서, 셀 안에서 줄을 바꾼 기록이 첫 줄에서 잘리고
 * 나머지 줄은 학년을 읽을 수 없는 행으로 버려졌다. 기존 기록이 있으면 잘린 내용이
 * 덮어쓰기 대상이 됐다. 필드 안의 CRLF·CR은 LF로 통일한다.
 *
 * 따옴표는 **필드의 첫 글자일 때만**(앞 공백·탭은 건너뛰고) 인용 시작으로 본다.
 * 필드 중간의 따옴표(`키 170" 정도`)까지 인용으로 보면, 짝이 없는 따옴표 하나가
 * 파일 끝까지를 한 칸으로 삼켜 뒤 학생 행이 경고 없이 사라지고 앞 학생의 기록에
 * 섞인다. 첫 글자에서 연 따옴표가 파일 끝까지 닫히지 않으면 같은 일이 생기므로
 * 조용히 넘기지 않고 연 위치를 알려주며 실패한다.
 *
 * 따옴표 밖의 빈 줄(공백뿐인 줄 포함)은 건너뛴다.
 */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = []
  let row: string[] = []
  let field = ''
  // 이 행에서 읽은 원문. 빈 줄 판정에만 쓴다.
  let raw = ''
  let inQuotes = false
  let atFieldStart = true
  // 파일의 몇 번째 줄인가(1부터). 닫히지 않은 따옴표의 위치 안내에만 쓴다.
  let line = 1
  let quoteLine = 0

  const endRecord = () => {
    row.push(field)
    if (raw.trim() !== '') rows.push(row)
    row = []
    field = ''
    raw = ''
    atFieldStart = true
  }

  for (let i = 0; i < text.length; i++) {
    const ch = text[i]
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"'
          raw += '""'
          i++
        } else {
          inQuotes = false
          raw += ch
        }
      } else if (ch === '\r' || ch === '\n') {
        field += '\n'
        raw += ch
        if (ch === '\r' && text[i + 1] === '\n') i++
        line++
      } else {
        field += ch
        raw += ch
      }
    } else if (ch === '"' && atFieldStart) {
      inQuotes = true
      atFieldStart = false
      quoteLine = line
      raw += ch
    } else if (ch === ',') {
      row.push(field)
      field = ''
      raw += ch
      atFieldStart = true
    } else if (ch === '\r' || ch === '\n') {
      if (ch === '\r' && text[i + 1] === '\n') i++
      line++
      endRecord()
    } else {
      field += ch
      raw += ch
      // 앞 공백(`1, "내용, 쉼표"`)은 아직 필드 시작으로 본다. 공백은 예전 파서처럼
      // 내용에 남긴다.
      if (ch !== ' ' && ch !== '\t') atFieldStart = false
    }
  }
  if (inQuotes) {
    throw new Error(
        `따옴표가 닫히지 않은 칸이 있습니다(${quoteLine}행 부근). ` +
        '칸 안에서 따옴표로 시작했다면 닫는 따옴표가 있는지 확인해 주세요.')
  }
  // 마지막 줄에 개행이 없는 경우
  if (raw !== '') endRecord()
  return rows
}

/**
 * exceljs 셀 값을 문자열로 바꾼다.
 *
 * - 수식 셀(`{formula|sharedFormula, result}`)은 계산 결과를 쓴다. 객체를 그대로
 *   문자열로 만들면 '[object Object]'가 되어, 번호 열이면 행이 전부 빠지고 내용
 *   열이면 그 문자열이 기록으로 저장됐다.
 * - 오류 셀(`{error}`)은 엑셀 화면에 보이는 오류값('#N/A' 등)을 그대로 남긴다.
 *   빈 값으로 바꾸면 "비어 있음"과 구분되지 않는다. 식별 열이면 해석할 수 없는
 *   행으로 제외 안내에 잡히고, 내용 열이면 미리보기에 그대로 보인다.
 */
export function cellValue(v: any): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'object') {
    if (v instanceof Date) return v.toLocaleDateString()
    if (v.richText) return v.richText.map((r: { text: string }) => r.text).join('')
    if ('formula' in v || 'sharedFormula' in v) return cellValue(v.result)
    if (v.error !== undefined) return String(v.error)
    if (v.text !== undefined) return cellValue(v.text)
  }
  return String(v)
}

// 뒤쪽 빈 칸이 생략된 행을 헤더 길이만큼 채운다.
function padToHeader(rows: string[][]): string[][] {
  if (rows.length > 1) {
    const headerLen = rows[0].length
    for (let i = 1; i < rows.length; i++) {
      while (rows[i].length < headerLen) rows[i].push('')
    }
  }
  return rows
}

/** xlsx 첫 시트를 행 배열로 읽는다. exceljs가 못 읽는 파일은 SheetJS로 다시 읽는다. */
export async function loadXlsxRows(buffer: ArrayBuffer): Promise<string[][]> {
  try {
    const workbook = new Workbook()
    await workbook.xlsx.load(buffer)
    const worksheet = workbook.worksheets[0]
    const rows: string[][] = []
    worksheet.eachRow((row) => {
      rows.push((row.values as unknown[]).slice(1).map(cellValue))
    })
    return padToHeader(rows)
  } catch {
    // 한셀 등 비표준 xlsx 폴백
    const wb = XLSX.read(buffer, {type: 'array'})
    const ws = wb.Sheets[wb.SheetNames[0]]
    const raw = XLSX.utils.sheet_to_json<unknown[]>(ws, {header: 1, defval: ''})
    return padToHeader(raw.map(row => row.map(v => (v === null || v === undefined) ? '' : String(v))))
  }
}
