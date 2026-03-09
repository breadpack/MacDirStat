# 13. 패널 레이아웃 관리 (Panel Layout Management)

## 1. 개요

3-pane 레이아웃(TreeView | ExtensionList | TreemapView)을 구현하고, 패널 토글(표시/숨기기), 분할 바(리사이저) 드래그, 레이아웃 상태 저장/복원을 지원한다.

## 2. 현재 상태

| 파일 | 현재 상태 |
|------|-----------|
| `src/App.svelte` | 2-pane 고정 레이아웃 (tree-panel 35% | treemap-panel flex:1) |
| `src/lib/components/TreeView.svelte` | tree-panel에 배치, 크기 고정 |
| `src/lib/components/Treemap.svelte` | treemap-panel에 배치, ResizeObserver 사용 |

### 현재 레이아웃
```
+-------+------------------------+
| Tree  | Breadcrumb             |
| View  +------------------------+
| (35%) | Treemap (flex:1)       |
+-------+------------------------+
```

## 3. 구현 단계

### 단계 1: 패널 가시성 Store

새 파일 `src/lib/stores/layoutStore.ts`:

```typescript
export interface PanelLayout {
  showTree: boolean;
  showExtensions: boolean;
  showTreemap: boolean;
  treePanelWidth: number;      // px
  extensionPanelWidth: number;  // px
}

const DEFAULT_LAYOUT: PanelLayout = {
  showTree: true,
  showExtensions: false,
  showTreemap: true,
  treePanelWidth: 350,
  extensionPanelWidth: 250,
};
```

localStorage 연동으로 상태 영속 저장.

### 단계 2: 리사이저 컴포넌트

새 파일 `src/lib/components/PanelResizer.svelte`:

- 너비 4px, 높이 100%의 세로 바
- hover 시 배경색 변경 (#4A90D9)
- cursor: col-resize
- mousedown -> mousemove -> mouseup 패턴
- 드래그 중 document에 이벤트 캡처

### 단계 3: App.svelte 레이아웃 리팩토링

```svelte
<div class="main-content">
  {#if $layout.showTree}
    <div class="tree-panel" style="width: {$layout.treePanelWidth}px">
      <!-- TreeView -->
    </div>
    <PanelResizer onResize={(d) => layout.setTreeWidth($layout.treePanelWidth + d)} />
  {/if}

  {#if $layout.showExtensions}
    <div class="extension-panel" style="width: {$layout.extensionPanelWidth}px">
      <!-- ExtensionList -->
    </div>
    <PanelResizer onResize={(d) => layout.setExtensionWidth($layout.extensionPanelWidth + d)} />
  {/if}

  {#if $layout.showTreemap}
    <div class="treemap-panel">
      <!-- Treemap -->
    </div>
  {/if}
</div>
```

### 단계 4: 패널 토글 연동

Toolbar.svelte에 토글 버튼 추가 + 키보드 단축키:
- F8 -> `layout.toggleExtensions()`
- F9 -> `layout.toggleTreemap()`

### 단계 5: 최소/최대 크기 제한

- tree-panel: min 200px, max 60% of window
- extension-panel: min 150px, max 400px
- treemap-panel: 항상 최소 200px 확보
- 리사이저 onResize에서 clamp 적용

### 단계 6: 레이아웃 상태 저장/복원

- `$effect`로 변경 감지 후 localStorage에 자동 저장 (500ms debounce)
- 앱 시작 시 복원

## 4. 목표 레이아웃

```
+--------------------------------------------------+
| Toolbar  [Open] [Stop] | [Tree] [Ext] [Treemap]  |
+--------------------------------------------------+
| ProgressBar                                      |
+----------+--+----------+--+---------------------+
|          |##|          |##| Breadcrumb           |
| TreeView |##| Extension|##|---------------------+
|          |##| List     |##| Treemap              |
|          |##|          |##|                      |
+----------+--+----------+--+---------------------+
| LogViewer (toggle)                               |
+--------------------------------------------------+
| StatusBar                                        |
+--------------------------------------------------+
 ## = PanelResizer (드래그 가능)
```

## 5. 수정 대상 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/lib/stores/layoutStore.ts` | **신규** - 패널 가시성/크기 상태 |
| `src/lib/components/PanelResizer.svelte` | **신규** - 드래그 가능한 분할 바 |
| `src/App.svelte` | 레이아웃 구조 변경, 리사이저 통합 |
| `src/lib/components/Toolbar.svelte` | 패널 토글 버튼 추가 |
| `src/lib/components/Treemap.svelte` | ResizeObserver가 이미 있어 변경 최소 |

## 6. 기술적 세부사항

### CSS Flexbox 기반 레이아웃

```css
.main-content { display: flex; flex: 1; min-height: 0; overflow: hidden; }
.tree-panel { flex-shrink: 0; min-width: 200px; overflow: hidden; }
.extension-panel { flex-shrink: 0; min-width: 150px; overflow: hidden; }
.treemap-panel { flex: 1; min-width: 200px; overflow: hidden; }
```

### Treemap 자동 리사이즈

Treemap.svelte에 이미 `ResizeObserver`가 구현되어 있어, 패널 크기 변경 시 자동으로 캔버스 크기를 재조정한다. 추가 수정 거의 불필요.

### 모든 패널 숨김 방지

최소 하나의 패널은 항상 표시되도록 강제.

## 7. 예상 난이도 및 의존성

- **난이도**: 높음
- **예상 시간**: 5~7시간
- **의존성**:
  - Plan 11 (키보드 단축키)의 F8/F9 토글과 연동
  - ExtensionList는 Plan 04에서 구현
- **주의사항**:
  - 리사이저 드래그 중 Treemap 캔버스 재렌더링 성능 주의 (rAF 또는 debounce)
  - CSS width 고정값(35%)을 px로 전환하면서 반응형 최소 크기 보장 필요
