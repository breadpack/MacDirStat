# 06. 트리뷰 추가 컬럼

## 1. 개요

현재 트리뷰는 Name, Size, Percent Bar 3개 컬럼만 표시한다. 디스크 분석 도구로서의 유용성을 높이기 위해 Files, Subdirs, Last Change, Attributes 컬럼을 추가하고, 컬럼 헤더 클릭 정렬, 너비 드래그 조절, 표시/숨기기 기능을 구현한다.

## 2. 현재 상태

### 관련 파일
| 파일 | 역할 |
|------|------|
| `src-tauri/src/models.rs` | `FileNode` 구조체 (name, path, size, is_dir, children, file_count, extension) |
| `src-tauri/src/scanner.rs` | 2-phase 스캔. 현재 `metadata().len()`만 읽음 |
| `src/lib/types.ts` | `FileNode`, `FlatTreeRow` TS 인터페이스 |
| `src/lib/components/TreeView.svelte` | 가상 스크롤 + flatten 로직. 정렬은 size 내림차순 고정 |
| `src/lib/components/TreeRow.svelte` | 단일 행 렌더링: toggle + icon + name + size + bar |
| `src/lib/utils/format.ts` | `formatSize`, `formatNumber` 유틸 |

### 현재 FileNode에 없는 데이터
- `modified_time` (최종 수정일) - Rust `metadata().modified()` 필요
- `subdirs_count` (서브트리 내 디렉토리 수) - 현재 `file_count`만 존재
- `attributes` (읽기전용, 숨김, 심볼릭링크 등) - 현재 수집 안 함

## 3. 구현 단계

### 단계 1: Rust 백엔드 - FileNode 메타데이터 확장

**models.rs 수정:**
```rust
pub struct FileNode {
    // 기존 필드 유지
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub file_count: u64,
    pub extension: Option<String>,
    // 신규 필드
    pub dir_count: u64,           // 서브트리 내 디렉토리 수
    pub modified: Option<u64>,    // Unix timestamp (초)
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_readonly: bool,
}
```

**scanner.rs 수정:**
- `metadata().modified()` -> `SystemTime::duration_since(UNIX_EPOCH)` -> u64 초
- macOS 숨김 파일: 이름이 `.`으로 시작
- 읽기전용: `metadata().permissions().readonly()`
- `dir_count`: 재귀적으로 자식 디렉토리 수 합산
- `modified`: 디렉토리의 경우 자식 중 최신 `modified` 값을 버블업

### 단계 2: 프론트엔드 타입 업데이트

```typescript
export interface FileNode {
  // 기존 + 신규
  dir_count: number;
  modified: number | null;
  is_symlink: boolean;
  is_hidden: boolean;
  is_readonly: boolean;
}
```

### 단계 3: 컬럼 설정 시스템

**신규 파일: `src/lib/stores/columnStore.ts`**

```typescript
export interface ColumnDef {
  id: string;
  label: string;
  width: number;
  minWidth: number;
  visible: boolean;
  sortable: boolean;
  align: 'left' | 'right' | 'center';
}

export type SortDirection = 'asc' | 'desc';
export interface SortState {
  columnId: string;
  direction: SortDirection;
}
```

### 단계 4: 컬럼 헤더 컴포넌트

**신규: `src/lib/components/TreeColumnHeader.svelte`**

- 컬럼 헤더 행 (높이 28px, 트리뷰 상단 고정)
- 클릭 시 정렬 toggle
- 컬럼 사이 드래그 핸들로 너비 조절
- 우클릭 시 컬럼 표시/숨기기 컨텍스트 메뉴

### 단계 5: TreeRow 리팩토링

현재 flex 레이아웃을 컬럼 기반으로 변경:
- Name: 기존 toggle + icon + name
- Size: `formatSize(displaySize)`
- Percent: 바 차트
- Files: `formatNumber(node.file_count)` - 디렉토리만
- Subdirs: `formatNumber(node.dir_count)` - 디렉토리만
- Modified: `formatDate(node.modified)` - 신규 포맷 함수
- Attributes: 아이콘 조합

### 단계 6: TreeView 정렬 로직 수정

`sortState`를 반영하도록 flatten 내 정렬 변경:

```typescript
function compareNodes(a: FileNode, b: FileNode, sort: SortState): number {
  const getValue = (node: FileNode) => {
    switch (sort.columnId) {
      case 'name': return node.name.toLowerCase();
      case 'size': return getEffectiveSize(node, sizes);
      case 'files': return node.file_count;
      case 'subdirs': return node.dir_count;
      case 'modified': return node.modified ?? 0;
      default: return getEffectiveSize(node, sizes);
    }
  };
}
```

### 단계 7: format 유틸 확장

```typescript
export function formatDate(timestamp: number | null): string {
  if (timestamp === null || timestamp === 0) return '-';
  const d = new Date(timestamp * 1000);
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}
```

## 4. 수정 대상 파일

| 파일 | 작업 |
|------|------|
| `src-tauri/src/models.rs` | FileNode에 신규 필드 추가 |
| `src-tauri/src/scanner.rs` | 메타데이터 추가 수집, dir_count/modified 합산 |
| `src/lib/types.ts` | FileNode 인터페이스에 신규 필드 추가 |
| `src/lib/stores/columnStore.ts` | **신규** - 컬럼 정의, 정렬 상태, persist |
| `src/lib/components/TreeColumnHeader.svelte` | **신규** - 컬럼 헤더 |
| `src/lib/components/TreeView.svelte` | 컬럼 헤더 통합, flatten 정렬 변경 |
| `src/lib/components/TreeRow.svelte` | 컬럼 기반 레이아웃, 추가 컬럼 렌더링 |
| `src/lib/utils/format.ts` | `formatDate` 추가 |

## 5. 기술적 세부사항

### 성능 고려
- `modified()` 수집: 이미 `metadata()` 호출하므로 추가 I/O 없음
- `dir_count`: assemble_tree 재귀 시 O(N) 한 번 순회
- 컬럼 리사이즈: CSS width 변경은 가상 스크롤과 충돌 없음

### 하위 호환성
- 신규 필드를 `#[serde(default)]` 사용으로 역직렬화 안전성 확보

### 컬럼 너비 드래그 구현
- mousedown -> document에 mousemove/mouseup 리스너
- `requestAnimationFrame`으로 throttle
- 드래그 중 overlay div로 텍스트 선택 방지

## 6. 예상 난이도 및 의존성

| 항목 | 난이도 | 예상 시간 |
|------|--------|-----------|
| Rust FileNode 확장 + scanner 수정 | 중 | 2-3h |
| columnStore + 타입 업데이트 | 하 | 1h |
| TreeColumnHeader 컴포넌트 | 중 | 2-3h |
| TreeRow 컬럼 리팩토링 | 중 | 2h |
| 정렬 로직 변경 | 하 | 1h |
| 컬럼 너비 드래그 | 중-상 | 2-3h |
| 컬럼 표시/숨기기 | 하-중 | 1-2h |
| **합계** | | **11-15h** |

### 의존성
- Rust 백엔드 변경이 선행되어야 프론트엔드에서 데이터 사용 가능
- columnStore는 TreeColumnHeader와 TreeRow 모두에서 사용 -> 먼저 구현
