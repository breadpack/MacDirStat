# 05. Settings (설정 화면) 구현 계획

## 1. 개요

MacDirStat에 사용자 설정 화면을 추가한다. WinDirStat의 Options 대화 상자를 macOS 환경에 맞게 재설계하며, 현재 하드코딩된 스캔 제외 패턴과 다크 테마 고정 문제를 해결한다. 설정은 Tauri의 tauri-plugin-store를 사용하여 JSON으로 영속 저장하고, Svelte writable store를 통해 프론트엔드 전체에서 반응형으로 참조한다.

## 2. 현재 상태

| 항목 | 현재 | 비고 |
|------|------|------|
| 테마 | 다크 테마 하드코딩 | `App.svelte` :global(body) background: #1a1a1a |
| 제외 패턴 | `scanner.rs`에 `SKIP_PATHS`, `SKIP_NAMES` 상수 고정 | 사용자가 변경 불가 |
| 심볼릭 링크 | 항상 스킵 (`ft.is_symlink()` -> continue) | scanner.rs |
| Treemap 스타일 | squarify 고정, padding 1px 고정 | treemapLayout.ts |
| 색상 팔레트 | `colorMap.ts`에 하드코딩 | 변경 불가 |
| 설정 저장 | 없음 | 플러그인 미설치 |

## 3. 설정 항목 정의

### 3.1 General (일반)

| 설정 키 | 타입 | 기본값 | 설명 |
|---------|------|--------|------|
| `general.followSymlinks` | boolean | false | 심볼릭 링크 따라가기 |
| `general.excludePaths` | string[] | 현재 SKIP_PATHS | 스캔 제외 절대 경로 |
| `general.excludeNames` | string[] | 현재 SKIP_NAMES | 스캔 제외 디렉토리 이름 |
| `general.excludePatterns` | string[] | [] | glob 패턴 기반 제외 |
| `general.maxChildrenPerDir` | number | 200 | 디렉토리당 최대 표시 자식 수 |

### 3.2 TreeView (디렉토리 목록)

| 설정 키 | 타입 | 기본값 | 설명 |
|---------|------|--------|------|
| `treeView.showColumns` | object | {name:true, size:true, percent:true, files:true} | 표시할 컬럼 |
| `treeView.sortBy` | string | "size" | 기본 정렬 기준 |
| `treeView.sortDesc` | boolean | true | 내림차순 정렬 |
| `treeView.showHiddenFiles` | boolean | false | 숨김 파일 표시 |

### 3.3 Treemap (시각화)

| 설정 키 | 타입 | 기본값 | 설명 |
|---------|------|--------|------|
| `treemap.style` | string | "squarify" | 레이아웃 알고리즘 |
| `treemap.padding` | number | 1 | 셀 간 패딩 (px) |
| `treemap.showLabels` | boolean | true | 셀 내 파일명 표시 |
| `treemap.colorPalette` | string | "default" | 색상 팔레트 |
| `treemap.cushionBrightness` | number | 1.0 | 쿠션 효과 밝기 |

### 3.4 Appearance (외관)

| 설정 키 | 타입 | 기본값 | 설명 |
|---------|------|--------|------|
| `appearance.theme` | string | "dark" | UI 테마 (dark/light/system) |
| `appearance.treePanelWidth` | number | 35 | 트리 패널 너비 비율 (%) |

## 4. 설정 저장 방식

### 선택: `tauri-plugin-store` v2

**근거**:
- Tauri 공식 플러그인, Tauri v2 호환
- JSON 파일로 자동 저장/로드 (`$APPCONFIG/settings.json`)
- 프론트엔드에서 `@tauri-apps/plugin-store` API로 직접 접근 가능
- Rust 측에서도 접근 가능

## 5. UI 설계

### 모달 다이얼로그 방식

```
+--------------------------------------------------+
|  Settings                                         |
|  [General] [TreeView] [Treemap] [Appearance]      |
|  ------------------------------------------------ |
|                                                    |
|  (선택된 탭의 설정 컨트롤)                          |
|                                                    |
|  ------------------------------------------------ |
|              [Reset to Defaults]  [Close]          |
+--------------------------------------------------+
```

### Toolbar 연동
- Toolbar.svelte에 기어 아이콘 버튼을 추가하여 설정 모달을 열기

## 6. 설정 스토어 설계 (Svelte)

### Rust 측 설정 전달 패턴

**방법 A (권장)**: `scan_directory` 커맨드에 설정 파라미터 추가
```rust
pub async fn scan_directory(
    path: String,
    exclude_paths: Vec<String>,
    exclude_names: Vec<String>,
    follow_symlinks: bool,
    on_progress: Channel<ScanProgress>,
    ...
) -> Result<Option<FileNode>, String>
```

## 7. 수정 대상 파일 목록

### 새 파일

| 파일 | 설명 |
|------|------|
| `src/lib/stores/settingsStore.ts` | 설정 스토어 (load/save/defaults) |
| `src/lib/types/settings.ts` | 설정 타입 정의 |
| `src/lib/components/SettingsDialog.svelte` | 설정 모달 본체 |
| `src/lib/components/settings/GeneralTab.svelte` | General 탭 |
| `src/lib/components/settings/TreeViewTab.svelte` | TreeView 탭 |
| `src/lib/components/settings/TreemapTab.svelte` | Treemap 탭 |
| `src/lib/components/settings/AppearanceTab.svelte` | Appearance 탭 |

### 수정 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/App.svelte` | SettingsDialog import, 테마 CSS 변수 적용 |
| `src/lib/components/Toolbar.svelte` | 설정 버튼 추가 |
| `src/lib/components/Treemap.svelte` | treemap 설정 반영 |
| `src/lib/utils/treemapLayout.ts` | 레이아웃 알고리즘 선택 파라미터화 |
| `src/lib/stores/scanStore.ts` | startScan()에 설정 전달 |
| `src/lib/api.ts` | scanDirectory() 시그니처에 설정 파라미터 추가 |
| `src-tauri/src/scanner.rs` | SKIP_PATHS/SKIP_NAMES 동적화, follow_symlinks 파라미터 |
| `src-tauri/src/commands.rs` | scan_directory 커맨드 파라미터 확장 |
| `src-tauri/Cargo.toml` | `tauri-plugin-store` 의존성 추가 |
| `src-tauri/src/lib.rs` | store 플러그인 등록 |
| `package.json` | `@tauri-apps/plugin-store` 의존성 추가 |

## 8. 구현 순서 및 예상 난이도

### Phase 1: 인프라 (난이도: 낮음, 1일)
1. `tauri-plugin-store` 설치
2. 설정 타입 정의
3. `settingsStore.ts` 구현
4. App.svelte 초기화 시 설정 로드

### Phase 2: 설정 UI (난이도: 중간, 2일)
1. SettingsDialog 모달 프레임 + 탭 네비게이션
2. 각 탭 컴포넌트 구현
3. Toolbar에 설정 버튼 추가

### Phase 3: 설정 적용 - 스캐너 (난이도: 중간, 1일)
1. scanner.rs SKIP_PATHS/SKIP_NAMES 파라미터화
2. commands.rs 시그니처 확장
3. api.ts, scanStore.ts에서 설정 전달

### Phase 4: 설정 적용 - 시각화 (난이도: 중간, 1일)
1. treemapLayout.ts 알고리즘 선택
2. colorMap.ts 팔레트 시스템
3. Treemap/TreeView에서 설정 store 구독

### Phase 5: 테마 시스템 (난이도: 높음, 별도 작업 권장)
- CSS 변수 기반 테마 시스템
- 라이트 모드 컬러셋 정의
- 규모가 크므로 별도 계획으로 분리 권장

## 9. 의존성 및 리스크

| 항목 | 영향 | 대응 |
|------|------|------|
| tauri-plugin-store v2 호환성 | 버전 불일치 시 빌드 실패 | Tauri v2 공식 문서 확인 |
| scanner.rs 시그니처 변경 | 기존 스캔 호출 모두 수정 | api.ts -> scanStore.ts 단일 호출 지점만 수정 |
| 설정 마이그레이션 | 스키마 변경 시 이전 설정과 충돌 | 버전 필드 추가 |
| 라이트 테마 | 전체 하드코딩 색상 수정 필요 | Phase 5로 분리 |
