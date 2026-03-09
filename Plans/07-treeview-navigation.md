# 07. 트리뷰 네비게이션

## 1. 개요

현재 트리뷰는 마우스 클릭과 더블클릭만 지원한다. 키보드 네비게이션(화살표, Enter), 부모 선택(Select Parent), 이전 자식 재선택(Re-select Child) 기능을 추가하여 효율적인 탐색을 가능하게 한다.

## 2. 현재 상태

### 관련 파일
| 파일 | 역할 |
|------|------|
| `src/lib/components/TreeView.svelte` | 가상 스크롤, expandedPaths 관리, selectedPath 기반 자동 스크롤 |
| `src/lib/components/TreeRow.svelte` | click -> selectedPath 설정, dblclick -> toggleExpand |
| `src/lib/stores/selectionStore.ts` | `selectedPath`, `hoveredPath`, `zoomRoot` writable stores |
| `src/lib/components/Toolbar.svelte` | 네비게이션 버튼 없음 |

### 현재 인터랙션 모델
- **클릭**: 행 선택
- **더블클릭**: 디렉토리 접기/펼치기
- **자동 스크롤**: selectedPath 변경 시 조상 자동 expand + 뷰포트로 스크롤
- **키보드**: 미지원
- **히스토리**: 미지원

## 3. 구현 단계

### 단계 1: 키보드 네비게이션 - TreeView 키 이벤트 처리

TreeView.svelte에 `tabindex="0"` 추가, `onkeydown` 핸들러 등록:

| 키 | 동작 |
|---|---|
| ArrowUp | 이전 행 선택 |
| ArrowDown | 다음 행 선택 |
| ArrowLeft | 접기 / 부모로 이동 |
| ArrowRight | 펼치기 / 첫 자식으로 이동 |
| Enter | 접기/펼치기 토글 |
| Home | 첫 번째 행 선택 |
| End | 마지막 행 선택 |
| Alt+ArrowUp | Select Parent (항상 부모로) |
| Alt+ArrowDown | Re-select Child |

**ArrowLeft 동작:**
1. 현재 노드가 펼쳐진 디렉토리면 -> 접기
2. 그렇지 않으면 -> 부모 디렉토리로 이동

**ArrowRight 동작:**
1. 접혀진 디렉토리면 -> 펼치기
2. 이미 펼쳐져 있으면 -> 첫 번째 자식으로 이동

### 단계 2: Select Parent 기능

```typescript
function getParentPath(path: string): string | null {
  const lastSlash = path.lastIndexOf('/');
  if (lastSlash <= 0) return null;
  return path.substring(0, lastSlash);
}

function selectParent() {
  const sp = $selectedPath;
  if (!sp) return;
  const parentPath = getParentPath(sp);
  if (!parentPath) return;
  // 루트 경로보다 상위로 올라가지 않도록 체크
  selectedPath.set(parentPath);
}
```

### 단계 3: Re-select Child (이전 자식 재선택)

**신규 파일: `src/lib/stores/navigationStore.ts`**

```typescript
// 각 디렉토리 경로 -> 마지막으로 방문한 자식 경로
const childHistory = writable<Map<string, string>>(new Map());

export function recordChildVisit(parentPath: string, childPath: string) { ... }
export function getLastChild(parentPath: string): string | null { ... }
export function clearHistory() { ... }
```

`$selectedPath` 변경을 감지하여 부모-자식 관계 기록:

```typescript
$effect(() => {
  const sp = $selectedPath;
  if (!sp) return;
  const parentPath = getParentPath(sp);
  if (parentPath) recordChildVisit(parentPath, sp);
});
```

### 단계 4: 툴바 네비게이션 버튼

Toolbar.svelte에 Parent/Child 네비게이션 버튼 추가.

### 단계 5: 포커스 관리

- 트리뷰 컨테이너에만 포커스 (개별 행에는 포커스하지 않음 - 가상 스크롤 때문)
- 선택 상태는 `selectedPath` store로 관리
- 키 이벤트는 트리뷰가 포커스된 상태에서만 동작

## 4. 수정 대상 파일

| 파일 | 작업 |
|------|------|
| `src/lib/stores/navigationStore.ts` | **신규** - 자식 히스토리 맵 |
| `src/lib/components/TreeView.svelte` | keydown 핸들러, tabindex, selectParent/reselectChild |
| `src/lib/components/TreeRow.svelte` | 포커스 관련 aria 속성 추가 |
| `src/lib/components/Toolbar.svelte` | Parent/Child 네비게이션 버튼 추가 |

## 5. 기술적 세부사항

### flatRows 인덱스 최적화
- `findIndex`는 O(N). 최적화: `selectedIndex`를 `$derived`로 캐시

### 부모 경로 계산
- FileNode에 parent 참조 없음. 경로 문자열의 `/` 기준 분할로 계산
- `$tree.path`와 비교하여 루트 이상으로 올라가지 않도록 보장

### 접근성 (a11y)
- 트리뷰에 `role="tree"`, 각 행에 `role="treeitem"`
- `aria-expanded`, `aria-level`, `aria-selected` 속성

## 6. 예상 난이도 및 의존성

| 항목 | 난이도 | 예상 시간 |
|------|--------|-----------|
| 키보드 네비게이션 | 하-중 | 2-3h |
| Select Parent | 하 | 0.5h |
| Re-select Child (navigationStore) | 중 | 1-2h |
| 툴바 버튼 | 하 | 0.5h |
| a11y 속성 | 하 | 1h |
| **합계** | | **5-7h** |

### 의존성
- 독립적으로 구현 가능 (06-treeview-columns와 의존성 없음)
- 새 스캔 시작 시 `clearHistory()` 호출하여 초기화
