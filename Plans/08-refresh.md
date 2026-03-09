# 08. 새로고침 기능 (Refresh)

## 1. 개요

현재 Toolbar에 `refresh()` 함수가 빈 상태로 TODO로 남아 있다 (Toolbar.svelte:14-16).
전체 새로고침과 선택 항목 부분 새로고침 두 가지 기능을 구현한다.

- **전체 새로고침 (F5)**: 현재 트리 루트 경로를 다시 전체 스캔
- **선택 항목 새로고침 (Refresh Selected)**: 선택된 디렉토리만 부분 재스캔하여 기존 트리에 병합

## 2. 현재 상태

| 파일 | 상태 |
|------|------|
| `src/lib/components/Toolbar.svelte` | `refresh()` 함수가 빈 구현 (14행) |
| `src/lib/stores/scanStore.ts` | `startScan(path)` 함수 존재, 전체 스캔만 지원 |
| `src/lib/stores/selectionStore.ts` | `selectedPath`, `zoomRoot` store 존재 |
| `src-tauri/src/commands.rs` | `scan_directory` 커맨드만 존재 |
| `src-tauri/src/scanner.rs` | 전체 스캔 로직만 존재 |
| `src/lib/components/ContextMenu.svelte` | 새로고침 메뉴 항목 없음 |

## 3. 구현 단계

### 3.1 전체 새로고침 (난이도: 낮음)

1. **Toolbar.svelte - `refresh()` 구현**
   - `get(tree)?.path`에서 현재 루트 경로 읽기
   - 경로가 있으면 `startScan(path)` 호출

2. **Toolbar UI 수정**
   - Refresh 버튼 추가 (스캔 중이 아니고 트리가 있을 때만 활성화)
   - 단축키: F5 또는 Cmd+R

### 3.2 선택 항목 새로고침 (난이도: 중간)

1. **Rust 백엔드 - `scan_subdirectory` 커맨드 추가**
   - 기존 `scanner::scan_directory` 로직 재활용, 지정된 서브 경로만 스캔
   - Channel API로 진행률 스트리밍
   - 반환: 해당 서브트리의 `FileNode`

2. **프론트엔드 API 추가**
   - `src/lib/api.ts`에 `scanSubdirectory(path, channels)` 함수

3. **scanStore에 부분 새로고침 함수 추가**
   - `refreshSubtree(path: string)` 함수
   - 기존 트리 유지, 해당 서브트리 노드만 교체
   - `tree.set(...)` 호출로 treemap 자동 재계산

4. **트리 병합 유틸리티**

```typescript
function replaceSubtree(node: FileNode, targetPath: string, newSubtree: FileNode): FileNode {
  if (node.path === targetPath) return newSubtree;
  if (!targetPath.startsWith(node.path)) return node;

  const newChildren = node.children.map(child =>
    replaceSubtree(child, targetPath, newSubtree)
  );
  const newSize = newChildren.reduce((s, c) => s + c.size, 0);
  const newFileCount = newChildren.reduce((s, c) => s + c.file_count, 0);

  return { ...node, children: newChildren, size: newSize, file_count: newFileCount };
}
```

5. **ContextMenu에 "Refresh" 메뉴 추가** (디렉토리인 경우에만 표시)

## 4. 수정 대상 파일

| 파일 | 변경 내용 |
|------|-----------|
| `src/lib/components/Toolbar.svelte` | refresh() 구현, Refresh 버튼 추가 |
| `src/lib/stores/scanStore.ts` | `refreshSubtree(path)` 함수 추가 |
| `src/lib/api.ts` | `scanSubdirectory()` 함수 추가 |
| `src/lib/utils/treeUtils.ts` | **신규** - `replaceSubtree()` 유틸리티 |
| `src/lib/components/ContextMenu.svelte` | "Refresh" 메뉴 항목 추가 |
| `src-tauri/src/commands.rs` | `scan_subdirectory` 커맨드 추가 |
| `src-tauri/src/lib.rs` | 새 커맨드 등록 |

## 5. 기술적 세부사항

### 부분 스캔 시 고려사항
- `scan_directory` 함수는 이미 임의의 경로를 받으므로 서브디렉토리 스캔에 그대로 사용 가능
- SKIP_PATHS 필터링이 절대 경로 기반이므로 서브디렉토리 스캔 시에도 정상 동작
- cancel_token은 ScanState에서 공유, 부분 스캔 중에도 취소 가능
- `partialScanning` 별도 store 사용 권장 (전체 UI를 잠그지 않기 위함)

### 크기 재계산
- 서브트리 교체 후 루트까지 모든 조상의 size, file_count 재계산 필요
- `replaceSubtree` 함수가 재귀적으로 처리

## 6. 예상 난이도 및 의존성

| 항목 | 난이도 | 의존성 |
|------|--------|--------|
| 전체 새로고침 | **낮음** (1-2시간) | 없음 |
| 부분 새로고침 | **중간** (4-6시간) | 전체 새로고침 |
| 트리 병합 로직 | **중간** | 없음 |

구현 순서: 전체 새로고침 -> 트리 병합 유틸리티 -> 부분 새로고침
