# 10. 사용자 정의 클린업 (User-Defined Cleanups)

## 1. 개요

WinDirStat의 핵심 기능인 사용자 정의 클린업을 macOS 버전으로 구현한다. 사용자가 최대 10개의 커스텀 쉘 명령어를 등록하고, 선택된 파일/폴더에 대해 실행할 수 있는 기능이다.

### macOS 조정 사항
- `%sp`, `%sn`(Windows 짧은 경로)은 불필요, 대신 `%e`(확장자), `%d`(부모 디렉토리) 추가
- 단축키: Cmd+Shift+0~9
- 콘솔 표시 -> Terminal.app에서 실행 옵션
- 실행 후 새로고침은 08-refresh 기능에 의존

## 2. 현재 상태

| 파일 | 상태 |
|------|------|
| `src-tauri/src/commands.rs` | 쉘 커맨드 실행 기능 없음 |
| `src-tauri/src/platform/macos.rs` | `open_in_finder`, `open_full_disk_access_settings`만 존재 |
| `src/lib/components/ContextMenu.svelte` | 정적 메뉴만 존재, 동적 메뉴 없음 |

## 3. 구현 단계

### 3.1 데이터 모델

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupAction {
    pub id: u8,               // 0-9
    pub name: String,
    pub command: String,      // 쉘 명령어 템플릿
    pub enabled: bool,
    pub target: CleanupTarget, // Files, Dirs, Both
    pub confirm: bool,
    pub run_in_terminal: bool,
    pub refresh_after: bool,
    pub shortcut_key: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupTarget {
    Files,
    Dirs,
    Both,
}
```

### 변수 치환 규칙

| 변수 | 설명 | 예시 |
|------|------|------|
| `%p` | 전체 경로 | `/Users/me/Documents/file.txt` |
| `%n` | 파일/폴더명 | `file.txt` |
| `%d` | 부모 디렉토리 경로 | `/Users/me/Documents` |
| `%e` | 확장자 | `txt` |

### 3.2 설정 저장/로드

- 설정 파일: Tauri의 `app.path().app_config_dir()` + `cleanups.json`
- 신규 파일 `src-tauri/src/config.rs`에서 로드/저장 관리

### 3.3 쉘 명령어 실행 엔진

```rust
#[tauri::command]
pub fn execute_cleanup(
    action_id: u8,
    path: String,
    name: String,
) -> Result<String, String> {
    let actions = config::load_cleanups();
    let action = actions.iter().find(|a| a.id == action_id)
        .ok_or("Cleanup action not found")?;

    let command = substitute_variables(&action.command, &path, &name);

    if action.run_in_terminal {
        execute_in_terminal(&command)
    } else {
        execute_background(&command)
    }
}

fn substitute_variables(template: &str, path: &str, name: &str) -> String {
    let parent = Path::new(path).parent()
        .map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let ext = Path::new(path).extension()
        .map(|e| e.to_string_lossy().to_string()).unwrap_or_default();

    template
        .replace("%p", &shell_escape(path))
        .replace("%n", &shell_escape(name))
        .replace("%d", &shell_escape(&parent))
        .replace("%e", &shell_escape(&ext))
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
```

### 3.4 설정 UI

새 컴포넌트 `CleanupSettings.svelte`:
- 10개 슬롯의 클린업 액션 편집
- 각 슬롯: 이름, 명령어, 대상 타입, 체크박스 옵션, 활성화 토글
- 변수 참조 도움말 영역
- 저장/취소 버튼

### 3.5 ContextMenu 통합

- cleanups store에서 캐시된 목록 사용
- `isDir`에 따라 해당하는 cleanup action만 표시
- 단축키 표시: `Cmd+Shift+N`

### 3.6 기본 제공 프리셋 (첫 실행 시)

1. **Compress (zip)**: `zip -r %p.zip %p`
2. **Calculate checksum**: `shasum -a 256 %p`
3. **List contents**: `ls -lahR %p` (Terminal)
4. **Disk usage**: `du -sh %p` (Terminal)

## 4. 수정 대상 파일

| 파일 | 변경 내용 |
|------|-----------|
| `src-tauri/src/models.rs` | `CleanupAction`, `CleanupTarget` 추가 |
| `src-tauri/src/config.rs` | **신규** - 설정 파일 로드/저장 |
| `src-tauri/src/commands.rs` | 3개 커맨드 추가 |
| `src-tauri/src/lib.rs` | 새 모듈 및 커맨드 등록 |
| `src-tauri/Cargo.toml` | `dirs` 크레이트 추가 |
| `src/lib/types.ts` | `CleanupAction` 인터페이스 추가 |
| `src/lib/api.ts` | cleanup 관련 API 래퍼 3개 추가 |
| `src/lib/stores/cleanupStore.ts` | **신규** - cleanup actions 캐시 store |
| `src/lib/components/CleanupSettings.svelte` | **신규** - 설정 모달 |
| `src/lib/components/ContextMenu.svelte` | 동적 cleanup 메뉴 항목 |
| `src/App.svelte` | Cmd+Shift+0~9 키보드 단축키 |

## 5. 보안 고려사항

- **쉘 인젝션 방지**: 변수 치환 시 반드시 쉘 이스케이프 적용
- **위험 명령 경고**: `rm -rf`, `sudo` 등의 패턴 감지 시 추가 경고
- **실행 권한**: macOS sandbox 밖에서 실행되므로 주의 필요

## 6. 예상 난이도 및 의존성

| 항목 | 난이도 | 예상 시간 |
|------|--------|-----------|
| 데이터 모델 | 낮음 | 1h |
| 설정 저장/로드 | 낮음 | 2h |
| 쉘 명령어 실행 엔진 | 중간 | 3-4h |
| 설정 UI | 중간~높음 | 6-8h |
| ContextMenu 통합 | 중간 | 2-3h |
| 키보드 단축키 | 낮음 | 1-2h |
| **총 예상 시간** | | **16-21h** |

### 의존성 그래프
```
데이터 모델
+-- 설정 저장/로드
|   +-- 설정 UI
+-- 쉘 명령어 실행 엔진
|   +-- ContextMenu 통합
|   +-- 키보드 단축키
+-- 실행 후 새로고침 <-- 08-refresh (외부 의존)
```
