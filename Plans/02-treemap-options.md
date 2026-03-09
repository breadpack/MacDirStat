# 02. 트리맵 스타일 옵션 (Treemap Style Options)

## 1. 개요

WinDirStat과 KDirStat은 트리맵의 시각적 스타일을 사용자가 세밀하게 조절할 수 있다. MacDirStat에 트리맵 옵션 패널을 추가하여 쿠션 파라미터, 그리드 표시, 스타일 프리셋 등을 제어할 수 있게 한다.

이 기능은 Plan 01(쿠션 셰이딩) 구현 이후에 의미가 있다. 쿠션 없이도 그리드, 색상 모드 등 일부 옵션은 독립적으로 구현 가능하다.

## 2. 현재 상태

### 관련 파일

| 파일 | 역할 | 현재 상태 |
|------|------|-----------|
| `src/lib/components/Treemap.svelte` | 렌더링 | 하드코딩된 padding(1), 색상, 폰트 |
| `src/lib/utils/treemapLayout.ts` | 레이아웃 | `treemapSquarify`만 사용, padding 1 고정 |
| `src/lib/utils/colorMap.ts` | 색상 | 확장자별 고정 색상 |
| `src/lib/components/Toolbar.svelte` | 상단 바 | Open/Stop 버튼만 존재, 옵션 UI 없음 |

## 3. 구현 단계

### Step 1: 트리맵 설정 Store 생성

새 파일 `src/lib/stores/treemapOptionsStore.ts` 생성.

```typescript
import { writable } from "svelte/store";

export interface TreemapOptions {
  // 쿠션 셰이딩
  cushionEnabled: boolean;
  brightness: number;      // 0.0 ~ 1.0, 기본 0.84
  cushionHeight: number;   // 0.0 ~ 1.0, 기본 0.40
  scaleFactor: number;     // 0.0 ~ 1.0, 기본 0.90
  ambientLight: number;    // 0.0 ~ 1.0, 기본 0.15

  // 광원 위치 (정규화된 벡터)
  lightX: number;          // 기본 -1
  lightY: number;          // 기본 -1

  // 그리드
  gridEnabled: boolean;    // 기본 true
  gridColor: string;       // 기본 "#000000"
  gridWidth: number;       // 기본 1

  // 스타일 프리셋
  style: "kdirstat" | "sequoiaview";

  // 레이아웃
  padding: number;         // 기본 1
}

const DEFAULT_OPTIONS: TreemapOptions = {
  cushionEnabled: true,
  brightness: 0.84,
  cushionHeight: 0.40,
  scaleFactor: 0.90,
  ambientLight: 0.15,
  lightX: -1,
  lightY: -1,
  gridEnabled: true,
  gridColor: "#000000",
  gridWidth: 1,
  style: "sequoiaview",
  padding: 1,
};
```

### Step 2: 스타일 프리셋 정의

**KDirStat 스타일:**
- 쿠션 높이가 낮고 (0.25), 그리드가 두껍고 (2px), 밝기가 높음 (0.90)
- 디렉토리 경계가 뚜렷한 "평면적" 느낌

**SequoiaView 스타일:**
- 높은 쿠션 높이 (0.50), 얇은 그리드 (1px), 낮은 ambient light (0.10)
- "볼록한 타일" 느낌, 3D 효과가 강함

### Step 3: 옵션 패널 컴포넌트 생성

새 파일 `src/lib/components/TreemapOptionsPanel.svelte` 생성.

UI 구성:
```
+----------------------------------+
| Treemap Options             [x]  |
+----------------------------------+
| Style: [KDirStat] [SequoiaView]  |
+----------------------------------+
| [ ] Cushion Shading              |
| Brightness    --*------ 0.84     |
| Height        ----*---- 0.40     |
| Scale Factor  ------*-- 0.90     |
| Ambient Light --*------ 0.15     |
+----------------------------------+
| Light Position                   |
| [*  ] <- 2D 드래그 영역          |
+----------------------------------+
| [ ] Grid                         |
| Color [#] Width --*-- 1          |
+----------------------------------+
| [Reset to Defaults]              |
+----------------------------------+
```

### Step 4: 광원 위치 2D 컨트롤

작은 정사각형(80x80px) 내에서 드래그하여 광원의 X, Y 방향을 설정한다. 중앙이 (0, 0), 좌상단이 (-1, -1).

### Step 5: Treemap.svelte에서 옵션 구독

```typescript
import { treemapOptions } from "../stores/treemapOptionsStore";

// $effect 내에서 $treemapOptions를 참조하면 옵션 변경 시 자동 재렌더링
$effect(() => {
  const opts = $treemapOptions;
  if (opts.cushionEnabled) {
    renderCushion(ctx, rects, opts);
  } else {
    renderFlat(ctx, rects);
  }
});
```

### Step 6: 실시간 미리보기

슬라이더 조작 시 `input` 이벤트(drag 중)에서 store를 업데이트하면, Svelte의 반응성으로 즉시 트리맵이 재렌더링된다. 단, 쿠션 렌더링이 무거울 경우 debounce(50ms)를 적용한다.

## 4. 수정 대상 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/lib/stores/treemapOptionsStore.ts` | **신규 생성**. 옵션 상태 관리 + localStorage 연동 |
| `src/lib/components/TreemapOptionsPanel.svelte` | **신규 생성**. 옵션 패널 UI |
| `src/lib/components/Treemap.svelte` | 옵션 store 구독, 조건부 렌더링 분기 |
| `src/lib/components/Toolbar.svelte` | 옵션 패널 토글 버튼 추가 |
| `src/lib/utils/treemapLayout.ts` | padding 옵션 반영 |
| `src/lib/utils/cushionShading.ts` | 옵션 파라미터를 인자로 받도록 설계 (Plan 01에서 생성) |
| `src/App.svelte` | 옵션 패널 컴포넌트 배치 (Treemap 영역 내 오버레이) |

## 5. 기술적 세부사항

### 슬라이더 범위 및 단계

| 파라미터 | 최소 | 최대 | 단계 | 기본값 |
|----------|------|------|------|--------|
| brightness | 0.0 | 1.0 | 0.01 | 0.84 |
| cushionHeight | 0.0 | 1.0 | 0.01 | 0.40 |
| scaleFactor | 0.0 | 1.0 | 0.01 | 0.90 |
| ambientLight | 0.0 | 1.0 | 0.01 | 0.15 |
| gridWidth | 0 | 5 | 1 | 1 |
| padding | 0 | 5 | 1 | 1 |

### 성능 고려사항

- 슬라이더 드래그 중 매 `input` 이벤트마다 전체 쿠션을 재계산하면 느릴 수 있다
- 50ms debounce 또는 `requestAnimationFrame` 기반 throttle 적용
- 대안: 드래그 중에는 저해상도(1/2 스케일) 미리보기, `change` 이벤트에서 풀 해상도 렌더링

## 6. 예상 난이도 및 의존성

- **난이도**: 중간
- **예상 작업량**: 2-3일
- **의존성**: Plan 01 (쿠션 셰이딩) - 쿠션 파라미터 슬라이더가 의미를 가지려면 쿠션 렌더링이 먼저 구현되어야 함. 단, 그리드/패딩 옵션은 독립적으로 구현 가능.
