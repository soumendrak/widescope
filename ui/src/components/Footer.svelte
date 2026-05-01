<script lang="ts">
</script>

<footer class="footer">
  <div class="footer-links">
    <a href="https://github.com/soumendrak/widescope" target="_blank" rel="noopener noreferrer" class="footer-link">
      <svg class="footer-icon" viewBox="0 0 24 24" fill="currentColor" width="14" height="14" aria-hidden="true">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.3 3.438 9.8 8.205 11.387.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.73.083-.73 1.205.085 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.418-1.305.762-1.604-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 21.795 24 17.295 24 12c0-6.63-5.37-12-12-12z"/>
      </svg>
      Source
    </a>
    <span class="footer-sep">·</span>
    <span class="footer-license">MIT License</span>
    <span class="footer-sep">·</span>
    <a href="https://www.soumendrak.com" target="_blank" rel="noopener" class="footer-link">Developed by Soumendra Kumar Sahoo</a>
  </div>
</footer>

<style>
  .footer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem 1rem;
    background: color-mix(in srgb, var(--color-bg, #0f172a) 92%, var(--color-accent, #3b82f6));
    box-shadow:
      0 -1px 0 rgba(0, 0, 0, 0.06),
      0 -2px 4px rgba(0, 0, 0, 0.04);
    z-index: 5;
  }

  :global([data-theme='light']) .footer {
    box-shadow:
      0 -1px 0 rgba(0, 0, 0, 0.04),
      0 -2px 6px rgba(0, 0, 0, 0.03);
  }

  .footer-links {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .footer-link {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    color: var(--color-text-muted, #94a3b8);
    text-decoration: none;
    padding: 0.35rem 0.55rem;
    margin: -0.35rem -0.1rem;
    border-radius: 6px;
    transition-property: color, background;
    transition-duration: 0.12s;
    transition-timing-function: ease;
  }

  .footer-link:hover {
    color: var(--color-text, #e2e8f0);
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
  }

  .footer-link:focus-visible {
    outline: 2px solid var(--focus-color, #3b82f6);
    outline-offset: 2px;
  }

  .footer-icon {
    flex-shrink: 0;
    opacity: 0.65;
  }

  .footer-link:hover .footer-icon {
    opacity: 1;
  }

  .footer-sep {
    color: var(--color-border, #334155);
    font-size: 0.65rem;
    user-select: none;
  }

  .footer-license {
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 520px) {
    .footer {
      padding: 0.4rem 0.75rem;
    }

    .footer-links {
      gap: 0.35rem;
      font-size: 0.7rem;
    }
  }
</style>
