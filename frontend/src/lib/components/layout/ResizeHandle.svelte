<script lang="ts">
  let { ondrag }: {
    ondrag: (deltaX: number) => void;
  } = $props();

  let dragging = $state(false);
  let startX = 0;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    startX = e.clientX;
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  function onMouseMove(e: MouseEvent) {
    const delta = e.clientX - startX;
    startX = e.clientX;
    ondrag(delta);
  }

  function onMouseUp() {
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
    dragging = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resize-handle"
  class:dragging
  onmousedown={onMouseDown}
>
  <div class="handle-grip">
    <span class="grip-dot"></span>
    <span class="grip-dot"></span>
    <span class="grip-dot"></span>
  </div>
</div>

<style>
  .resize-handle {
    width: 8px;
    flex-shrink: 0;
    position: relative;
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    user-select: none;
    z-index: 2;
  }

  .resize-handle::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    background: var(--border);
    transition: background 150ms ease;
  }

  .resize-handle:hover::before,
  .resize-handle.dragging::before {
    background: var(--accent);
  }

  .handle-grip {
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 1;
    opacity: 0;
    transition: opacity 150ms ease;
  }

  .resize-handle:hover .handle-grip,
  .resize-handle.dragging .handle-grip {
    opacity: 1;
  }

  .grip-dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .resize-handle:hover .grip-dot,
  .resize-handle.dragging .grip-dot {
    background: var(--accent);
  }
</style>
