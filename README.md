# Nora Schedule

일정, 할 일, 디데이, 메모, 집중 시간까지 한곳에서 관리하는 가벼운 데스크톱 스케줄러입니다.
모든 데이터는 내 컴퓨터에만 저장돼요.

## 공지사항
Windows 11 호환. (MacOS는 제가 자고 일어나서 올리든가 하겠습니다.)

참고로 이 섹션 말고는 다 Claude Code가 썼습니다. 알아서 쓰세요.

## 주요 기능

### 한눈에 보기 (홈)
- 위젯을 골라 배치하는 대시보드 (인사말, 오늘 일정, 달력, 할 일, 디데이, 뽀모도로, 작업 시간, 주간 차트, 메모, 북마크, 가계부 등)
- 위젯 크기 조절, 드래그로 순서 변경
- 헤더 이미지와 꾸미기용 이미지 카드 (드래그·확대로 보이는 부분 조절)
- 달력 위젯에서 날짜에 마우스를 올리면 그날 일정 미리보기

### 캘린더
- 월간 / 주간 / 일간 보기, 드래그로 일정 만들기
- 여러 날짜에 걸친 일정은 끊기지 않는 막대로 표시
- 반복 일정 (매일·매주·매월·매년, 횟수/종료일 지정, "이 일정만 / 모든 일정" 수정)
- 일정 취소 (취소선으로 표시, 되돌리기 가능)
- 태그 (색·카테고리 지정, 태그별 필터)
- 사전 알림 (없음, 5분, 10분, 30분, 1시간, 사용자 지정) — 알림음과 Windows 알림
- 일정에 마우스를 올리면 시간·장소·태그·메모 표시
- 마감일이 있는 할 일도 캘린더에 함께 표시 (클릭해서 완료)
- 복사·붙여넣기 (Ctrl+C / Ctrl+V), 우클릭 메뉴, 실행 취소

### 할 일
- 그룹별 관리, 하위 할 일, 마감일·마감 시간

### 디데이
- 다가오는 날 / 지난 날 카운트, 커버 사진
- 매년 반복 (생일·기념일), 1일부터 세기 (사귄 날 = D+1)

### 그 밖의 기능
- **뽀모도로** 타이머와 집중 기록
- **메모** (리치 텍스트 편집)
- **작업 시간 추적**: 지정한 앱에서 보낸 시간만 기록, 자리 비움 감지, 앱·창 제목별 통계
- **통계**: 집중 시간, 할 일 완료, 작업 시간 차트
- **북마크**, **가계부**

### 꾸미기와 설정
- 라이트 / 다크 / 시스템 테마, 색상 테마 프리셋과 직접 만들기 (메인 색, 보조 색, 배경 색감)
- Windows 제목 표시줄 색이 테마를 따라감
- 스티커: 내 이미지(투명 PNG, 움직이는 GIF 포함)를 아무 페이지에나 붙이고 옮기기·크기·회전 조절 (꾸미기 모드에서만 편집, 평소에는 클릭을 방해하지 않음)
- 한국어 / English, Pretendard 글꼴
- 창 크기·위치 기억, 크기 프리셋
- 전체 데이터 백업·복원 (이미지 포함), 초기화

## 기술 스택

| 영역 | 사용 기술 |
|---|---|
| 앱 프레임워크 | [Tauri 2](https://tauri.app) (WebView2) |
| 백엔드 | Rust (edition 2024), SQLite ([rusqlite](https://github.com/rusqlite/rusqlite), 번들 빌드), chrono, serde, image, zip |
| 프론트엔드 | [Svelte 5](https://svelte.dev) (runes), TypeScript, Vite |
| 에디터 | [Tiptap 3](https://tiptap.dev) (Markdown 지원), marked, DOMPurify |
| UI | Lucide 아이콘, Pretendard 글꼴, 직접 만든 CSS 디자인 시스템 |
| Tauri 플러그인 | dialog, notification, opener, window-state |
| 작업 시간 추적 | active-win-pos-rs (현재 활성 창 읽기) |

데이터는 OS의 앱 데이터 폴더(Windows: `%APPDATA%\com.nora.schedule`)에 SQLite 파일과 이미지 폴더로 저장돼요.

## 직접 빌드하기

필요한 것:
- Node.js 22.12 이상 (`.nvmrc` 참고)
- Rust (rustup)
- Windows: Microsoft C++ Build Tools, WebView2 (Windows 11에는 기본 포함)

```sh
npm install
npm run tauri dev      # 개발 모드로 실행
npm run tauri build    # 설치 파일 만들기
```

빌드 결과물은 `src-tauri/target/release/bundle/` 아래에 생겨요 (`nsis/…-setup.exe`, `msi/….msi`).

그 밖의 명령:

```sh
npm run check                  # 프론트엔드 타입 검사
cd src-tauri && cargo test     # 백엔드 테스트
```

개발 중에 실제 데이터 대신 다른 폴더를 쓰려면 `NORA_DATA_DIR` 환경 변수로 지정하세요.

## 라이선스

[MIT](LICENSE)
