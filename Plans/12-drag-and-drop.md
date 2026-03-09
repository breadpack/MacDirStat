# 12. 드래그 앤 드롭 스캔 (Drag & Drop Scan)

## 1. 개요

사용자가 Finder에서 폴더를 MacDirStat 윈도우에 드래그 앤 드롭하면 해당 폴더의 스캔을 자동으로 시작한다.

## 2. 현재 상태

| 파일 | 상태 |
|------|------|
| `src/App.svelte` | 드래그 앤 드롭 미구현 |
| `src/lib/components/Toolbar.svelte` | `@tauri-apps/plugin-dialog`의 `open()`으로 폴더 선택 |
| `src/lib/stores/scanStore.ts` | `startScan(path)` 함수로 경로 기반 스캔 시작 |
| `src-tauri/capabilities/default.json` | drag-drop 관련 설정 없음 |

## 3. 구현 단계

### 단계 1: 접근 방식

**Tauri의 내장 drag-drop 이벤트 사용**

Tauri v2는 `Window.onDragDropEvent()`를 내장 지원한다. HTML5 API는 webview에서 파일의 실제 경로를 얻기 어려우므로 Tauri 방식이 적합하다. 별도 플러그인 설치 불필요.

### 단계 2: 이벤트 리스너 설정

App.svelte에 추가:

```typescript
import { getCurrentWindow } from "@tauri-apps/api/window";

let dragOver = $state(false);

$effect(() => {
  const appWindow = getCurrentWindow();
  const unlisten = appWindow.onDragDropEvent((event) => {
    if (event.payload.type === 'over') {
      dragOver = true;
    } else if (event.payload.type === 'drop') {
      dragOver = false;
      handleDrop(event.payload.paths);
    } else if (event.payload.type === 'leave') {
      dragOver = false;
    }
  });

  return () => { unlisten.then(fn => fn()); };
});
```

### 단계 3: 드롭 처리 로직

```typescript
async function handleDrop(paths: string[]) {
  if ($scanning) return;
  const targetPath = paths[0]; // 첫 번째 경로만 사용
  zoomRoot.set(null);
  await startScan(targetPath);
}
```

### 단계 4: 드래그 오버레이 UI

새 컴포넌트 `DropOverlay.svelte`:
- 전체 화면 반투명 오버레이 (position: fixed, z-index: 300)
- 중앙에 아이콘과 "Drop folder to scan" 텍스트
- dashed border
- 배경 `rgba(74, 144, 217, 0.15)`

### 단계 5: 에지 케이스 처리

1. **스캔 중 드롭**: 무시 또는 확인 대화상자 표시
2. **파일 드롭**: 부모 디렉토리를 스캔 대상으로 사용
3. **여러 항목 드롭**: 첫 번째 디렉토리만 사용
4. **빈 상태와 통합**: empty-state에 드래그 앤 드롭 안내 문구 추가

## 4. 수정 대상 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/App.svelte` | Tauri drag-drop 이벤트 리스너, dragOver 상태, 드롭 오버레이 |
| `src/lib/components/DropOverlay.svelte` | **신규** - 드래그 시 시각적 오버레이 |

## 5. 기술적 세부사항

### Tauri v2 Drag & Drop API

- `@tauri-apps/api/window`에서 import
- 별도 플러그인 설치 불필요
- capabilities에 별도 권한 추가 불필요 (core:default에 포함)

### macOS 고려사항
- Finder에서 드래그 시 파일 경로가 POSIX 형식으로 전달됨
- Full Disk Access 없이는 일부 경로에 접근 불가 - 기존 FDA 체크 로직이 처리

## 6. 예상 난이도 및 의존성

- **난이도**: 낮음
- **예상 시간**: 1~2시간
- **의존성**: 없음 (독립 구현 가능)
