import type { Action } from "svelte/action";

export const backdropClose: Action<HTMLElement, () => void> = (
  node,
  callback
) => {
  let activePointerId: number | null = null;

  function handlePointerDown(event: PointerEvent) {
    if (event.target === node) {
      activePointerId = event.pointerId;
    }
  }

  function handlePointerUp(event: PointerEvent) {
    if (event.pointerId === activePointerId && event.target === node) {
      callback();
    }
    if (event.pointerId === activePointerId) {
      activePointerId = null;
    }
  }

  function handlePointerCancel(event: PointerEvent) {
    if (event.pointerId === activePointerId) {
      activePointerId = null;
    }
  }

  node.addEventListener("pointerdown", handlePointerDown);
  node.addEventListener("pointerup", handlePointerUp);
  node.addEventListener("pointercancel", handlePointerCancel);

  return {
    update(newCallback: () => void) {
      callback = newCallback;
    },
    destroy() {
      node.removeEventListener("pointerdown", handlePointerDown);
      node.removeEventListener("pointerup", handlePointerUp);
      node.removeEventListener("pointercancel", handlePointerCancel);
    }
  };
};
