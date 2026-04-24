interface SynonymWordItem {
  id: number
  group_id: number
  word: string
}

interface SynonymGroupFull {
  id: number
  name: string
  created_at: string
  items: SynonymWordItem[]
}

interface InspectRecord {
  id: number
  activity_name: string
  student_name: string
  area_name: string
  grade: number
  class_num: number
  number: number
  content: string
}

export interface InspectResult {
  record: InspectRecord
  detectedWords: string[]
}

function escapeRegex(word: string): string {
  return word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function performInspection(
  selectedGroupIds: number[],
  groups: SynonymGroupFull[],
  records: InspectRecord[],
): InspectResult[] {
  const wordSet = new Set<string>()
  for (const group of groups) {
    if (selectedGroupIds.includes(group.id)) {
      for (const item of group.items) {
        const w = item.word.trim()
        if (w) wordSet.add(w)
      }
    }
  }

  if (wordSet.size === 0) return []

  const pattern = Array.from(wordSet).map(escapeRegex).join('|')
  const regex = new RegExp(pattern, 'g')

  return records.flatMap((record) => {
    const matches = record.content.match(regex)
    if (!matches) return []
    return [{ record, detectedWords: [...new Set(matches)] }]
  })
}
