export interface DefaultReplaceRule {
  oldText: string
  newText: string
  priority: number
  isRegex?: boolean
}

export const DEFAULT_REPLACE_RULES: DefaultReplaceRule[] = [
  { oldText: '\u201C', newText: "'", priority: 0 },  // 왼쪽 쌍따옴표 "
  { oldText: '\u201D', newText: "'", priority: 1 },  // 오른쪽 쌍따옴표 "
  { oldText: '\u2018', newText: "'", priority: 2 },  // 왼쪽 홑따옴표 '
  { oldText: '\u2019', newText: "'", priority: 3 },  // 오른쪽 홑따옴표 '
  { oldText: '「', newText: "'", priority: 4 },
  { oldText: '」', newText: "'", priority: 5 },
  { oldText: '『', newText: "'", priority: 6 },
  { oldText: '』', newText: "'", priority: 7 },
  { oldText: '<', newText: "'", priority: 8 },
  { oldText: '>', newText: "'", priority: 9 },
  { oldText: '`', newText: "'", priority: 10 },
  { oldText: '~', newText: "-", priority: 11 },
  { oldText: '\\n+', newText: ' ', priority: 12, isRegex: true },
  { oldText: ' {2,}', newText: ' ', priority: 13, isRegex: true },
]
