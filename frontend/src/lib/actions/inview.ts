export function inview(node: HTMLElement) {
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) {
        node.classList.add('in-view');
        observer.unobserve(node);
      }
    },
    { threshold: 0.15 }
  );
  observer.observe(node);
  return { destroy: () => observer.disconnect() };
}
