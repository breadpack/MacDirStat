# 01. 쿠션 트리맵 셰이딩 (Cushion Treemap Shading)

## 1. 개요

WinDirStat의 핵심 시각적 차별점은 쿠션 셰이딩(cushion shading)이다. 현재 MacDirStat은 각 사각형을 단일 플랫 색상(`ctx.fillRect`)으로 렌더링하므로, 디렉토리 경계가 불분명하고 시각적 깊이감이 없다. 쿠션 셰이딩을 적용하면 각 사각형에 3D 쿠션 효과를 부여하여 디렉토리 중첩 구조를 직관적으로 파악할 수 있다.

쿠션 트리맵 알고리즘은 Jarke J. van Wijk와 Huub van de Wetering의 논문 "Cushion Treemaps: Visualization of Hierarchical Information" (2002)에 기반한다.

## 2. 현재 상태

### 관련 파일

| 파일 | 역할 | 현재 상태 |
|------|------|-----------|
| `src/lib/components/Treemap.svelte` | Canvas 렌더링 | 플랫 `fillRect`만 사용 (91-98행) |
| `src/lib/utils/treemapLayout.ts` | d3-hierarchy 기반 레이아웃 계산 | leaf 노드만 반환, 부모 계층 정보 없음 |
| `src/lib/utils/colorMap.ts` | 확장자별 색상 매핑 | hex 문자열 반환, RGB 분해 필요 |

### 현재 렌더링 로직 (Treemap.svelte 91-98행)

현재 각 rect에 대해 단순히 `ctx.fillStyle = getColor(...)` 후 `ctx.fillRect()`를 호출한다. 쿠션 셰이딩은 이 부분을 픽셀 단위 `ImageData` 렌더링으로 대체해야 한다.

### 현재 레이아웃 데이터 (treemapLayout.ts)

`TreemapRect`는 `{ x0, y0, x1, y1, data: FileNode }`만 포함한다. 쿠션 셰이딩에는 **루트에서 해당 leaf까지의 모든 조상 사각형 경계** 정보가 필요하다. 현재 d3-hierarchy의 `treeRoot.leaves()`만 수집하고 부모 정보를 버리고 있다.

## 3. 구현 단계

### Step 1: TreemapRect 확장 - 조상 경계 정보 추가

`treemapLayout.ts`에서 `TreemapRect`에 조상 사각형 경계 배열을 추가한다.

```typescript
export interface AncestorRect {
  x0: number; y0: number; x1: number; y1: number;
  isHorizontalSplit: boolean; // 해당 레벨에서 분할 방향
}

export interface TreemapRect {
  x0: number; y0: number; x1: number; y1: number;
  data: FileNode;
  depth: number;
  ancestors: AncestorRect[]; // root -> parent 순서
}
```

`computeTreemap()`에서 leaf 수집 시 `leaf.ancestors()` (d3-hierarchy 제공)를 활용하여 각 조상의 `x0, y0, x1, y1`을 기록한다.

### Step 2: 쿠션 셰이딩 유틸리티 모듈 생성

새 파일 `src/lib/utils/cushionShading.ts` 생성.

**쿠션 높이 함수:**

각 축(x, y)에 대해, 각 계층 레벨에서 parabolic surface를 누적한다:

```
f(x) = h * (4 / (x1 - x0)^2) * (x - x0) * (x1 - x)
```

여기서 `h`는 해당 레벨의 쿠션 높이. 깊은 레벨일수록 `h *= scaleFactor`로 감소한다.

**법선 벡터 계산:**

각 픽셀 (px, py)에서의 표면 법선은 x, y 각 축의 누적된 표면 함수의 편미분으로 구한다:

```
nx = -df/dx = -sum_i(h_i * (4/(x1_i - x0_i)^2) * (x1_i + x0_i - 2*px))
ny = -df/dy = -sum_i(h_i * (4/(y1_i - y0_i)^2) * (y1_i + y0_i - 2*py))
nz = 1
```

이후 법선을 정규화하고 광원 벡터와의 내적으로 intensity를 계산한다.

**Phong 반사 모델 (간소화):**

```
intensity = ambientLight + (1 - ambientLight) * max(0, dot(normal, lightDir))
```

최종 픽셀 색상:

```
R = baseR * intensity * brightness
G = baseG * intensity * brightness
B = baseB * intensity * brightness
```

**WinDirStat 기본 파라미터:**
- `brightness`: 0.84
- `cushionHeight`: 0.40
- `scaleFactor`: 0.90 (레벨당 높이 감쇄)
- `ambientLight`: 0.15
- `lightSource`: { x: -1, y: -1, z: 1 } (좌상단 광원, 정규화)

### Step 3: 쿠션 계수 사전 계산 (최적화 핵심)

5000+ rect에 대해 매 픽셀마다 조상을 순회하면 느리다. **각 rect에 대해 누적 쿠션 계수를 사전 계산**한다.

각 rect의 쿠션은 결국 x, y 각 축에 대한 1차 및 2차 항의 합으로 표현된다:

```
surface(px, py) = sum h_i * f_x(px) + sum h_i * f_y(py)
```

각 축의 편미분은 px, py에 대한 **1차 함수**이므로, 조상 전체를 순회하는 대신 두 개의 선형 계수로 축약 가능:

```typescript
interface CushionCoeffs {
  // df/dx = ax * px + bx
  ax: number; bx: number;
  // df/dy = ay * py + by
  ay: number; by: number;
}
```

이렇게 하면 각 픽셀에서 법선 계산이 O(1)이 된다.

### Step 4: ImageData 기반 렌더링 구현

`Treemap.svelte`의 렌더링 로직을 교체한다.

```typescript
const imageData = ctx.createImageData(width, height);
const pixels = imageData.data; // Uint8ClampedArray

for (const rect of rects) {
  const baseColor = getColorRGB(rect.data.extension, rect.data.is_dir);
  const coeffs = computeCushionCoeffs(rect);

  for (let py = Math.floor(rect.y0); py < Math.ceil(rect.y1); py++) {
    for (let px = Math.floor(rect.x0); px < Math.ceil(rect.x1); px++) {
      const nx = coeffs.ax * px + coeffs.bx;
      const ny = coeffs.ay * py + coeffs.by;
      const intensity = computeIntensity(nx, ny, lightDir, ambientLight);

      const idx = (py * width + px) * 4;
      pixels[idx]     = baseColor.r * intensity * brightness;
      pixels[idx + 1] = baseColor.g * intensity * brightness;
      pixels[idx + 2] = baseColor.b * intensity * brightness;
      pixels[idx + 3] = 255;
    }
  }
}

ctx.putImageData(imageData, 0, 0);
```

### Step 5: 그리드(테두리) 렌더링

쿠션 렌더링 후, `ctx.strokeRect`로 1px 경계선을 별도로 그린다 (선택 사항, 옵션화).

### Step 6: 성능 최적화

1. **작은 rect 스킵**: 면적 < 4px인 rect는 단색 fillRect로 대체
2. **OffscreenCanvas**: 가능한 경우 Web Worker에서 ImageData 계산
3. **캐싱**: rect 배열이 변경되지 않으면 ImageData를 재사용, 선택/호버 하이라이트만 overlay
4. **requestAnimationFrame**: 렌더링을 rAF에 맞추어 불필요한 재렌더 방지

### Step 7: colorMap.ts RGB 변환 지원

현재 hex 문자열만 반환하므로, RGB 값 반환 함수를 추가한다.

```typescript
export interface RGB { r: number; g: number; b: number; }

export function getColorRGB(extension: string | null, isDir: boolean): RGB {
  const hex = getColor(extension, isDir);
  return {
    r: parseInt(hex.slice(1, 3), 16),
    g: parseInt(hex.slice(3, 5), 16),
    b: parseInt(hex.slice(5, 7), 16),
  };
}
```

## 4. 수정 대상 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/lib/utils/treemapLayout.ts` | `TreemapRect`에 `depth`, `ancestors` 추가. `computeTreemap()`에서 조상 정보 수집 |
| `src/lib/utils/colorMap.ts` | `getColorRGB()` 함수 추가 |
| `src/lib/utils/cushionShading.ts` | **신규 생성**. 쿠션 계수 계산, intensity 계산, ImageData 렌더링 함수 |
| `src/lib/components/Treemap.svelte` | 렌더링 $effect를 쿠션 모드/플랫 모드 분기. ImageData 기반 렌더링 통합 |

## 5. 기술적 세부사항

### 쿠션 표면 함수

레벨 i에서 x축 분할 시:
```
f_i(x) = h * scaleFactor^depth * 4 * (x - x0_i)(x1_i - x) / (x1_i - x0_i)^2
```

편미분:
```
df_i/dx = h * scaleFactor^depth * 4 * (x1_i + x0_i - 2x) / (x1_i - x0_i)^2
```

이를 모든 조상에 대해 합산하면:
```
dF/dx = sum_i [h * s^d_i * 4 / (x1_i - x0_i)^2] * (x1_i + x0_i) - 2x * sum_i [h * s^d_i * 4 / (x1_i - x0_i)^2]
       = bx - 2x * ax
```

여기서:
- `ax = sum_i h * s^d_i * 4 / (x1_i - x0_i)^2`
- `bx = sum_i h * s^d_i * 4 * (x1_i + x0_i) / (x1_i - x0_i)^2`

y축도 동일.

### 광원 벡터

기본 광원 위치 (-1, -1, 1)을 정규화:
```
L = (-1/sqrt(3), -1/sqrt(3), 1/sqrt(3))
```

### 법선 벡터 및 Intensity

```
N = (-dF/dx, -dF/dy, 1)  ->  정규화
intensity = ambient + (1 - ambient) * max(0, dot(N_normalized, L))
```

## 6. 예상 난이도 및 의존성

- **난이도**: 높음
- **예상 작업량**: 3-5일
- **의존성**: 없음 (독립적으로 구현 가능)
- **위험 요소**:
  - ImageData 픽셀 루프 성능: 1920x1080 해상도에서 2M+ 픽셀 처리. 각 rect의 영역만 순회하므로 전체 픽셀 수와 동일. 30fps 유지를 위해 33ms 이내 완료 필요
  - 작은 rect(< 2px)에서의 쿠션 효과 무의미 -> 임계값 이하는 플랫 렌더링 필요
  - Retina 디스플레이에서 devicePixelRatio 고려 필요
