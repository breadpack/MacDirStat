<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import ProgressBar from "./lib/components/ProgressBar.svelte";
  import TreeView from "./lib/components/TreeView.svelte";
  import Breadcrumb from "./lib/components/Breadcrumb.svelte";
  import Treemap from "./lib/components/Treemap.svelte";
  import TreemapOptionsPanel from "./lib/components/TreemapOptionsPanel.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import FullDiskAccessPrompt from "./lib/components/FullDiskAccessPrompt.svelte";
  import LogViewer from "./lib/components/LogViewer.svelte";
  import DropOverlay from "./lib/components/DropOverlay.svelte";
  import ExtensionList from "./lib/components/ExtensionList.svelte";
  import PanelResizer from "./lib/components/PanelResizer.svelte";
  import { tree, scanning, scanLogs, scanError, startScan, refreshSubtree } from "./lib/stores/scanStore";
  import { selectedPath, zoomRoot } from "./lib/stores/selectionStore";
  import { showExtensionPanel, extensionColorMap, highlightedExtension } from "./lib/stores/extensionStore";
  import { layoutStore } from "./lib/stores/layoutStore";
  import { setActiveColorMap } from "./lib/utils/colorMap";
  import { checkFullDiskAccess } from "./lib/api";
  import { matchShortcut, isTextInputFocused, SHORTCUTS } from "./lib/utils/shortcuts";
  import {
    actionOpenFolder,
    actionRefresh,
    actionCopyPath,
    actionMoveToTrash,
    actionShowInfo,
    actionToggleFreeSpace,
    actionToggleUnknown,
  } from "./lib/utils/actions";
  import CleanupSettings from "./lib/components/CleanupSettings.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import { cleanupActions, loadCleanupActions, runCleanup } from "./lib/stores/cleanupStore";
  import { settings } from "./lib/stores/settingsStore";
  import { findNode } from "./lib/utils/treeUtils";
  import { confirm, message } from "@tauri-apps/plugin-dialog";

  let fdaChecked = $state(false);
  let fdaGranted = $state(false);
  let logViewerOpen = $state(false);
  let optionsPanelOpen = $state(false);
  let cleanupSettingsOpen = $state(false);
  let settingsOpen = $state(false);
  let dragOver = $state(false);

  let treeViewRef: TreeView | undefined = $state();

  // Sync layoutStore.showExtensions <-> extensionStore.showExtensionPanel
  $effect(() => {
    const unsub = layoutStore.subscribe((l) => {
      showExtensionPanel.set(l.showExtensions);
    });
    return unsub;
  });
  $effect(() => {
    const unsub = showExtensionPanel.subscribe((v) => {
      layoutStore.update((l) => {
        if (l.showExtensions !== v) return { ...l, showExtensions: v };
        return l;
      });
    });
    return unsub;
  });

  // Load settings and cleanup actions on startup
  $effect(() => {
    settings.init();
    loadCleanupActions();
  });

  // Apply theme from settings
  $effect(() => {
    const theme = $settings.appearance.theme;
    let effectiveTheme = theme;
    if (theme === "system") {
      effectiveTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    document.documentElement.setAttribute("data-theme", effectiveTheme);
  });

  // Listen for system theme changes when theme is "system"
  $effect(() => {
    if ($settings.appearance.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      document.documentElement.setAttribute("data-theme", e.matches ? "dark" : "light");
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  });

  let errorCount = $derived($scanLogs.filter((l) => l.level === "error" || l.level === "warn").length);

  // Activate/deactivate dynamic color map based on extension panel visibility.
  $effect(() => {
    if ($showExtensionPanel) {
      // Subscribe to extensionColorMap to trigger dynamic color assignment
      const unsub = extensionColorMap.subscribe(() => {});
      return () => {
        unsub();
        setActiveColorMap(null);
        highlightedExtension.set(null);
      };
    } else {
      setActiveColorMap(null);
      highlightedExtension.set(null);
    }
  });

  $effect(() => {
    checkFullDiskAccess()
      .then((granted) => {
        fdaGranted = granted;
        fdaChecked = true;
      })
      .catch((e) => {
        console.warn("Failed to check Full Disk Access, assuming granted:", e);
        fdaGranted = true;
        fdaChecked = true;
      });
  });

  // Drag & drop event listener
  $effect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        dragOver = true;
      } else if (event.payload.type === "drop") {
        dragOver = false;
        handleDrop(event.payload.paths);
      } else if (event.payload.type === "leave") {
        dragOver = false;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  });

  async function handleDrop(paths: string[]) {
    if ($scanning) return;
    if (!paths || paths.length === 0) return;
    const targetPath = paths[0];
    zoomRoot.set(null);
    await startScan(targetPath);
  }

  function handleFdaGranted() {
    fdaGranted = true;
  }

  async function handleCleanupShortcut(slotId: number) {
    let actions: import("./lib/types").CleanupAction[] = [];
    cleanupActions.subscribe((a) => (actions = a))();
    const action = actions.find((a) => a.id === slotId);
    if (!action || !action.enabled || !action.name.trim() || !action.command.trim()) return;

    let currentPath: string | null = null;
    selectedPath.subscribe((p) => (currentPath = p))();
    if (!currentPath) return;

    let currentTree: import("./lib/types").FileNode | null = null;
    tree.subscribe((t) => (currentTree = t))();
    if (!currentTree) return;

    const node = findNode(currentTree, currentPath);
    if (!node) return;

    // Check target compatibility
    if (action.target === "Files" && node.is_dir) return;
    if (action.target === "Dirs" && !node.is_dir) return;

    if (action.confirm) {
      const ok = await confirm(`Run "${action.name}" on "${node.name}"?`, {
        title: "Run Cleanup",
        kind: "info",
        okLabel: "Run",
        cancelLabel: "Cancel",
      });
      if (!ok) return;
    }

    try {
      const result = await runCleanup(action.id, node.path, node.name);
      if (!action.run_in_terminal && result) {
        console.log(`Cleanup "${action.name}" result:`, result);
      }
      if (action.refresh_after && node.is_dir) {
        await refreshSubtree(node.path);
      }
    } catch (e) {
      console.error(`Cleanup "${action.name}" failed:`, e);
      await message(`Cleanup failed: ${e}`, { kind: "error" });
    }
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Cmd+, for settings
    if ((e.metaKey || e.ctrlKey) && e.key === ",") {
      e.preventDefault();
      settingsOpen = !settingsOpen;
      return;
    }

    // Cmd+Shift+0~9 for cleanup actions
    if (e.metaKey && e.shiftKey && !e.ctrlKey && !e.altKey) {
      const digit = e.key.match(/^[0-9]$/);
      if (digit) {
        e.preventDefault();
        handleCleanupShortcut(parseInt(digit[0]));
        return;
      }
    }

    const action = matchShortcut(e, SHORTCUTS);
    if (!action) return;

    // Actions that should always work (even in text inputs)
    switch (action) {
      case "open-folder":
        e.preventDefault();
        actionOpenFolder();
        return;
      case "refresh":
        e.preventDefault();
        actionRefresh();
        return;
      case "show-info":
        e.preventDefault();
        actionShowInfo();
        return;
      case "toggle-free-space":
        e.preventDefault();
        actionToggleFreeSpace();
        return;
      case "toggle-unknown":
        e.preventDefault();
        actionToggleUnknown();
        return;
      case "toggle-extensions":
        e.preventDefault();
        layoutStore.toggleExtensions();
        return;
      case "toggle-treemap":
        e.preventDefault();
        layoutStore.toggleTreemap();
        return;
      case "escape":
        // Escape: close context menu is handled by ContextMenu itself.
        // Close dialogs or panels in priority order.
        if (settingsOpen) {
          settingsOpen = false;
          e.preventDefault();
          return;
        }
        if (optionsPanelOpen) {
          optionsPanelOpen = false;
          e.preventDefault();
        } else if (logViewerOpen) {
          logViewerOpen = false;
          e.preventDefault();
        }
        return;
    }

    // Actions that should NOT fire when a text input is focused
    if (isTextInputFocused()) return;

    switch (action) {
      case "copy-path":
        e.preventDefault();
        actionCopyPath();
        return;
      case "move-to-trash":
        e.preventDefault();
        actionMoveToTrash();
        return;
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if fdaChecked && !fdaGranted}
  <FullDiskAccessPrompt onGranted={handleFdaGranted} />
{/if}

{#if dragOver}
  <DropOverlay />
{/if}

<div class="app">
  <Toolbar
    bind:optionsPanelOpen
    onSelectParent={() => treeViewRef?.selectParent()}
    onReselectChild={() => treeViewRef?.reselectChild()}
    onOpenCleanupSettings={() => cleanupSettingsOpen = true}
    onOpenSettings={() => settingsOpen = true}
  />
  <ProgressBar />
  <div class="main-content">
    {#if $layoutStore.showTree}
      <div
        class="tree-panel"
        class:full-width={!$layoutStore.showTreemap && !$layoutStore.showExtensions}
        style={$layoutStore.showTreemap || $layoutStore.showExtensions ? `width: ${$layoutStore.treePanelWidth}px` : ''}
      >
        {#if $tree}
          <TreeView bind:this={treeViewRef} />
        {:else if $scanError && !$scanning}
          <div class="empty-state">
            <div class="error-banner">
              <p class="error-message">Scan failed: {$scanError}</p>
              <button class="error-dismiss" onclick={() => scanError.set(null)}>Dismiss</button>
            </div>
          </div>
        {:else}
          <div class="empty-state">
            <p>Open or drop a folder to analyze disk usage</p>
          </div>
        {/if}
      </div>
      {#if $layoutStore.showExtensions || $layoutStore.showTreemap}
        <PanelResizer onResize={(d) => layoutStore.setTreeWidth($layoutStore.treePanelWidth + d)} />
      {/if}
    {/if}
    {#if $layoutStore.showExtensions && $tree}
      <div class="extension-panel" style="width: {$layoutStore.extensionPanelWidth}px">
        <ExtensionList />
      </div>
      {#if $layoutStore.showTreemap}
        <PanelResizer onResize={(d) => layoutStore.setExtensionWidth($layoutStore.extensionPanelWidth + d)} />
      {/if}
    {/if}
    {#if $layoutStore.showTreemap}
      <div class="treemap-panel">
        {#if $tree}
          <Breadcrumb />
          <div class="treemap-area">
            <Treemap />
            {#if optionsPanelOpen}
              <TreemapOptionsPanel onclose={() => optionsPanelOpen = false} />
            {/if}
          </div>
        {:else}
          <div class="empty-state">
            <p>Treemap visualization will appear here</p>
          </div>
        {/if}
      </div>
    {/if}
  </div>
  {#if logViewerOpen}
    <div class="log-panel">
      <LogViewer onClose={() => logViewerOpen = false} />
    </div>
  {/if}
  <StatusBar>
    {#snippet logButton()}
      <button
        class="log-toggle"
        class:has-errors={errorCount > 0}
        onclick={() => logViewerOpen = !logViewerOpen}
      >
        {logViewerOpen ? "\u25BC" : "\u25B2"} Log
        {#if errorCount > 0}
          <span class="error-badge">{errorCount}</span>
        {/if}
      </button>
    {/snippet}
  </StatusBar>
</div>

{#if cleanupSettingsOpen}
  <CleanupSettings onClose={() => cleanupSettingsOpen = false} />
{/if}

{#if settingsOpen}
  <SettingsDialog onClose={() => settingsOpen = false} />
{/if}

<style>
  :global(:root),
  :global([data-theme="dark"]) {
    --bg-primary: #1a1a1a;
    --bg-secondary: #1e1e1e;
    --bg-tertiary: #2d2d2d;
    --text-primary: #ccc;
    --text-secondary: #888;
    --border-color: #333;
    --border-color-strong: #555;
    --accent-color: #4A90D9;
  }

  :global([data-theme="light"]) {
    --bg-primary: #f5f5f5;
    --bg-secondary: #ffffff;
    --bg-tertiary: #e8e8e8;
    --text-primary: #333;
    --text-secondary: #666;
    --border-color: #ddd;
    --border-color-strong: #bbb;
    --accent-color: #2870BD;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: var(--bg-primary, #1a1a1a);
    color: var(--text-primary, #ccc);
    overflow: hidden;
    height: 100vh;
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(#app) {
    height: 100vh;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .main-content {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .tree-panel {
    flex-shrink: 0;
    min-width: 200px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tree-panel.full-width {
    flex: 1;
  }

  .extension-panel {
    flex-shrink: 0;
    min-width: 150px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .treemap-panel {
    flex: 1;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .treemap-area {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    font-size: 14px;
  }

  .error-banner {
    background: rgba(200, 50, 50, 0.15);
    border: 1px solid rgba(200, 50, 50, 0.4);
    border-radius: 6px;
    padding: 16px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    max-width: 500px;
  }

  .error-message {
    color: #e55;
    margin: 0;
    word-break: break-word;
    text-align: center;
  }

  .error-dismiss {
    background: rgba(200, 50, 50, 0.2);
    border: 1px solid rgba(200, 50, 50, 0.4);
    color: #ccc;
    border-radius: 4px;
    padding: 4px 16px;
    cursor: pointer;
    font-size: 12px;
  }

  .error-dismiss:hover {
    background: rgba(200, 50, 50, 0.35);
  }

  .log-panel {
    height: 200px;
    flex-shrink: 0;
    border-top: 1px solid #333;
  }

  .log-toggle {
    background: none;
    border: none;
    color: #888;
    font-size: 12px;
    cursor: pointer;
    padding: 2px 8px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .log-toggle:hover {
    color: #ccc;
  }

  .log-toggle.has-errors {
    color: #da4;
  }

  .error-badge {
    background: #c44;
    color: #fff;
    font-size: 10px;
    border-radius: 8px;
    padding: 0 5px;
    min-width: 16px;
    text-align: center;
  }
</style>
