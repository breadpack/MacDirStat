# 03. Free Space / Unknown Space 표시

## 1. 개요

WinDirStat은 F6 키로 디스크 여유 공간, F7 키로 "알 수 없는 공간"(디스크 전체 용량 - 스캔된 합계 - 여유 공간)을 트리맵에 시각적으로 표시한다. 이를 통해 사용자는 디스크 사용 현황을 전체적으로 파악할 수 있다.

MacDirStat은 이미 `VolumeInfo` (total_bytes, available_bytes)를 백엔드에서 제공하고 있으나, 이를 트리맵에 표시하는 기능이 없다.

## 2. 현재 상태

### 관련 파일

| 파일 | 역할 | 현재 상태 |
|------|------|-----------|
| `src-tauri/src/platform/macos.rs` | 디스크 정보 조회 | `get_volumes()` 이미 구현됨. `sysinfo::Disks` 사용 |
| `src-tauri/src/models.rs` | VolumeInfo 구조체 | `total_bytes`, `available_bytes` 필드 존재 |
| `src-tauri/src/commands.rs` | Tauri 커맨드 | `get_system_volumes()` 커맨드 이미 존재 |
| `src/lib/api.ts` | API 호출 | `getSystemVolumes()` 이미 구현 |
| `src/lib/types.ts` | 프론트엔드 타입 | `VolumeInfo` 인터페이스 존재 |
| `src/lib/utils/treemapLayout.ts` | 트리맵 레이아웃 | FileNode 기반, 특수 노드 미지원 |
| `src/lib/components/Treemap.svelte` | 렌더링 | FileNode만 렌더링 |

이미 디스크 정보를 조회하는 인프라가 완비되어 있으므로, 프론트엔드에서 트리맵에 통합하는 작업만 필요하다.

## 3. 구현 단계

### Step 1: 스캔 경로의 볼륨 정보 연동

스캔 시작 시 해당 경로가 속한 볼륨의 정보를 자동으로 조회하여 store에 저장한다.

`src/lib/stores/scanStore.ts`에 추가:

```typescript
export const currentVolume = writable<VolumeInfo | null>(null);
```

`startScan()` 함수에서:
```typescript
const volumes = await getSystemVolumes();
// 스캔 경로가 속한 볼륨 찾기 (가장 긴 mount_point 매칭)
const vol = volumes
  .filter(v => path.startsWith(v.mount_point))
  .sort((a, b) => b.mount_point.length - a.mount_point.length)[0];
currentVolume.set(vol ?? null);
```

### Step 2: Free Space / Unknown 토글 상태 관리

```typescript
export const showFreeSpace = writable<boolean>(false);
export const showUnknown = writable<boolean>(false);
```

### Step 3: 가상 FileNode 생성

트리맵 계산 직전, 표시 옵션에 따라 루트 노드의 children에 가상 노드를 추가한다.

```typescript
function injectSpecialNodes(
  root: FileNode,
  volume: VolumeInfo | null,
  showFree: boolean,
  showUnk: boolean,
): FileNode {
  if (!volume) return root;

  const scannedSize = root.size;
  const totalSize = volume.total_bytes;
  const freeSize = volume.available_bytes;
  const unknownSize = Math.max(0, totalSize - scannedSize - freeSize);

  const extraChildren: FileNode[] = [];

  if (showFree && freeSize > 0) {
    extraChildren.push({
      name: "<Free Space>",
      path: "/__freespace__",
      size: freeSize,
      is_dir: false,
      children: [],
      file_count: 0,
      extension: "__freespace__",
    });
  }

  if (showUnk && unknownSize > 0) {
    extraChildren.push({
      name: "<Unknown>",
      path: "/__unknown__",
      size: unknownSize,
      is_dir: false,
      children: [],
      file_count: 0,
      extension: "__unknown__",
    });
  }

  if (extraChildren.length === 0) return root;

  return {
    ...root,
    size: scannedSize + extraChildren.reduce((s, c) => s + c.size, 0),
    children: [...root.children, ...extraChildren],
  };
}
```

### Step 4: 특수 색상 매핑

`colorMap.ts`에 특수 확장자에 대한 색상을 추가한다.

```typescript
const SPECIAL_COLORS: Record<string, string> = {
  "__freespace__": "#2a2a2a",   // 어두운 회색 (빈 공간)
  "__unknown__":   "#3a2a1a",   // 어두운 갈색 (알 수 없는 공간)
};
```

### Step 5: UI 토글 추가

- F6: Free Space 토글
- F7: Unknown Space 토글
- Toolbar에 토글 버튼도 추가

### Step 6: zoomRoot 시 처리

`zoomRoot`가 설정된 상태(하위 디렉토리를 보고 있을 때)에서는 Free Space / Unknown 표시가 의미가 없으므로, 루트 전체를 볼 때만 활성화한다.

## 4. 수정 대상 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/lib/stores/scanStore.ts` | `currentVolume` store 추가, `startScan()`에서 볼륨 정보 조회 |
| `src/lib/stores/selectionStore.ts` | `showFreeSpace`, `showUnknown` store 추가 |
| `src/lib/components/Treemap.svelte` | 가상 노드 주입 로직, 툴팁 특수 처리 |
| `src/lib/utils/colorMap.ts` | 특수 확장자 색상 추가 |
| `src/lib/components/Toolbar.svelte` | Free Space / Unknown 토글 버튼 추가 |
| `src/App.svelte` | F6/F7 키보드 단축키 핸들러 추가 |

## 5. 기술적 세부사항

### Unknown Space 계산

```
unknown = total_bytes - available_bytes - scanned_size
```

`scanned_size`는 `root.size` (트리 루트의 합산 크기). 이 값은 스캔에서 제외된 시스템 디렉토리의 크기를 포함하지 않으므로, unknown이 상당히 클 수 있다.

### 특수 노드의 경로 충돌 방지

가상 노드의 path(`/__freespace__`, `/__unknown__`)는 실제 파일 시스템 경로와 충돌할 수 없는 패턴을 사용한다. `hitTest()`에서 이 경로를 감지하여 "Open in Finder" 등의 컨텍스트 메뉴를 비활성화해야 한다.

## 6. 예상 난이도 및 의존성

- **난이도**: 낮음~중간
- **예상 작업량**: 1-2일
- **의존성**: 없음 (Plan 01, 02와 독립적으로 구현 가능)
- **위험 요소**:
  - macOS firmlink로 인한 볼륨 매칭 부정확 -> `/` 볼륨을 기본 폴백으로 사용
  - 스캔 중에 Free Space가 변할 수 있음 -> 스캔 시작 시점의 스냅샷 사용
  - 하위 디렉토리 스캔 시 Unknown 값의 의미론적 모호함 -> 루트 스캔 시에만 활성화 권장
