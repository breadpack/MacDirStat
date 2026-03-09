# 11. 키보드 단축키 (Keyboard Shortcuts)

## 1. 개요

WinDirStat 스타일의 키보드 단축키를 macOS 관습에 맞게 매핑한다. 앱 전역 단축키와 TreeView 내부 네비게이션 키를 구분하여 구현한다.

## 2. 현재 상태

- `src/App.svelte` - keydown 이벤트 바인딩 없음
- `src/lib/components/Toolbar.svelte` - Open/Stop 버튼만 존재, 단축키 없음
- `src/lib/components/TreeView.svelte` - 키보드 네비게이션 없음
- `src/lib/components/ContextMenu.svelte` - Copy Path, Reveal in Finder, Move to Trash 이미 구현됨
- 포커스 관리 시스템 없음

## 3. 구현 단계

### 단계 1: 단축키 매핑 정의

새 파일 `src/lib/utils/shortcuts.ts`:

```typescript
export interface ShortcutDef {
  key: string;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  action: string;
  label: string;
}

export const SHORTCUTS: ShortcutDef[] = [
  { key: 'o', metaKey: true, action: 'open-folder', label: 'Open Folder' },
  { key: 'r', metaKey: true, action: 'refresh', label: 'Refresh' },
  { key: 'c', metaKey: true, action: 'copy-path', label: 'Copy Path' },
  { key: 'i', metaKey: true, action: 'show-info', label: 'Show Info' },
  { key: 'Backspace', action: 'move-to-trash', label: 'Move to Trash' },
  { key: 'F8', action: 'toggle-extensions', label: 'Toggle Extension List' },
  { key: 'F9', action: 'toggle-treemap', label: 'Toggle Treemap' },
  { key: 'ArrowUp', action: 'tree-up', label: 'Move Up' },
  { key: 'ArrowDown', action: 'tree-down', label: 'Move Down' },
  { key: 'ArrowLeft', action: 'tree-collapse', label: 'Collapse' },
  { key: 'ArrowRight', action: 'tree-expand', label: 'Expand' },
  { key: 'Enter', action: 'tree-zoom', label: 'Zoom Into' },
  { key: 'Escape', action: 'tree-zoom-out', label: 'Zoom Out / Close' },
];
```

### 단계 2: App.svelte에 전역 keydown 핸들러

- `<svelte:window onkeydown={handleKeydown} />`
- SHORTCUTS 배열 순회하며 매칭
- ContextMenu 열려 있으면 Escape로 닫기

### 단계 3: TreeView 키보드 네비게이션

TreeView.svelte:
- `tabindex="0"` 추가
- `focusedIndex` 상태 추가
- 방향키: ArrowUp/Down (행 이동), ArrowLeft/Right (접기/펼치기 + 부모/자식)
- Enter: 접기/펼치기 토글
- Home/End: 첫/마지막 행 이동
- 포커스된 행을 뷰포트 내 자동 스크롤

### 단계 4: 전역 액션 함수 연결

기존 API 함수를 단축키 액션에 연결:
- `open-folder`: Toolbar의 `openFolder()` 로직 재사용
- `copy-path`: ContextMenu의 `handleCopyPath()` 로직 재사용
- `move-to-trash`: ContextMenu의 `handleMoveToTrash()` 로직 재사용
- `refresh`: startScan을 현재 tree root path로 재호출

### 단계 5: 패널 토글 (F8/F9)

- App.svelte에 `showExtensions`, `showTreemap` 상태 추가
- F8 -> showExtensions 토글
- F9 -> showTreemap 토글

## 4. 수정 대상 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/lib/utils/shortcuts.ts` | **신규** - 단축키 정의 및 매칭 유틸리티 |
| `src/App.svelte` | 전역 keydown 핸들러, 패널 토글 상태 |
| `src/lib/components/TreeView.svelte` | 방향키 네비게이션, tabindex |
| `src/lib/components/TreeRow.svelte` | 포커스 상태 시각적 표시 |
| `src/lib/utils/actions.ts` | **신규** - 공유 액션 함수 |

## 5. 기술적 세부사항

### globalShortcut vs keydown 이벤트

**결론: 일반 `keydown` 이벤트 사용**

이유:
- `@tauri-apps/plugin-global-shortcut`은 앱이 포커스를 잃어도 동작 - 부적절
- 앱 내부에서만 동작하는 단축키는 웹 표준 `keydown`으로 충분
- Tauri 플러그인 추가 의존성 불필요
- Cmd+O, Cmd+R 등은 `e.preventDefault()`로 브라우저 기본 동작 차단 필요

### 이벤트 우선순위
1. ContextMenu 열린 상태: Escape -> 닫기
2. TreeView에 포커스: 방향키 -> TreeView 네비게이션
3. 전역: Cmd+O, Cmd+R, F8, F9 등 -> 앱 전역 액션

### 접근성 고려
- TreeRow에 `role="treeitem"`, `aria-expanded`, `aria-selected`
- TreeView에 `role="tree"`

## 6. 예상 난이도 및 의존성

- **난이도**: 중간
- **예상 시간**: 3~4시간
- **의존성**: 없음 (독립 구현 가능)
- **주의사항**:
  - macOS에서 Cmd+R은 Tauri webview 새로고침이므로 `preventDefault()` 필수
  - Delete/Backspace 처리 시 텍스트 입력 필드와 충돌하지 않도록 `activeElement` 체크
