<div class="spinner">
  <div class="dot"></div>
  <div class="dot"></div>
  <div class="dot"></div>
</div>

<style>
  .spinner {
    display: flex;
    gap: 6px;
    justify-content: center;
    padding: 40px 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: bounce 1.4s ease-in-out infinite;
  }

  .dot:nth-child(2) { animation-delay: 0.16s; }
  .dot:nth-child(3) { animation-delay: 0.32s; }

  @keyframes bounce {
    0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
    40% { opacity: 1; transform: scale(1); }
  }
</style>
