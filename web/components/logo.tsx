export function Star({ className = "" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M12 1.5l2.9 6.6 7.1.7-5.3 4.8 1.5 7-6.2-3.7-6.2 3.7 1.5-7L1.5 8.8l7.1-.7z" />
    </svg>
  );
}

export function Wordmark({ className = "" }: { className?: string }) {
  return (
    <span className={`inline-flex items-center gap-2 ${className}`}>
      <Star className="h-[1.05em] w-[1.05em] text-red" />
      <span className="font-medium tracking-tight">Redstart</span>
    </span>
  );
}
