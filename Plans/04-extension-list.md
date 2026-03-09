# 04. 확장자 통계 패널 (Extension List Panel)

## 1. 개요

WinDirStat의 Extension List 기능을 MacDirStat에 구현한다. 스캔된 파일 트리에서 확장자별로 바이트 수/파일 수를 집계하여 별도 패널에 표시하고, 트리맵의 색상 범례(legend) 역할을 겸한다. 상위 N개 확장자에 동적으로 색상을 할당하여 트리맵과 연동하며, 확장자 클릭 시 해당 타입 파일을 트리맵에서 하이라이트한다.

## 2. 현재 상태

### 관련 파일 및 현황

| 파일 | 역할 | 현재 상태 |
|------|------|-----------|
| `src/lib/utils/colorMap.ts` | 확장자 -> 색상 매핑 | 37개 타입 고정 색상. `getColor(extension, isDir)` 함수 export |
| `src/lib/types.ts` | 타입 정의 | `FileNode.extension: string | null` 필드 이미 존재 |
| `src-tauri/src/models.rs` | Rust FileNode 모델 | `extension: Option<String>` 필드 존재 |
| `src-tauri/src/scanner.rs` | 스캔 로직 | Phase 2에서 파일별 extension 추출 완료. 별도 확장자 집계 없음 |
| `src/lib/stores/scanStore.ts` | 스캔 상태 관리 | `tree` writable store. 확장자 통계 store 없음 |
| `src/App.svelte` | 메인 레이아웃 | 2-pane: tree-panel(35%) | treemap-panel(flex:1). 확장자 패널 없음 |

## 3. 데이터 수집 방식

### 결정: 프론트엔드에서 계산 (Rust 변경 불필요)

**근거:**
- `FileNode`에 이미 `extension` 필드가 존재
- 트리가 프론트엔드 store에 이미 로드
- 트리 순회 한 번으로 집계 가능 (수 ms)
- `zoomRoot` 변경 시 현재 보이는 서브트리 기준으로 실시간 재집계가 자연스러움

### 집계 알고리즘

```typescript
export interface ExtensionStat {
  extension: string;      // 소문자, 확장자 없는 파일은 "(no ext)"
  totalBytes: number;
  fileCount: number;
  percentage: number;     // totalBytes 기준 퍼센트
  color: string;          // 동적 할당된 색상
}

export function computeExtensionStats(root: FileNode): ExtensionStat[] {
  const map = new Map<string, { bytes: number; count: number }>();

  function walk(node: FileNode) {
    if (!node.is_dir) {
      const ext = node.extension ?? "(no ext)";
      const entry = map.get(ext);
      if (entry) {
        entry.bytes += node.size;
        entry.count += 1;
      } else {
        map.set(ext, { bytes: node.size, count: 1 });
      }
    }
    for (const child of node.children) walk(child);
  }

  walk(root);
  // Bytes 내림차순 정렬 후 percentage 계산, 색상 할당
}
```

## 4. UI 컴포넌트 설계

### ExtensionList.svelte (신규)

- 헤더 행: Extension | Color | Bytes | %Bytes | Files (클릭으로 정렬 전환)
- 가상 스크롤 행 (확장자가 수백 개 될 수 있으므로)
- 각 행: `.ext-name` | 색상 스와치(12x12 div) | formatSize(bytes) | percentage bar + text | formatNumber(count)
- 기본 정렬: Bytes 내림차순
- 행 높이: 24px (TreeView와 동일)

**상호작용:**
- 행 클릭: `highlightedExtension` store에 해당 확장자 설정 (토글)
- 행 호버: 해당 확장자의 트리맵 rect들을 약간 밝게 처리
- 컬럼 헤더 클릭: 해당 컬럼으로 정렬 전환 (asc/desc)

## 5. 동적 색상 할당 알고리즘

### 현재 문제
고정 색상은 카테고리별로 비슷한 색이라 트리맵에서 확장자 구분이 어려움.

### 알고리즘

```typescript
// 상위 12개에 할당할 고채도 팔레트 (HSL 기반, 균등 분포)
const DYNAMIC_PALETTE = [
  "#4A90D9", "#D94A4A", "#4AD94A", "#D9944A",
  "#944AD9", "#4AD9D9", "#D9D94A", "#D94A94",
  "#4A6BD9", "#D96B4A", "#6BD94A", "#D94AD9",
];

const GRAY_COLOR = "#666666";

export function buildColorMap(stats: ExtensionStat[]): Map<string, string> {
  const map = new Map<string, string>();
  for (let i = 0; i < stats.length; i++) {
    map.set(stats[i].extension, i < DYNAMIC_PALETTE.length ? DYNAMIC_PALETTE[i] : GRAY_COLOR);
  }
  return map;
}
```

## 6. 트리맵 연동 (확장자 선택 시 하이라이트)

### Store 추가

```typescript
export const highlightedExtension = writable<string | null>(null);
```

### Treemap.svelte 수정

확장자 하이라이트 모드일 때:
- 선택되지 않은 확장자는 어둡게 (alpha 0.2)
- 선택된 확장자는 밝은 테두리 추가

## 7. 레이아웃 통합

### 3-Pane 레이아웃

```
+---------+-----------+---------------------+
| TreeView| ExtList   | Treemap             |
| (35%)   | (200px)   | (flex:1)            |
|         | (토글)    |                     |
+---------+-----------+---------------------+
```

## 8. 수정 대상 파일 목록

### 신규 파일

| 파일 | 역할 |
|------|------|
| `src/lib/utils/extensionStats.ts` | 트리 순회 -> 확장자별 집계 로직 |
| `src/lib/components/ExtensionList.svelte` | 확장자 통계 테이블 UI |
| `src/lib/stores/extensionStore.ts` | `extensionStats`, `extensionColorMap`, `highlightedExtension` store |

### 수정 파일

| 파일 | 변경 내용 |
|------|-----------|
| `src/App.svelte` | 3-pane 레이아웃, ExtensionList import, 토글 상태, F8 단축키 |
| `src/lib/utils/colorMap.ts` | `getColor()`를 동적 색상맵 참조로 리팩토링. `buildColorMap()` 추가 |
| `src/lib/components/Treemap.svelte` | `highlightedExtension` store 구독, 하이라이트 dim 처리 |
| `src/lib/components/Toolbar.svelte` | 확장자 패널 토글 버튼 추가 |
| `src/lib/stores/selectionStore.ts` | `highlightedExtension` writable 추가 |

## 9. 예상 난이도 및 의존성

### 구현 순서 (의존성 기반)

```
1. extensionStats.ts (독립)
2. colorMap.ts 리팩토링 (extensionStats에 의존)
3. extensionStore.ts (extensionStats + colorMap에 의존)
4. ExtensionList.svelte (extensionStore에 의존)
5. App.svelte 레이아웃 변경
6. Treemap.svelte 하이라이트
7. Toolbar.svelte 토글 버튼
```

### 난이도 평가

| 항목 | 난이도 |
|------|--------|
| 확장자 집계 로직 | 낮음 |
| 동적 색상 할당 | 중간 |
| ExtensionList UI | 중간 |
| 트리맵 하이라이트 | 중간 |
| 3-pane 레이아웃 | 낮음 |

### 주의사항

1. **성능**: 트리 순회 집계는 `tree` 또는 `zoomRoot` 변경 시에만 실행. `$derived`로 캐싱
2. **색상 일관성**: 동적 색상맵이 변경되면 트리맵 전체를 다시 그려야 함
3. **prune된 노드**: scanner.rs의 `prune_tree`가 200개 초과 파일을 합친 노드 -> `"(others)"` 카테고리로 처리
