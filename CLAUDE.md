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
- Font size: `text-base` minimum. `text-sm` / `text-xs` only for exceptions (table cell preview, badge, caption) with explicit justification.

## DB SCHEMA RULES (정식 출시 이후 적용)
- **릴리스 기준 버전은 0.2.11이다.** 그보다 이전 버전(0.1.x~0.2.10)으로 만든 파일은 모두 개발용이었고 삭제되어 **실제로 존재하지 않는다.**
- 따라서 감사에서 **0.2.11 이전 스키마와의 호환성은 범위에서 제외**한다. "옛 릴리즈 파일에 테이블·컬럼이 없다"는 류의 발견은 보고하지 말 것.
- **스키마 버전(`SCHEMA_VERSION`)을 올리는 변경은 착수 전에 반드시 사용자에게 확인한다.** 아래 절차는 확인을 받은 뒤에 수행한다.
- 이미 배포된 앱이므로 사용자 PC에 기존 구조의 DB 파일이 존재한다. **`schema.sql`을 변경할 때, 또는 `ENCRYPTED_COLUMNS`(`commands/crypto.rs`)에 컬럼을 추가·제거해 저장된 값의 표현이 바뀔 때 아래를 모두 수행한다.**
  1. `db.rs`의 `SCHEMA_VERSION`을 올린다.
  2. `db.rs`의 `MIGRATIONS`에 이전 버전 → 새 버전 SQL을 추가한다.
  3. 수정한 `schema.sql`을 `src-tauri/src/tests/schema_history/vN.sql`로 복사한다.
  4. `schema_lock_tests.rs`의 `SCHEMA_FINGERPRINTS` / `SCHEMA_BASELINES`에 항목을 추가한다.
  5. `tauri.conf.json`의 앱 버전도 함께 올린다 (릴리즈 노트 모달 표시 조건).
- **`schema_history/vN.sql`과 기존 지문 값은 절대 수정 금지.** 배포된 DB 구조의 기록이다.
- 위를 빠뜨리면 `schema_lock_tests.rs`가 실패한다. 테스트를 맞추려고 지문만 고치는 것은 금지.
- **DDL이 그대로인 버전은 이전 버전과 지문이 같다**(v1·v2가 그렇다). 중복으로 보고 지우지 말 것. 어느 파일이 이미 변환됐는지 구분할 표식이 `user_version`뿐이라 버전을 나눈 것이다.
- 키가 필요한 데이터 변환은 `MIGRATIONS`(SQL)이 아니라 `db::migrate`의 `data_step` 훅으로 간다. 버전 승격과 한 트랜잭션이어야 하기 때문이다. 어느 버전에서 무엇을 암호화하는지는 `ENCRYPTED_COLUMNS`의 `since_version` 하나가 정한다 — 호출부에서 버전별로 분기하지 말 것.
- **컬럼을 추가하는 경로만 구현되어 있다**(`encrypt_columns_introduced_in`). 제거도 버전 bump 사유이긴 하나, 대응하는 복호화 훅이 없어 그냥 빼면 그 컬럼은 암호문인 채 읽기 경로에서 `maybe_decrypt`가 빠지고 `decrypt_all_data` 대상에서도 사라진다 — 값을 되찾을 방법이 없어진다. 빼야 한다면 훅부터 만들 것.

## DESIGN DECISIONS (의도된 설계, 버그 아님)
- `restore_snapshot_impl`은 복원 전 현재 상태를 `ActivityRecordHistory`에 저장하지 않는다. 복원은 명시적 사용자 액션이므로, 히스토리 자동 저장 없이 스냅샷 시점으로 덮어쓰는 것이 의도된 동작이다.
- `upsert_record_impl`은 셀 편집 시 히스토리를 생성하지 않는다. 과도한 히스토리 누적 방지를 위한 설계다. 히스토리는 치환 적용(`apply_replace_impl`)과 수동 스냅샷 생성 시에만 기록된다.
- `ExportSection`의 `normalizeContent`(줄바꿈·이중 공백 정리)는 치환 규칙과 무관하게 항상 적용된다. 기본 치환 규칙에 같은 정규식이 있지만, 사용자가 그 규칙을 꺼도 내보내기는 정리해야 하므로 의도적으로 독립된 로직이다. 치환 엔진과 묶지 말 것.
- `backup_project`는 파일을 열 때마다 백업본을 만든다. 앱이 백업 파일을 스캔하거나 자동 삭제하지 않는 것도 의도된 동작이다. 파일명만으로는 그 파일이 이 앱이 만든 것인지 사용자가 보관 중인 것인지 구분할 수 없기 때문이다.
- **`restore_snapshot_impl`은 스냅샷 시점 이후에 만들어진 기록을 빈 값으로 되돌린다.** `COALESCE(..., '')`의 `''`가 "그 시점에 이 기록은 비어 있었다"는 뜻이다. 셀 편집은 히스토리를 남기지 않으므로(위 항목) 스냅샷 이후에 입력한 내용은 되찾을 수 없지만, **그것이 스냅샷의 정의다** — 남겨 두면 복원 결과가 그 시점의 상태가 아니게 된다. 감사에서 "데이터 손실"로 반복 보고되는 지점이니 고치지 말 것.
- **`save_snapshot_internal`은 내용이 같으면 새 히스토리 행을 만들지 않고, 가장 최근 행의 `note`를 이번 작업 이름으로 갱신한다.** 같은 내용의 행이 쌓이는 것을 막기 위한 설계다. `note`는 사용자가 쓴 메모이든 앱이 붙인 라벨(`"치환 적용 전"` 등)이든 **"이 상태가 마지막으로 어떤 작업 직전에 보존됐는가"**를 가리키므로, 최신 작업 이름으로 덮어쓰는 것이 맞다. 사용자가 직접 쓴 메모가 덮이는 경우가 있다는 보고가 반복되지만 의도된 동작이다.
- **기록 화면의 영역 선택 드롭다운에 너비 상한을 걸지 않는다.** 영역 이름은 앞부분이 같고 **끝에 구분 정보가 오는 경우가 많다**(`… (3학년 2학기)`, `… - 자율·자치활동`). `max-width`로 자르면 정작 구분해야 할 부분이 사라져 드롭다운이 제 역할을 못 한다. 그래서 이름이 길면 드롭다운이 넓어지고, 그만큼 **도구 버튼이 두 줄로 접힌다 — 그것이 의도된 폴백이다.** 툴바의 `flex-wrap`이 그 폴백이고, 두 줄이 되는 것을 레이아웃 버그로 보고하지 말 것. 창 너비(`WorkspaceView.vue`의 `setSize`)는 **흔한 길이의 이름에서 한 줄에 들어가도록** 맞춰 둔 값이지, 모든 이름을 보장하는 값이 아니다.
- **마이그레이션이 끝난 뒤의 단계는 무엇이든 열기를 막지 않는다.** 백업(`backupProject`)만이 아니라 버전 기록(`checkAndUpdateVersion`), 정리 상태 조회(`refreshEncryptionStatus`)도 마찬가지다. 파일이 이미 새 형식이라 **이전 버전으로도 열 수 없으므로**, 여기서 막으면 사용자가 파일에 갇힌다. 암호화 파일이면 그 오류가 비밀번호 모달에 떠 "비밀번호가 틀렸다"로 오해된다. 실패는 `openWarnings`에 담아 작업 화면 배너로 알린다. **`showReleaseNotesOrNavigate`에 단계를 추가할 때는 try/catch로 감쌌는지 반드시 확인할 것.**
- **백업은 마이그레이션 뒤에 한다.** `backup_project_impl`이 `ensure_migrated`로 그 순서를 강제하고, 프론트엔드도 `migrateSchema()` → `backupProject()` 순서로 부른다. 변환 전에 뜨면 메모 암호화로 넘어가는 그 한 번의 열기에서 **평문 메모가 담긴 사본**이 만들어지고, 본 DB만 암호화된 채 비밀번호 없이 읽히는 파일이 그 옆에 영구히 남는다. 마이그레이션이 실패하면 그 열기에는 백업을 만들지 않는다(열기 흐름이 거기서 멈춘다). 반대로 **백업 실패는 열기를 막지 않는다** — 마이그레이션이 이미 끝났으면 파일은 정상이고, 거기서 막으면 사본을 못 뜨는 사용자는 파일이 이미 새 형식이라 이전 버전으로도 열 수 없게 되어 갇힌다. 실패는 작업 화면 배너로 알린다.

## PROHIBITED
- Silent failures
- Business logic in frontend

## 문서에 낡는 숫자를 적지 않는다
주석·README·매뉴얼·설계 문서에 **세면 바뀌는 수**를 적지 말 것. 테스트 개수,
파일 개수, 줄 수, 단어 수, 커맨드 개수 따위는 다음 커밋에 바로 틀린 값이 되고,
아무도 고치지 않아 결국 문서 전체의 신뢰를 깎는다.

- ❌ `// 엣지 케이스 (32개)` → ✅ `// 엣지 케이스`
- ❌ `테스트 161개 전부 통과` → ✅ `전체 통과` (수는 `cargo test`가 알려준다)
- ❌ `유의어 345개 제공` → ✅ `경쟁 앱 수준 이상` 또는 수를 아예 언급하지 않기

**예외 — 적어도 되는 수:**
- 날짜가 붙은 **이력**. "2026-08-29 기준 통과", 세션 로그(`.claude/SESSIONS.md`),
  "그 세션에 테스트 3개 추가" 같은 과거 사실은 낡지 않는다.
- **고정 상수**: PBKDF2 반복 600,000, nonce 12바이트, `SCHEMA_VERSION`, 스키마 지문.
- **UI 목업 속 예시 숫자**(매뉴얼의 가짜 화면 등) — 실제 값에 대한 주장이 아니다.

판단 기준은 하나다: **"코드가 바뀌면 이 문장이 틀려지는가?"** 그렇다면 수를 빼고
확인 방법을 대신 적는다.

## 실행 가능한 명령은 셋뿐이다 — `dev` / `build` / `ci`
로컬·IntelliJ 실행 구성(`.idea/runConfigurations/`)·GitHub Actions가 **모두 이 세
이름만** 부른다. `package.json`의 나머지는 이 셋이 부르는 내부 단계다.

```
dev        tauri dev            ← 진입점
build      tauri build          ← 진입점
ci         npm run test:rust && npm run test:ts   ← 진입점(검증)
vite:dev   vite                 ← tauri.conf.json의 beforeDevCommand가 부른다
vite:build vite build           ← tauri.conf.json의 beforeBuildCommand가 부른다
test:rust  cargo test --manifest-path src-tauri/Cargo.toml
test:ts    vitest run
```

- **진입점을 늘리지 말 것.** 넷째가 생기는 순간 "어디까지 돌려야 검증인지"가 갈린다.
- **어느 한 곳에 개별 명령을 직접 적지 말 것.** 워크플로우에 `cargo test`를 따로 적거나
  IntelliJ 구성에 다른 스크립트를 넣으면 로컬과 CI가 갈라진다. 검사를 더하거나 빼려면
  `ci` 스크립트만 고친다.
- **`dev`/`build`는 `tauri`를 부르고, tauri는 다시 `vite:*`를 부른다.** `tauri.conf.json`의
  `beforeDevCommand`/`beforeBuildCommand`가 진입점 이름(`npm run dev`/`npm run build`)을
  가리키면 **무한 재귀**가 된다. 이름을 바꿀 때 반드시 같이 확인할 것.
- 빌드 옵션은 `npm run build -- <옵션>`으로 넘긴다. 플랫폼별 스크립트를 따로 만들지 말 것
  (배포 워크플로우가 이 방식을 쓴다).
- `.idea/`는 `.gitignore`에 있지만 실행 구성은 **추적 대상이다**. 새로 추가하려면
  `git add -f`가 필요하다.

## 셸 힙독 금지
- **Bash 도구에서 셸 힙독(`<<EOF`, `<<'EOF'` 등)을 쓰지 않는다.** 힙독은 백슬래시를 조용히
  먹어 `\b`가 제어문자, `\n`이 진짜 줄바꿈이 된다 — 정규식·이스케이프가 든 파일이 문법 오류
  없이 망가진 채 초록불이 될 수 있다.
- 파일은 Write/Edit 도구로 쓴다. 여러 줄을 셸로 넘겨야 하면 파일로 쓰고 경로를 넘긴다
  (`git commit -F <파일>`처럼). 히어스트링(`<<<`)은 허용한다.
- 규칙만으로는 지켜지지 않아 `.claude/hooks/no-heredoc.mjs`(PreToolUse 훅, `.claude/settings.json`)가
  도구 단계에서 막는다. `.claude/`는 `.gitignore`에 있지만 **이 두 파일은 추적 대상이다**
  (새로 추가하려면 `git add -f`).

## GIT / COMMIT RULES
- **GPG 서명 필수**: 모든 커밋에 `-S` 플래그 사용. `git commit -S -m "..."`
- **Co-Authored-By / Co-Worked 문구 삽입 금지**: 커밋 메시지에 Claude 관련 문구 일절 포함하지 않는다.
- 커밋 메시지: 한국어 또는 영어, 간결하게 작성.

### PR 머지는 로컬에서 (웹 UI 머지 금지)
GitHub 웹 UI나 `gh pr merge`로 머지하면 **GitHub이 자기 키(web-flow)로 서명**한다.
로컬에 그 공개키가 없으면 `git log --format=%G?`에서 `E`(검증 불가)로 뜨고,
"모든 커밋에 서명" 규칙이 히스토리상 깨진다. 그래서 **로컬에서 머지하고 직접 서명한다.**

```
git fetch origin
git checkout master && git pull --ff-only
git merge --no-ff origin/<PR 브랜치> -m "Merge pull request #N from <브랜치>"
git log --format="%h %G? %s" -1     # G인지 확인
git push origin master
```
- `commit.gpgsign = true`가 설정돼 있어 `git merge`도 자동으로 서명한다(확인 완료).
  명시하고 싶으면 `git merge -S`.
- `--no-ff`로 머지 커밋을 남긴다. PR head가 조상이 되므로 GitHub이 PR을 자동으로
  Merged 처리한다. 별도로 닫지 않아도 된다.
- **`merge.verifySignatures`는 켜지 말 것.** dependabot 커밋은 GitHub 키로 서명돼
  있는데 로컬에 그 공개키가 없으면 켜는 순간 머지가 전부 거부된다.

### GitHub 서명 검증용 공개키 (import 완료)
GitHub이 서명한 커밋이 `%G?`에서 `E`(검증 불가)로 뜨지 않게 하려면 공개키가 필요하다.
```
curl -fsSL https://github.com/web-flow.gpg -o web-flow.gpg
"C:/Program Files/GnuPG/bin/gpg.exe" --import web-flow.gpg
```
- **반드시 `gpg.program`이 가리키는 GnuPG에 넣어야 한다.** Git Bash의 `/usr/bin/gpg`는
  키링이 따로라 거기 넣으면 git이 못 찾는다(실제로 한 번 헛짚었다).
- 파일에 키가 둘 들어 있다. `4AEE18F83AFDEB23`은 **2024-01-16 만료된 구 키**이고,
  현재 서명에 쓰이는 것은 `B5690EEEBB952194`다.
- import 후에도 `%G?`는 `U`(유효하나 신뢰도 미지정)다. `G`로 만들려면 해당 키에
  ownertrust를 부여해야 하는데, 이는 "이 키가 GitHub의 것"이라는 신뢰 선언이므로
  선택 사항이다. 검증 자체는 `U`로도 이미 되고 있다.