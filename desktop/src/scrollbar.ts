// Keep the two visible insets equal without reserving an extra scrollbar width
// on both sides. Overlay scrollbars need no additional space.
export function balancedScrollInsets(node: HTMLElement) {
  const measure = () => {
    const scrollbar = Math.max(0, node.offsetWidth - node.clientWidth);
    node.style.setProperty('--scrollbar-width', `${scrollbar}px`);
  };
  measure();
  const observer = new ResizeObserver(measure);
  observer.observe(node);
  return { destroy: () => observer.disconnect() };
}

// Keep native wheel, keyboard and thumb dragging on both desktop platforms.
// Visibility follows activity, including with a mouse in a macOS VM.
export function observeScrollActivity() {
  const timers = new Map<HTMLElement, ReturnType<typeof setTimeout>>();
  const scroll = (event: Event) => {
    const node = event.target === document ? document.scrollingElement : event.target;
    if (!(node instanceof HTMLElement)) return;
    clearTimeout(timers.get(node));
    node.dataset.scrolling = 'true';
    timers.set(
      node,
      setTimeout(() => {
        delete node.dataset.scrolling;
        timers.delete(node);
      }, 800),
    );
  };
  document.addEventListener('scroll', scroll, { capture: true, passive: true });
  return () => {
    document.removeEventListener('scroll', scroll, true);
    for (const [node, timer] of timers) {
      clearTimeout(timer);
      delete node.dataset.scrolling;
    }
    timers.clear();
  };
}
