import { describe, it, expect } from 'vitest'
import { Workbook } from 'exceljs'
import { cellValue, decodeCsvBytes, loadXlsxRows, parseCsv } from './spreadsheet'

describe('parseCsv', () => {
  it('쉼표로 나누고 행 단위로 모은다', () => {
    expect(parseCsv('a,b\n1,2')).toEqual([['a', 'b'], ['1', '2']])
  })

  it('따옴표 안의 LF 줄바꿈은 필드 내용이다', () => {
    const text = '학년,내용\n1,"첫 줄.\n둘째 줄, 쉼표 포함."\n2,정상\n'
    expect(parseCsv(text)).toEqual([
      ['학년', '내용'],
      ['1', '첫 줄.\n둘째 줄, 쉼표 포함.'],
      ['2', '정상'],
    ])
  })

  it('따옴표 안의 CRLF·CR 줄바꿈은 LF로 보존한다', () => {
    expect(parseCsv('a\r\n"x\r\ny"\r\n"p\rq"')).toEqual([['a'], ['x\ny'], ['p\nq']])
  })

  it('따옴표 안의 빈 줄도 필드 내용으로 남는다', () => {
    expect(parseCsv('a\n"첫 문단\n\n둘째 문단"')).toEqual([['a'], ['첫 문단\n\n둘째 문단']])
  })

  it('이스케이프된 따옴표("")는 따옴표 하나다', () => {
    expect(parseCsv('a\n"그는 ""좋다""고 함"')).toEqual([['a'], ['그는 "좋다"고 함']])
  })

  it('마지막 줄에 개행이 없어도 마지막 행을 읽는다', () => {
    expect(parseCsv('a,b\n1,2')).toEqual([['a', 'b'], ['1', '2']])
    expect(parseCsv('a,b\n1,"2"')).toEqual([['a', 'b'], ['1', '2']])
  })

  it('따옴표 밖의 빈 줄과 공백뿐인 줄은 건너뛴다', () => {
    expect(parseCsv('a\r\n\r\n   \r\n1\r\n\r\n')).toEqual([['a'], ['1']])
  })

  it('필드 중간의 따옴표는 글자 그대로다 — 뒤 행을 삼키지 않는다', () => {
    // 인용 시작으로 보면 짝이 없는 따옴표 하나가 파일 끝까지를 한 칸으로 삼켜,
    // 뒤 학생 행이 사라지고 앞 학생의 기록에 섞인다.
    expect(parseCsv('a,b\n1,키 170" 정도\n2,정상')).toEqual([
      ['a', 'b'],
      ['1', '키 170" 정도'],
      ['2', '정상'],
    ])
  })

  it('쉼표 뒤 공백 다음의 따옴표도 인용 시작이다 — 앞 공백은 예전처럼 남긴다', () => {
    // 인용으로 보지 않으면 인용 안의 쉼표에서 칸이 조용히 쪼개진다.
    expect(parseCsv('a,b\n1, "내용, 쉼표"\n2,\t"탭"')).toEqual([
      ['a', 'b'],
      ['1', ' 내용, 쉼표'],
      ['2', '\t탭'],
    ])
  })

  it('닫힌 인용 뒤에 붙은 글자와 따옴표도 그대로 둔다', () => {
    expect(parseCsv('a\n"인용"뒤 "말"')).toEqual([['a'], ['인용뒤 "말"']])
  })

  it('파일 끝까지 따옴표가 닫히지 않으면 시작한 행을 알려주며 실패한다', () => {
    expect(() => parseCsv('a,b\n1,정상\n2,"닫히지 않음\n3,뒤 행'))
      .toThrow('따옴표가 닫히지 않은 칸이 있습니다(3행 부근)')
  })

  it('끝의 빈 필드를 유지한다', () => {
    expect(parseCsv('a,b,\n1,,')).toEqual([['a', 'b', ''], ['1', '', '']])
  })
})

describe('decodeCsvBytes', () => {
  const utf8 = (s: string) => new TextEncoder().encode(s)

  it('BOM을 떼어 첫 헤더에 붙지 않게 한다', () => {
    const bytes = new Uint8Array([0xEF, 0xBB, 0xBF, ...utf8('학년,반\n1,2')])
    expect(parseCsv(decodeCsvBytes(bytes.buffer))).toEqual([['학년', '반'], ['1', '2']])
  })

  it('BOM 없는 UTF-8', () => {
    expect(decodeCsvBytes(utf8('이름').buffer)).toBe('이름')
  })

  it('UTF-8이 아니면 EUC-KR로 읽는다', () => {
    // '학년' in EUC-KR
    const bytes = new Uint8Array([0xC7, 0xD0, 0xB3, 0xE2])
    expect(decodeCsvBytes(bytes.buffer)).toBe('학년')
  })
})

describe('cellValue', () => {
  it('빈 값은 빈 문자열', () => {
    expect(cellValue(null)).toBe('')
    expect(cellValue(undefined)).toBe('')
  })

  it('원시값은 문자열로', () => {
    expect(cellValue(3)).toBe('3')
    expect(cellValue('홍길동')).toBe('홍길동')
  })

  it('서식 있는 텍스트는 조각을 잇는다', () => {
    expect(cellValue({richText: [{text: '가'}, {text: '나'}]})).toBe('가나')
  })

  it('하이퍼링크는 표시 텍스트', () => {
    expect(cellValue({text: '링크', hyperlink: 'https://example.com'})).toBe('링크')
  })

  it('수식 셀은 계산 결과를 쓴다', () => {
    expect(cellValue({formula: 'ROW()-1', result: 1})).toBe('1')
    expect(cellValue({formula: '"가"&"나"', result: '가나'})).toBe('가나')
  })

  it('공유 수식 셀도 계산 결과를 쓴다', () => {
    expect(cellValue({sharedFormula: 'C2', result: 2})).toBe('2')
  })

  it('수식 결과가 서식 있는 텍스트여도 풀어낸다', () => {
    expect(cellValue({formula: 'A1', result: {richText: [{text: '가'}]}})).toBe('가')
  })

  it('오류 셀은 엑셀에 보이는 오류값 그대로 남긴다', () => {
    expect(cellValue({error: '#N/A'})).toBe('#N/A')
    expect(cellValue({formula: 'VLOOKUP(1,A:B,2,0)', result: {error: '#N/A'}})).toBe('#N/A')
  })
})

describe('loadXlsxRows', () => {
  it('실제 xlsx의 수식 셀을 계산 결과로 읽고 짧은 행을 헤더 길이로 채운다', async () => {
    const wb = new Workbook()
    const ws = wb.addWorksheet('a')
    ws.addRow(['학년', '반', '번호', '내용'])
    ws.addRow([1, 2, {formula: 'ROW()-1', result: 1}, {formula: '"가"&"나"', result: '가나'}])
    ws.addRow([1, 2, 3])
    const buffer = await wb.xlsx.writeBuffer()

    expect(await loadXlsxRows(buffer as ArrayBuffer)).toEqual([
      ['학년', '반', '번호', '내용'],
      ['1', '2', '1', '가나'],
      ['1', '2', '3', ''],
    ])
  })
})
