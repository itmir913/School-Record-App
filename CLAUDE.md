# GLOBAL RULES

## ARCHITECTURE
- Component MUST NOT call invoke()
- ALWAYS use Store
- Rust handles ALL DB and core logic

## PRINCIPLES
- Store = single source of truth
- Frontend = UI + state only
- No duplicated logic

## CONVENTIONS
- Rust commands: snake_case
- MUST handle errors explicitly

## PROHIBITED
- Silent failures
- Business logic in frontend

## GIT / COMMIT RULES
- **GPG 서명 필수**: 모든 커밋에 `-S` 플래그 사용. `git commit -S -m "..."`
- **Co-Authored-By / Co-Worked 문구 삽입 금지**: 커밋 메시지에 Claude 관련 문구 일절 포함하지 않는다.
- 커밋 메시지: 한국어 또는 영어, 간결하게 작성.