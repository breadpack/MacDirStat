# 09. 파일 작업 확장 (File Operations)

## 1. 개요

현재 구현된 파일 작업은 두 가지뿐이다:
- `move_to_trash`: 휴지통으로 이동 (trash crate 사용)
- `open_in_finder`: Finder에서 표시 (`open -R` 명령)

추가 구현 대상:
1. **영구 삭제** (Permanent Delete) - 복구 불가 삭제 + 경고 대화상자
2. **파일 속성 보기** (Get Info) - macOS Get Info 연동
3. **터미널 열기** (Open in Terminal) - Terminal.app에서 해당 디렉토리 열기
4. **파일 직접 열기** (Open) - 기본 앱으로 파일 열기

## 2. 현재 상태

| 파일 | 상태 |
|------|------|
| `src-tauri/src/commands.rs` | `open_in_finder`, `move_to_trash` 구현됨 |
| `src-tauri/src/platform/macos.rs` | `open_in_finder()` 구현 (`open -R` 사용) |
| `src/lib/api.ts` | `openInFinder()`, `moveToTrash()` 래퍼 존재 |
| `src/lib/components/ContextMenu.svelte` | Reveal in Finder, Copy Path, Move to Trash 메뉴 |
| `src-tauri/Cargo.toml` | `trash = "5"` 의존성 있음 |

## 3. 구현 단계

### 3.1 영구 삭제 (난이도: 낮음)

#### Rust 백엔드
```rust
#[tauri::command]
pub fn permanent_delete(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    }
}
```

#### 프론트엔드
- `api.ts`에 `permanentDelete(path)` 추가
- ContextMenu에 "Delete Permanently" 메뉴 (danger 스타일)
- **이중 확인 대화상자** 필수 (Tauri dialog `confirm()` 사용)
- 삭제 후 기존 `removeNode` 함수로 트리 갱신

#### 안전장치
- 시스템 경로 삭제 방지: `PROTECTED_ROOTS = ["/", "/System", "/usr", "/bin", "/sbin", "/Users"]`
- 디렉토리 삭제 시 하위 파일 수와 총 크기 표시

### 3.2 파일 속성 보기 (난이도: 낮음~중간)

#### 접근법 A: macOS Get Info 창 열기 (권장, 간단)
```rust
pub fn show_get_info(path: &str) -> Result<(), String> {
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&format!(
            r#"tell application "Finder" to open information window of (POSIX file "{}" as alias)"#,
            path
        ))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

#### 접근법 B: 커스텀 속성 패널 (고급, 추후)
- Rust에서 파일 메타데이터 수집
- 프론트엔드에 모달로 표시

### 3.3 터미널 열기 (난이도: 낮음)

```rust
pub fn open_in_terminal(path: &str) -> Result<(), String> {
    let dir = if std::path::Path::new(path).is_dir() {
        path.to_string()
    } else {
        std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    };
    std::process::Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

### 3.4 파일 직접 열기 (난이도: 낮음)

```rust
pub fn open_file(path: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

## 4. 수정 대상 파일

| 파일 | 변경 내용 |
|------|-----------|
| `src-tauri/src/commands.rs` | 4개 신규 커맨드 추가 |
| `src-tauri/src/platform/macos.rs` | 3개 함수 구현 |
| `src-tauri/src/lib.rs` | 새 커맨드들을 invoke_handler에 등록 |
| `src/lib/api.ts` | 4개 API 래퍼 함수 추가 |
| `src/lib/components/ContextMenu.svelte` | 새 메뉴 항목들 추가 |

## 5. ContextMenu 최종 메뉴 구조 (제안)

```
Open                        (파일/폴더 공통)
Reveal in Finder
Open in Terminal            (폴더 전용 또는 파일의 부모 폴더)
---
Copy Path
Get Info
---
Refresh                     (폴더 전용, 08-refresh에서 구현)
---
Move to Trash               (danger)
Delete Permanently          (danger, 이중 확인)
```

## 6. 예상 난이도 및 의존성

| 항목 | 난이도 | 예상 시간 |
|------|--------|-----------|
| 영구 삭제 | 낮음 | 1-2h |
| 파일 속성 (Get Info, 접근법 A) | 낮음 | 1h |
| 터미널 열기 | 낮음 | 30min |
| 파일 직접 열기 | 낮음 | 30min |

모든 항목이 독립적이므로 병렬 구현 가능.
구현 순서: 파일 직접 열기 -> 터미널 열기 -> 영구 삭제 -> 파일 속성
